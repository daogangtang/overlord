use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use creep::Context;
use crossbeam_channel::{unbounded, Receiver, Sender};
use hasher::{Hasher, HasherKeccak};
use lazy_static::lazy_static;
use rand::random;
use serde::{Deserialize, Serialize};

use overlord::types::{AggregatedSignature, Commit, Hash, Node, OverlordMsg, Status};
use overlord::{Codec, Consensus, Crypto, DurationConfig, Overlord, OverlordHandler};

lazy_static! {
    static ref HASHER_INST: HasherKeccak = HasherKeccak::new();
}

const SPEAKER_NUM: u8 = 20;

const SPEECH_INTERVAL: u64 = 1000; // ms

type Channel = (Sender<OverlordMsg<Speech>>, Receiver<OverlordMsg<Speech>>);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Speech {
    inner: Bytes,
}

impl Speech {
    fn from(thought: Bytes) -> Self {
        Speech { inner: thought }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Detail {
    inner: Bytes,
}

impl Detail {
    fn from(speech: Speech) -> Self {
        Detail {
            // explain your speech
            inner: speech.inner,
        }
    }
}

macro_rules! impl_codec_for {
    ($($struc: ident),+) => {
        $(
            impl Codec for $struc {
                fn encode(&self) -> Result<Bytes, Box<dyn Error + Send>> {
                    Ok(Bytes::from(bincode::serialize(&self.inner).unwrap()))
                }

                fn decode(data: Bytes) -> Result<Self, Box<dyn Error + Send>> {
                    let data: Option<Bytes> = bincode::deserialize(&data).unwrap();
                    Ok($struc { inner: data.unwrap() })
                }
            }
        )+
    }
}

impl_codec_for!(Speech, Detail);

struct MockCrypto {
    name: Bytes,
}

impl MockCrypto {
    fn new(name: Bytes) -> Self {
        MockCrypto { name }
    }
}

impl Crypto for MockCrypto {
    fn hash(&self, speech: Bytes) -> Bytes {
        hash(&speech)
    }

    fn sign(&self, _hash: Bytes) -> Result<Bytes, Box<dyn Error + Send>> {
        Ok(self.name.clone())
    }

    fn aggregate_signatures(
        &self,
        _signatures: Vec<Bytes>,
        _speaker: Vec<Bytes>,
    ) -> Result<Bytes, Box<dyn Error + Send>> {
        Ok(Bytes::new())
    }

    fn verify_signature(
        &self,
        signature: Bytes,
        _hash: Bytes,
    ) -> Result<Bytes, Box<dyn Error + Send>> {
        Ok(signature)
    }

    fn verify_aggregated_signature(
        &self,
        _aggregated_signature: AggregatedSignature,
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

struct Brain {
    speaker_list:     Vec<Node>,
    talk_to:          HashMap<Bytes, Sender<OverlordMsg<Speech>>>,
    hearing:          Receiver<OverlordMsg<Speech>>,
    consensus_speech: Arc<Mutex<HashMap<u64, Bytes>>>,
}

impl Brain {
    fn new(
        speaker_list: Vec<Node>,
        talk_to: HashMap<Bytes, Sender<OverlordMsg<Speech>>>,
        hearing: Receiver<OverlordMsg<Speech>>,
        consensus_speech: Arc<Mutex<HashMap<u64, Bytes>>>,
    ) -> Brain {
        Brain {
            speaker_list,
            talk_to,
            hearing,
            consensus_speech,
        }
    }
}

#[async_trait]
impl Consensus<Speech, Detail> for Brain {
    async fn get_epoch(
        &self,
        _ctx: Context,
        _epoch_id: u64,
    ) -> Result<(Speech, Hash), Box<dyn Error + Send>> {
        let thought = gen_random_bytes();
        Ok((Speech::from(thought.clone()), hash(&thought)))
    }

    async fn check_epoch(
        &self,
        _ctx: Context,
        _epoch_id: u64,
        _hash: Hash,
        speech: Speech,
    ) -> Result<Detail, Box<dyn Error + Send>> {
        Ok(Detail::from(speech))
    }

    async fn commit(
        &self,
        _ctx: Context,
        epoch_id: u64,
        commit: Commit<Speech>,
    ) -> Result<Status, Box<dyn Error + Send>> {
        let mut speeches = self.consensus_speech.lock().unwrap();
        if let Some(speech) = speeches.get(&commit.epoch_id) {
            assert_eq!(speech, &commit.content.inner);
        } else {
            println!(
                "In epoch_id: {:?}, commit with : {:?}",
                commit.epoch_id,
                hex::encode(commit.content.inner.clone())
            );
            speeches.insert(commit.epoch_id, commit.content.inner);
        }

        Ok(Status {
            epoch_id:       epoch_id + 1,
            interval:       Some(SPEECH_INTERVAL),
            authority_list: self.speaker_list.clone(),
        })
    }

    async fn get_authority_list(
        &self,
        _ctx: Context,
        _epoch_id: u64,
    ) -> Result<Vec<Node>, Box<dyn Error + Send>> {
        Ok(self.speaker_list.clone())
    }

    async fn broadcast_to_other(
        &self,
        _ctx: Context,
        words: OverlordMsg<Speech>,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.talk_to.iter().for_each(|(_, mouth)| {
            mouth.send(words.clone()).unwrap();
        });
        Ok(())
    }

    async fn transmit_to_relayer(
        &self,
        _ctx: Context,
        name: Bytes,
        words: OverlordMsg<Speech>,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.talk_to.get(&name).unwrap().send(words).unwrap();
        Ok(())
    }
}

struct Speaker {
    overlord: Arc<Overlord<Speech, Detail, Brain, MockCrypto>>,
    handler:  OverlordHandler<Speech>,
    brain:    Arc<Brain>,
}

impl Speaker {
    fn new(
        name: Bytes,
        speaker_list: Vec<Node>,
        talk_to: HashMap<Bytes, Sender<OverlordMsg<Speech>>>,
        hearing: Receiver<OverlordMsg<Speech>>,
        consensus_speech: Arc<Mutex<HashMap<u64, Bytes>>>,
    ) -> Self {
        let crypto = MockCrypto::new(name.clone());
        let brain = Arc::new(Brain::new(
            speaker_list.clone(),
            talk_to,
            hearing,
            consensus_speech,
        ));
        let overlord = Overlord::new(name, Arc::clone(&brain), crypto);
        let overlord_handler = overlord.get_handler();

        overlord_handler
            .send_msg(
                Context::new(),
                OverlordMsg::RichStatus(Status {
                    epoch_id:       1,
                    interval:       Some(SPEECH_INTERVAL),
                    authority_list: speaker_list,
                }),
            )
            .unwrap();

        Self {
            overlord: Arc::new(overlord),
            handler: overlord_handler,
            brain,
        }
    }

    async fn run(
        &self,
        interval: u64,
        timer_config: Option<DurationConfig>,
    ) -> Result<(), Box<dyn Error + Send>> {
        let brain = Arc::<Brain>::clone(&self.brain);
        let handler = self.handler.clone();

        thread::spawn(move || loop {
            if let Ok(msg) = brain.hearing.recv() {
                match msg {
                    OverlordMsg::SignedVote(vote) => {
                        handler
                            .send_msg(Context::new(), OverlordMsg::SignedVote(vote))
                            .unwrap();
                    }
                    OverlordMsg::SignedProposal(proposal) => {
                        handler
                            .send_msg(Context::new(), OverlordMsg::SignedProposal(proposal))
                            .unwrap();
                    }
                    OverlordMsg::AggregatedVote(agg_vote) => {
                        handler
                            .send_msg(Context::new(), OverlordMsg::AggregatedVote(agg_vote))
                            .unwrap();
                    }
                    _ => {}
                }
            }
        });

        self.overlord.run(interval, timer_config).await.unwrap();

        Ok(())
    }
}

#[runtime::main(runtime_tokio::Tokio)]
async fn main() {
    let speaker_list: Vec<Node> = (0..SPEAKER_NUM)
        .map(|_| Node::new(gen_random_bytes()))
        .collect();
    let channels: Vec<Channel> = (0..SPEAKER_NUM).map(|_| unbounded()).collect();
    let hearings: HashMap<Bytes, Receiver<OverlordMsg<Speech>>> = speaker_list
        .iter()
        .map(|node| node.address.clone())
        .zip(channels.iter().map(|(_, receiver)| receiver.clone()))
        .collect();
    let consensus_speech = Arc::new(Mutex::new(HashMap::new()));

    let speaker_list_clone = speaker_list.clone();

    for speaker in speaker_list {
        let name = speaker.address;
        let mut talk_to: HashMap<Bytes, Sender<OverlordMsg<Speech>>> = speaker_list_clone
            .iter()
            .map(|speaker| speaker.address.clone())
            .zip(channels.iter().map(|(sender, _)| sender.clone()))
            .collect();
        talk_to.remove(&name);

        let speaker = Arc::new(Speaker::new(
            name.clone(),
            speaker_list_clone.clone(),
            talk_to,
            hearings.get(&name).unwrap().clone(),
            Arc::<Mutex<HashMap<u64, Bytes>>>::clone(&consensus_speech),
        ));
        runtime::spawn(async move {
            speaker.run(SPEECH_INTERVAL, timer_config()).await.unwrap();
        });
    }

    thread::sleep(Duration::from_secs(100));
}

fn gen_random_bytes() -> Bytes {
    let vec: Vec<u8> = (0..10).map(|_| random::<u8>()).collect();
    Bytes::from(vec)
}

fn hash(bytes: &Bytes) -> Bytes {
    let mut out = [0u8; 32];
    out.copy_from_slice(&HASHER_INST.digest(bytes));
    Bytes::from(&out[..])
}

fn timer_config() -> Option<DurationConfig> {
    Some(DurationConfig::new(10, 10, 10))
}
