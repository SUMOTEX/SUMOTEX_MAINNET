use std::collections::HashMap;
use rocksdb::{IteratorMode,DB};
use serde::{Deserialize, Serialize};
use secp256k1::Error as Secp256k1Error;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::rock_storage;

const MIN_STAKE: u64 = 1_500_000; // Minimum stake of 1.5 million

// Staking structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeStaking {
    pub node_address: String,
    pub total_stake: u64,
    pub address_list: HashMap<String, u64>, // Add balances field
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeInfo {
    pub node_address: String,
    pub ip_address:String,
    pub last_active: u64, // Timestamp of last activity
    pub is_active: bool,  // Represents if the node is considered active
    // Add other relevant fields
}
#[derive(Debug)]
pub enum SigningError {
    Secp256k1Error(Secp256k1Error),
    MessageCreationError,
}
#[derive(Debug)]
pub enum StakingError {
    MinStakeNotMet,
    AddressAlreadyExists,
    SerializationError,
    DatabaseError,
    InitialStakeNotIncluded,
    Secp256k1Error(Secp256k1Error),
}
impl From<Secp256k1Error> for SigningError {
    fn from(err: Secp256k1Error) -> Self {
        SigningError::Secp256k1Error(err)
    }
}
impl From<serde_json::Error> for StakingError {
    fn from(err: serde_json::Error) -> Self {
        // Here you can map serde_json::Error to an appropriate variant of StakingError
        StakingError::SerializationError
    }
}

fn current_unix_timestamp() -> Result<u64, std::time::SystemTimeError> {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)?;
    Ok(since_the_epoch.as_secs())
}


impl NodeStaking {
    pub fn new(node_address: String, initial_stake: u64, address_list: HashMap<String, u64>) -> Result<Self, StakingError> {
        if address_list.contains_key(&node_address) {
            return Err(StakingError::InitialStakeNotIncluded);
        }
        let mut updated_address_list = address_list;
        updated_address_list.insert(node_address.clone(), initial_stake);
    
        let total_stake: u64 = updated_address_list.values().sum();
        if total_stake < MIN_STAKE {
            return Err(StakingError::MinStakeNotMet);
        }
    
        Ok(NodeStaking {
            node_address,
            total_stake,
            address_list: updated_address_list,
        })
    }
    
    pub fn add_staker_to_node_staking(
        mut node_staking: NodeStaking,
        address: String,
        stake: u64,
    ) -> Result<NodeStaking, StakingError> {
        if node_staking.address_list.contains_key(&address) {
            return Err(StakingError::AddressAlreadyExists);
        }
    
        node_staking.address_list.insert(address, stake);
        node_staking.total_stake += stake; // Update the total stake
    
        Ok(node_staking)
    }
    
    
}


