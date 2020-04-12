use std::error::Error;

use bytes::Bytes;
use overlord::{Blk, ConsensusConfig, Crypto, DefaultCrypto, Hash, Height, Proof};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub pre_hash:      Hash,
    pub height:        Height,
    pub exec_height:   Height,
    pub pre_proof:     Proof,
    pub state_root:    Hash,
    pub receipt_roots: Vec<Hash>,
    pub tx:            Transaction,
}

impl Block {
    pub fn genesis_block() -> Self {
        Block::default()
    }
}

impl Blk for Block {
    fn encode(&self) -> Result<Bytes, Box<dyn Error + Send>> {
        Ok(bincode::serialize(self).map(Bytes::from).unwrap())
    }

    fn decode(data: &Bytes) -> Result<Self, Box<dyn Error + Send>> {
        Ok(bincode::deserialize(data.as_ref()).unwrap())
    }

    fn get_block_hash(&self) -> Hash {
        DefaultCrypto::hash(&self.encode().unwrap())
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

pub type Transaction = ConsensusConfig;

#[derive(Clone, Debug, Default)]
pub struct ExecState {
    pub state_root:   Hash,
    pub receipt_root: Hash,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullBlock {
    pub block: Block,
}

#[test]
fn test_block_serialization() {
    let block = Block::default();
    println! {"{:?}", block};
    let encode = block.encode().unwrap();
    let decode = Block::decode(&encode).unwrap();
    assert_eq!(decode, block);
}
