use std::task::{Context, Poll};
use std::time::Duration;
use std::{future::Future, pin::Pin};

use derive_more::Display;
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::stream::{Stream, StreamExt};
use futures::{FutureExt, SinkExt};
use futures_timer::Delay;
use log::{debug, error, info};

use crate::smr::smr_types::{SMREvent, SMRTrigger, TriggerSource, TriggerType};
use crate::smr::{Event, SMRHandler};
use crate::DurationConfig;
use crate::{error::ConsensusError, ConsensusResult, INIT_EPOCH_ID, INIT_ROUND};
use crate::{types::Hash, utils::timer_config::TimerConfig};

/// Overlord timer used futures timer which is powered by a timer heap. When monitor a SMR event,
/// timer will get timeout interval from timer config, then set a delay. When the timeout expires,
#[derive(Debug)]
pub struct Timer {
    config:        TimerConfig,
    event:         Event,
    sender:        UnboundedSender<SMREvent>,
    notify:        UnboundedReceiver<SMREvent>,
    state_machine: SMRHandler,
    epoch_id:      u64,
    round:         u64,
}

///
impl Stream for Timer {
    type Item = ConsensusError;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut event_ready = true;
        let mut timer_ready = true;

        loop {
            match self.event.poll_next_unpin(cx) {
                Poll::Pending => event_ready = false,

                Poll::Ready(event) => {
                    if event.is_none() {
                        return Poll::Ready(Some(ConsensusError::TimerErr(
                            "Channel dropped".to_string(),
                        )));
                    }

                    let event = event.unwrap();
                    if event == SMREvent::Stop {
                        return Poll::Ready(None);
                    }
                    if let Err(e) = self.set_timer(event) {
                        return Poll::Ready(Some(e));
                    }
                }
            };

            match self.notify.poll_next_unpin(cx) {
                Poll::Pending => timer_ready = false,

                Poll::Ready(event) => {
                    if event.is_none() {
                        return Poll::Ready(Some(ConsensusError::TimerErr(
                            "Channel terminated".to_string(),
                        )));
                    }

                    let event = event.unwrap();
                    if let Err(e) = self.trigger(event) {
                        return Poll::Ready(Some(e));
                    }
                }
            }
            if !event_ready && !timer_ready {
                return Poll::Pending;
            }
        }
    }
}

impl Timer {
    pub fn new(
        event: Event,
        state_machine: SMRHandler,
        interval: u64,
        config: Option<DurationConfig>,
    ) -> Self {
        let (tx, rx) = unbounded();
        let mut timer_config = TimerConfig::new(interval);
        if let Some(tmp) = config {
            timer_config.update(tmp);
        }

        Timer {
            config: timer_config,
            epoch_id: INIT_EPOCH_ID,
            round: INIT_ROUND,
            sender: tx,
            notify: rx,
            event,
            state_machine,
        }
    }

    pub fn run(mut self) {
        runtime::spawn(async move {
            loop {
                if let Some(err) = self.next().await {
                    error!("Overlord: timer error {:?}", err);
                }
            }
        });
    }

    fn set_timer(&mut self, event: SMREvent) -> ConsensusResult<()> {
        let mut is_propose_timer = false;
        match event.clone() {
            SMREvent::NewRoundInfo {
                epoch_id, round, ..
            } => {
                if epoch_id > self.epoch_id {
                    self.epoch_id = epoch_id;
                }
                self.round = round;
                is_propose_timer = true;
            }
            SMREvent::Commit(_) => return Ok(()),
            _ => (),
        };

        let mut interval = self.config.get_timeout(event.clone())?;

        if is_propose_timer {
            let mut coef = self.round as u32;
            if coef > 10 {
                coef = 10;
            }
            interval *= 2u32.pow(coef);
        }

        info!("Overlord: timer set {:?} timer", event);

        let smr_timer = TimeoutInfo::new(interval, event, self.sender.clone());

        runtime::spawn(async move {
            smr_timer.await;
        });

        Ok(())
    }

    #[rustfmt::skip]
    fn trigger(&mut self, event: SMREvent) -> ConsensusResult<()> {
        let (trigger_type, round, epoch_id) = match event {
            SMREvent::NewRoundInfo { epoch_id, round, .. } => {
                if epoch_id < self.epoch_id || round < self.round {
                    return Ok(());
                }
                (TriggerType::Proposal, None, epoch_id)
            }

            SMREvent::PrevoteVote {
                epoch_id, round, ..
            } => {
                if epoch_id < self.epoch_id {
                    return Ok(());
                }
                (TriggerType::PrevoteQC, Some(round), epoch_id)
            }

            SMREvent::PrecommitVote {
                epoch_id, round, ..
            } => {
                if epoch_id < self.epoch_id {
                    return Ok(());
                }
                (TriggerType::PrecommitQC, Some(round), epoch_id)
            }

            _ => return Err(ConsensusError::TimerErr("No commit timer".to_string())),
        };

        debug!("Overlord: timer {:?} time out", event);

        self.state_machine.trigger(SMRTrigger {
            source: TriggerSource::Timer,
            hash: Hash::new(),
            trigger_type,
            round,
            epoch_id,
        })
    }
}

/// Timeout info which is a future consists of a `futures-timer Delay`, timeout info and a sender.
/// When the timeout expires, future will send timeout info by sender.
#[derive(Debug, Display)]
#[display(fmt = "{:?}", info)]
struct TimeoutInfo {
    timeout: Delay,
    info:    SMREvent,
    sender:  UnboundedSender<SMREvent>,
}

impl Future for TimeoutInfo {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let msg = self.info.clone();
        let mut tx = self.sender.clone();

        match self.timeout.poll_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(_) => {
                runtime::spawn(async move {
                    let _ = tx.send(msg).await;
                });
                Poll::Ready(())
            }
        }
    }
}

impl TimeoutInfo {
    fn new(interval: Duration, event: SMREvent, tx: UnboundedSender<SMREvent>) -> Self {
        // let mut delay = Delay::new_handle(Instant::now(), TimerHandle::default());
        // delay.reset(interval);

        let delay = Delay::new(interval);

        TimeoutInfo {
            timeout: delay,
            info:    event,
            sender:  tx,
        }
    }
}

#[cfg(test)]
mod test {
    use futures::channel::mpsc::unbounded;
    use futures::stream::StreamExt;

    use crate::smr::smr_types::{SMREvent, SMRTrigger, TriggerSource, TriggerType};
    use crate::smr::{Event, SMRHandler};
    use crate::{timer::Timer, types::Hash};

    async fn test_timer_trigger(input: SMREvent, output: SMRTrigger) {
        let (trigger_tx, mut trigger_rx) = unbounded();
        let (event_tx, event_rx) = unbounded();
        let mut timer = Timer::new(
            Event::new(event_rx),
            SMRHandler::new(trigger_tx),
            3000,
            None,
        );
        event_tx.unbounded_send(input).unwrap();

        runtime::spawn(async move {
            loop {
                match timer.next().await {
                    None => break,
                    Some(_) => panic!("Error"),
                }
            }
        });

        if let Some(res) = trigger_rx.next().await {
            assert_eq!(res, output);
            event_tx.unbounded_send(SMREvent::Stop).unwrap();
        }
    }

    fn gen_output(trigger_type: TriggerType, round: Option<u64>, epoch_id: u64) -> SMRTrigger {
        SMRTrigger {
            source: TriggerSource::Timer,
            hash: Hash::new(),
            trigger_type,
            round,
            epoch_id,
        }
    }

    #[runtime::test]
    async fn test_correctness() {
        // Test propose step timer.
        test_timer_trigger(
            SMREvent::NewRoundInfo {
                epoch_id:      0,
                round:         0,
                lock_round:    None,
                lock_proposal: None,
            },
            gen_output(TriggerType::Proposal, None, 0),
        )
        .await;

        // Test prevote step timer.
        test_timer_trigger(
            SMREvent::PrevoteVote {
                epoch_id:   0u64,
                round:      0u64,
                epoch_hash: Hash::new(),
            },
            gen_output(TriggerType::PrevoteQC, Some(0), 0),
        )
        .await;

        // Test precommit step timer.
        test_timer_trigger(
            SMREvent::PrecommitVote {
                epoch_id:   0u64,
                round:      0u64,
                epoch_hash: Hash::new(),
            },
            gen_output(TriggerType::PrecommitQC, Some(0), 0),
        )
        .await;
    }

    #[runtime::test]
    async fn test_order() {
        let (trigger_tx, mut trigger_rx) = unbounded();
        let (event_tx, event_rx) = unbounded();
        let mut timer = Timer::new(
            Event::new(event_rx),
            SMRHandler::new(trigger_tx),
            3000,
            None,
        );

        let new_round_event = SMREvent::NewRoundInfo {
            epoch_id:      0,
            round:         0,
            lock_round:    None,
            lock_proposal: None,
        };

        let prevote_event = SMREvent::PrevoteVote {
            epoch_id:   0u64,
            round:      0u64,
            epoch_hash: Hash::new(),
        };

        let precommit_event = SMREvent::PrecommitVote {
            epoch_id:   0u64,
            round:      0u64,
            epoch_hash: Hash::new(),
        };

        runtime::spawn(async move {
            loop {
                match timer.next().await {
                    None => break,
                    Some(_) => panic!("Error"),
                }
            }
        });

        event_tx.unbounded_send(new_round_event).unwrap();
        event_tx.unbounded_send(prevote_event).unwrap();
        event_tx.unbounded_send(precommit_event).unwrap();

        let mut count = 1u32;
        let mut output = Vec::new();
        let predict = vec![
            gen_output(TriggerType::PrecommitQC, Some(0), 0),
            gen_output(TriggerType::PrevoteQC, Some(0), 0),
            gen_output(TriggerType::Proposal, None, 0),
        ];

        while let Some(res) = trigger_rx.next().await {
            output.push(res);
            if count != 3 {
                count += 1;
            } else {
                assert_eq!(predict, output);
                event_tx.unbounded_send(SMREvent::Stop).unwrap();
                return;
            }
        }
    }
}
