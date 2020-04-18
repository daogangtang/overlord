use std::fmt::Debug;

use bytes::Bytes;
use derive_more::Display;
use serde::{Deserialize, Serialize};

use crate::{Blk, St};

pub type Hash = Bytes;
pub type Address = Bytes;
pub type Signature = Bytes;

pub type PriKeyHex = String;
pub type PubKeyHex = String;
pub type CommonHex = String;

pub type Height = u64;
pub type Round = u64;

pub type Weight = u32;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Display, PartialEq, Eq)]
pub enum OverlordMsg<B: Blk> {
    #[display(fmt = "signed_proposal: {}", _0)]
    SignedProposal(SignedProposal<B>),
    #[display(fmt = "signed_Pre_vote: {}", _0)]
    SignedPreVote(SignedPreVote),
    #[display(fmt = "signed_Pre_vote: {}", _0)]
    SignedPreCommit(SignedPreCommit),
    #[display(fmt = "pre_vote_qc: {}", _0)]
    PreVoteQC(PreVoteQC),
    #[display(fmt = "pre_commit_qc: {}", _0)]
    PreCommitQC(PreCommitQC),
    #[display(fmt = "signed_choke: {}", _0)]
    SignedChoke(SignedChoke),
    #[display(fmt = "signed_height: {}", _0)]
    SignedHeight(SignedHeight),
    #[display(fmt = "sync_request: {}", _0)]
    SyncRequest(SyncRequest),
    #[display(fmt = "sync_response: {}", _0)]
    SyncResponse(SyncResponse<B>),
    #[display(fmt = "stop overlord")]
    Stop,
}

impl_from!(
    OverlordMsg<B: Blk>,
    [
        SignedProposal<B>,
        SignedPreVote,
        SignedPreCommit,
        SignedChoke,
        PreVoteQC,
        PreCommitQC,
        SignedHeight,
        SyncRequest,
        SyncResponse<B>,
    ]
);

#[derive(Clone, Debug, Display, Default, PartialEq, Eq)]
#[display(fmt = "{}", proposal)]
pub struct SignedProposal<B: Blk> {
    pub proposal:  Proposal<B>,
    pub signature: Signature,
}

impl<B: Blk> SignedProposal<B> {
    pub fn new(proposal: Proposal<B>, signature: Signature) -> Self {
        SignedProposal {
            proposal,
            signature,
        }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Serialize, Deserialize)]
#[display(
    fmt = "{{ height: {}, round: {}, block_hash: {}, lock: {}, proposer: {} }}",
    height,
    round,
    "block_hash.tiny_hex()",
    "lock.clone().map_or(\"None\".to_owned(), |lock| format!(\"{}\", lock))",
    "proposer.tiny_hex()"
)]
pub struct Proposal<B: Blk> {
    pub height:     Height,
    pub round:      Round,
    pub block:      B,
    pub block_hash: Hash,
    pub lock:       Option<PreVoteQC>,
    pub proposer:   Address,
}

impl<B: Blk> Proposal<B> {
    pub fn new(
        height: Height,
        round: Round,
        block: B,
        block_hash: Hash,
        lock: Option<PreVoteQC>,
        proposer: Address,
    ) -> Self {
        Proposal {
            height,
            round,
            block,
            block_hash,
            lock,
            proposer,
        }
    }

    pub fn as_vote(&self) -> Vote {
        Vote {
            height:     self.height,
            round:      self.round,
            block_hash: self.block_hash.clone(),
        }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[display(
    fmt = "{{ height: {}, round: {}, block_hash: {} }}",
    height,
    round,
    "block_hash.tiny_hex()"
)]
pub struct Vote {
    pub height:     Height,
    pub round:      Round,
    pub block_hash: Hash,
}

impl Vote {
    pub fn new(height: Height, round: Round, block_hash: Hash) -> Self {
        Vote {
            height,
            round,
            block_hash,
        }
    }

    pub fn empty_vote(height: Height, round: Round) -> Vote {
        Vote {
            height,
            round,
            block_hash: Hash::default(),
        }
    }

    pub fn is_empty_vote(&self) -> bool {
        self.block_hash == Hash::default()
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Hash)]
#[display(
    fmt = "{{ vote: {}, vote_weight: {}, voter: {} }}",
    vote,
    vote_weight,
    "voter.tiny_hex()"
)]
pub struct SignedPreVote {
    pub vote:        Vote,
    pub vote_weight: Weight,
    pub voter:       Address,
    pub signature:   Signature,
}

impl SignedPreVote {
    pub fn new(vote: Vote, vote_weight: Weight, voter: Address, signature: Signature) -> Self {
        SignedPreVote {
            vote,
            vote_weight,
            voter,
            signature,
        }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Hash)]
#[display(
    fmt = "{{ vote: {}, vote_weight: {}, voter: {} }}",
    vote,
    vote_weight,
    "voter.tiny_hex()"
)]
pub struct SignedPreCommit {
    pub vote:        Vote,
    pub vote_weight: Weight,
    pub voter:       Address,
    pub signature:   Signature,
}

impl SignedPreCommit {
    pub fn new(vote: Vote, vote_weight: Weight, voter: Address, signature: Signature) -> Self {
        SignedPreCommit {
            vote,
            vote_weight,
            voter,
            signature,
        }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[display(fmt = "{}", vote)]
pub struct PreVoteQC {
    pub vote:       Vote,
    pub aggregates: Aggregates,
}

impl PreVoteQC {
    pub fn new(vote: Vote, aggregates: Aggregates) -> Self {
        PreVoteQC { vote, aggregates }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[display(fmt = "{}", vote)]
pub struct PreCommitQC {
    pub vote:       Vote,
    pub aggregates: Aggregates,
}

impl PreCommitQC {
    pub fn new(vote: Vote, aggregates: Aggregates) -> Self {
        PreCommitQC { vote, aggregates }
    }
}

pub type Proof = PreCommitQC;

#[derive(Clone, Debug, Default, Display, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[display(fmt = "{}", "address_bitmap.tiny_hex()")]
pub struct Aggregates {
    pub address_bitmap: Bytes,
    pub signature:      Signature,
}

impl Aggregates {
    pub fn new(address_bitmap: Bytes, signature: Signature) -> Self {
        Aggregates {
            address_bitmap,
            signature,
        }
    }
}

#[derive(Clone, Debug, Default, Display, PartialEq, Eq, Hash)]
#[display(
    fmt = "{{ choke: {}, vote_weight: {}, from: {}, voter: {} }}",
    choke,
    vote_weight,
    "from.clone().map_or(\"None\".to_owned(), |from| format!(\"{}\", from))",
    "voter.tiny_hex()"
)]
pub struct SignedChoke {
    pub choke:       Choke,
    pub vote_weight: Weight,
    pub from:        Option<UpdateFrom>,
    pub voter:       Address,
    pub signature:   Signature,
}

impl SignedChoke {
    pub fn new(
        choke: Choke,
        vote_weight: Weight,
        from: Option<UpdateFrom>,
        voter: Address,
        signature: Signature,
    ) -> Self {
        SignedChoke {
            choke,
            vote_weight,
            from,
            voter,
            signature,
        }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[display(fmt = "{{ height: {}, round: {}}}", height, round)]
pub struct Choke {
    pub height: Height,
    pub round:  Round,
}

impl Choke {
    pub fn new(height: Height, round: Round) -> Self {
        Choke { height, round }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[display(fmt = "{}", choke)]
pub struct ChokeQC {
    pub choke:      Choke,
    pub aggregates: Aggregates,
}

impl ChokeQC {
    pub fn new(choke: Choke, aggregates: Aggregates) -> Self {
        ChokeQC { choke, aggregates }
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpdateFrom {
    #[display(fmt = "pre_vote_qc: {}", _0)]
    PreVoteQC(PreVoteQC),
    #[display(fmt = "pre_commit_qc: {}", _0)]
    PreCommitQC(PreCommitQC),
    #[display(fmt = "choke_qc: {}", _0)]
    ChokeQC(ChokeQC),
}

impl Default for UpdateFrom {
    fn default() -> Self {
        UpdateFrom::PreVoteQC(PreVoteQC::default())
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Serialize, Deserialize)]
#[display(fmt = "{{ height: {}, address: {} }}", height, "address.tiny_hex()")]
pub struct SignedHeight {
    pub height:    Height,
    pub address:   Address,
    pub signature: Signature,
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Serialize, Deserialize)]
#[display(
    fmt = "{{ request_id: {}, request_range: {}, requester: {} }}",
    "request_id.tiny_hex()",
    request_range,
    "requester.tiny_hex()"
)]
pub struct SyncRequest {
    pub request_id:    Hash,
    pub request_range: HeightRange,
    pub requester:     Address,
    pub signature:     Signature,
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Serialize, Deserialize)]
#[display(
    fmt = "{{ request_id: {}, responder: {} }}",
    "request_id.tiny_hex()",
    "responder.tiny_hex()"
)]
pub struct SyncResponse<B: Blk> {
    pub request_id:        Hash,
    pub block_with_proofs: Vec<(B, Proof)>,
    pub responder:         Address,
    pub signature:         Signature,
}

#[derive(Clone, Debug, Default, Display, PartialEq, Eq, Serialize, Deserialize)]
#[display(fmt = "{{ from: {}, to: {} }}", from, to)]
pub struct HeightRange {
    pub from: Height,
    pub to:   Height,
}

impl HeightRange {
    pub fn new(start: Height, number: u64) -> Self {
        HeightRange {
            from: start,
            to:   start + number,
        }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq)]
#[display(
    fmt = "{{ height: {}, block_hash: {}}}",
    height,
    "block_hash.tiny_hex()"
)]
pub struct FetchedFullBlock {
    pub height:     Height,
    pub block_hash: Hash,
    pub full_block: Bytes,
}

impl FetchedFullBlock {
    pub fn new(height: Height, block_hash: Hash, full_block: Bytes) -> Self {
        FetchedFullBlock {
            height,
            block_hash,
            full_block,
        }
    }
}

#[derive(Clone, Debug, Display, Default)]
#[display(
    fmt = "{{ consensus_config: {}, block_states: {} }}",
    consensus_config,
    block_states
)]
pub struct ExecResult<S: St> {
    pub consensus_config: OverlordConfig,
    pub block_states:     BlockState<S>,
}

#[derive(Clone, Debug, Display, Default)]
#[display(fmt = "{{ height: {}, state: {:?} }}", height, state)]
pub struct BlockState<S: St> {
    pub height: Height,
    pub state:  S,
}

impl<S: St> BlockState<S> {
    pub fn new(height: Height, state: S) -> Self {
        BlockState { height, state }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Serialize, Deserialize)]
#[display(
    fmt = "{{ max_exec_behind: {}, auth_config: {}, timer_config: {} }}",
    max_exec_behind,
    auth_config,
    time_config
)]
pub struct OverlordConfig {
    pub max_exec_behind: u64,
    pub auth_config:     AuthConfig,
    pub time_config:     TimeConfig,
}

#[derive(Clone, Debug, Display, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectMode {
    #[display(fmt = "InTurn")]
    InTurn,
    #[display(fmt = "Random")]
    Random,
}

impl Default for SelectMode {
    fn default() -> Self {
        SelectMode::InTurn
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Serialize, Deserialize)]
#[display(
    fmt = "{{ common_ref: {}, mode: {}, auth_list: {} }}",
    common_ref,
    mode,
    "DisplayVec(auth_list.clone())"
)]
pub struct AuthConfig {
    pub common_ref: CommonHex,
    pub mode:       SelectMode,
    pub auth_list:  Vec<Node>,
}

#[derive(Clone, Debug, Display, PartialEq, Eq, Serialize, Deserialize)]
#[display(
    fmt = "{{ interval: {}, propose_ratio: {}, pre_vote_ratio: {}, pre_commit_ratio: {}, brake_ratio: {} }}",
    interval,
    propose_ratio,
    pre_vote_ratio,
    pre_commit_ratio,
    brake_ratio
)]
pub struct TimeConfig {
    pub interval:         u64,
    pub propose_ratio:    u64,
    pub pre_vote_ratio:   u64,
    pub pre_commit_ratio: u64,
    pub brake_ratio:      u64,
}

impl Default for TimeConfig {
    fn default() -> TimeConfig {
        TimeConfig {
            interval:         1000,
            propose_ratio:    15,
            pre_vote_ratio:   10,
            pre_commit_ratio: 7,
            brake_ratio:      10,
        }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq, Serialize, Deserialize)]
#[display(
    fmt = "{{ address: {}, propose_w: {}, vote_w: {} }}",
    "address.tiny_hex()",
    propose_weight,
    vote_weight
)]
pub struct Node {
    pub address:        Address,
    pub pub_key:        PubKeyHex,
    pub propose_weight: Weight,
    pub vote_weight:    Weight,
}

impl Node {
    pub fn new(
        address: Address,
        pub_key: PubKeyHex,
        propose_weight: Weight,
        vote_weight: Weight,
    ) -> Self {
        Node {
            address,
            pub_key,
            propose_weight,
            vote_weight,
        }
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    #[display(fmt = "PreVote")]
    PreVote,
    #[display(fmt = "PreCommit")]
    PreCommit,
    #[display(fmt = "Choke")]
    Choke,
}

impl Default for VoteType {
    fn default() -> Self {
        VoteType::PreVote
    }
}

impl Into<u8> for VoteType {
    fn into(self) -> u8 {
        match self {
            VoteType::PreVote => 0,
            VoteType::PreCommit => 1,
            VoteType::Choke => 2,
        }
    }
}

impl From<u8> for VoteType {
    fn from(s: u8) -> Self {
        match s {
            0 => VoteType::PreVote,
            1 => VoteType::PreCommit,
            2 => VoteType::Choke,
            _ => panic!("Invalid Vote type!"),
        }
    }
}

#[derive(Clone, Debug, Display, Default, PartialEq, Eq)]
#[display(
    fmt = "{{ cum_weight: {}, vote_type: {}, round: {}, block_hash: {} }}",
    cum_weight,
    vote_type,
    round,
    "block_hash.clone().map_or(\"None\".to_owned(), |hash| hash.tiny_hex())"
)]
pub struct CumWeight {
    pub cum_weight: Weight,
    pub vote_type:  VoteType,
    pub round:      Round,
    pub block_hash: Option<Hash>,
}

impl CumWeight {
    pub fn new(
        cum_weight: Weight,
        vote_type: VoteType,
        round: Round,
        block_hash: Option<Hash>,
    ) -> Self {
        CumWeight {
            cum_weight,
            vote_type,
            round,
            block_hash,
        }
    }
}

pub trait TinyHex {
    fn tiny_hex(&self) -> String;
}

impl TinyHex for Bytes {
    fn tiny_hex(&self) -> String {
        let mut hex = hex::encode(self);
        hex.truncate(8);
        hex
    }
}

struct DisplayVec<T: std::fmt::Display>(Vec<T>);

impl<T: std::fmt::Display> std::fmt::Display for DisplayVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "[ ")?;
        for el in &self.0 {
            write!(f, "{}, ", el)?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

/// for test
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestBlock {
    pub pre_hash:    Hash,
    pub height:      Height,
    pub exec_height: Height,
    pub pre_proof:   Proof,
}

impl TestBlock {
    pub fn genesis_block() -> Self {
        TestBlock::default()
    }
}

impl Blk for TestBlock {
    fn fixed_encode(&self) -> Result<Bytes, Box<dyn std::error::Error + Send>> {
        Ok(bincode::serialize(self).map(Bytes::from).unwrap())
    }

    fn fixed_decode(data: &Bytes) -> Result<Self, Box<dyn std::error::Error + Send>> {
        Ok(bincode::deserialize(data.as_ref()).unwrap())
    }

    fn get_block_hash(&self) -> Hash {
        use crate::{Crypto, DefaultCrypto};
        DefaultCrypto::hash(&self.fixed_encode().unwrap())
    }

    fn get_pre_hash(&self) -> Hash {
        self.pre_hash.clone()
    }

    fn get_height(&self) -> Height {
        self.height
    }

    fn get_exec_height(&self) -> Height {
        self.exec_height
    }

    fn get_proof(&self) -> Proof {
        self.pre_proof.clone()
    }
}
