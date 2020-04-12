pub use creep::Context;

use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use async_trait::async_trait;
use bytes::Bytes;
use futures::channel::mpsc::UnboundedSender;

use crate::error::ConsensusError;
use crate::types::{
    Address, BlockState, ExecResult, Hash, Height, HeightRange, OverlordMsg, Proof, Signature,
};

#[async_trait]
pub trait Adapter<B: Blk, S: Clone + Debug + Default>: Send + Sync {
    async fn create_block(
        &self,
        ctx: Context,
        height: Height,
        exec_height: Height,
        pre_hash: Hash,
        pre_proof: Proof,
        block_states: Vec<BlockState<S>>,
    ) -> Result<B, Box<dyn Error + Send>>;

    async fn check_block_states(
        &self,
        ctx: Context,
        block: &B,
        block_states: &[BlockState<S>],
    ) -> Result<(), Box<dyn Error + Send>>;

    async fn fetch_full_block(
        &self,
        ctx: Context,
        block: &B,
    ) -> Result<Bytes, Box<dyn Error + Send>>;

    async fn save_and_exec_block_with_proof(
        &self,
        ctx: Context,
        height: Height,
        full_block: Bytes,
        proof: Proof,
    ) -> Result<ExecResult<S>, Box<dyn Error + Send>>;

    async fn register_network(&self, _ctx: Context, sender: UnboundedSender<OverlordMsg<B>>);

    async fn broadcast(
        &self,
        ctx: Context,
        msg: OverlordMsg<B>,
    ) -> Result<(), Box<dyn Error + Send>>;

    async fn transmit(
        &self,
        ctx: Context,
        to: Address,
        msg: OverlordMsg<B>,
    ) -> Result<(), Box<dyn Error + Send>>;

    async fn get_block_with_proofs(
        &self,
        ctx: Context,
        height_range: HeightRange,
    ) -> Result<Vec<(B, Proof)>, Box<dyn Error + Send>>;

    async fn get_latest_height(&self, ctx: Context) -> Result<Height, Box<dyn Error + Send>>;

    async fn handle_error(&self, ctx: Context, err: ConsensusError);
}

/// should ensure the same serialization results in different environments
pub trait Blk: Clone + Debug + Default + Send + PartialEq + Eq {
    fn encode(&self) -> Result<Bytes, Box<dyn Error + Send>>;

    fn decode(data: &Bytes) -> Result<Self, Box<dyn Error + Send>>;

    fn get_block_hash(&self) -> Hash;

    fn get_pre_hash(&self) -> Hash;

    fn get_height(&self) -> Height;

    fn get_exec_height(&self) -> Height;

    fn get_proof(&self) -> Proof;
}

/// provide DefaultCrypto
pub trait Crypto: Send {
    fn hash(msg: &Bytes) -> Hash;

    fn sign(&self, hash: &Hash) -> Result<Signature, Box<dyn Error + Send>>;

    fn verify_signature(
        &self,
        signature: &Signature,
        hash: &Hash,
        signer: &Address,
    ) -> Result<(), Box<dyn Error + Send>>;

    fn aggregate_sign(
        &self,
        signature_map: HashMap<&Address, &Signature>,
    ) -> Result<Signature, Box<dyn Error + Send>>;

    fn verify_aggregated_signature(
        &self,
        aggregate_signature: &Signature,
        msg_hash: &Hash,
        signers: Vec<&Address>,
    ) -> Result<(), Box<dyn Error + Send>>;
}
