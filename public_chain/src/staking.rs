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
    pub fn new(node_address: String,ip_address:String, initial_stake: u64, address_list: HashMap<String, u64>) -> Result<Self, StakingError> {
        let db_path = "./node/db";
        let node_path = rock_storage::open_db(db_path);
        match node_path {
            Ok(db_handle) => {
                if address_list.contains_key(&node_address) {
                    return Err(StakingError::InitialStakeNotIncluded);
                }
                let mut updated_address_list = address_list.clone();
                updated_address_list.insert(node_address.clone(), initial_stake);
            
                let total_stake: u64 = updated_address_list.values().sum();
                if total_stake < MIN_STAKE {
                    return Err(StakingError::MinStakeNotMet);
                }
                let node_info = NodeInfo {
                    node_address: node_address.clone(),
                    ip_address: ip_address.to_string(), // Example default value
                    last_active: current_unix_timestamp().unwrap_or_default(), // Use a sensible default or handle the error
                    is_active: true, // Assuming new nodes are active by default
                };
                // Serialize NodeStaking and NodeInfo to JSON strings
                let node_staking_json = serde_json::to_string(&NodeStaking {
                    node_address: node_address.clone(),
                    total_stake,
                    address_list: updated_address_list.clone(),
                }).map_err(|_| StakingError::SerializationError)?;
        
                let node_info_json = serde_json::to_string(&node_info)
                    .map_err(|_| StakingError::SerializationError)?;
        
                rock_storage::put_to_db(&db_handle, format!("node_staking:{}", &node_address), &node_staking_json)
                    .map_err(|_| StakingError::DatabaseError)?;
                rock_storage::put_to_db(&db_handle, format!("node_info:{}", &node_address), &node_info_json)
                    .map_err(|_| StakingError::DatabaseError)?;
                Ok(NodeStaking {
                    node_address,
                    total_stake,
                    address_list: updated_address_list,
                })
            }
            Err(e) => {
                println!("{:?}",e);
                return Err(StakingError::DatabaseError);
            }
        }
    }
    pub fn get_node_info(node_address: &str) -> Result<Option<NodeInfo>, StakingError> {
        // Construct the key used to store the node staking information.
        let db_path = "./node/db";
        let node_path = rock_storage::open_db(db_path);
        let key = format!("node_info:{}", node_address);
        match node_path {
            Ok(db_handle) => {
                match rock_storage::get_from_db(&db_handle, &key) {
                    Some(node_staking_json) => {
                        // Attempt to deserialize the JSON string back into a `NodeStaking` struct.
                        let node_staking = serde_json::from_str::<NodeInfo>(&node_staking_json)
                            .map_err(|_| StakingError::SerializationError)?;
                        Ok(Some(node_staking))
                    },
                    None => {
                        // No data found for the given node address.
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                println!("{:?}",e);
                return Err(StakingError::DatabaseError);
            }
        }

    }
    pub fn get_node_staking(node_address: &str) -> Result<Option<NodeStaking>, StakingError> {
        // Construct the key used to store the node staking information.
        let db_path = "./node/db";
        let node_path = rock_storage::open_db(db_path);
        let key = format!("node_staking:{}", node_address);
        match node_path {
            Ok(db_handle) => {
                match rock_storage::get_from_db(&db_handle, &key) {
                    Some(node_staking_json) => {
                        // Attempt to deserialize the JSON string back into a `NodeStaking` struct.
                        let node_staking = serde_json::from_str::<NodeStaking>(&node_staking_json)
                            .map_err(|_| StakingError::SerializationError)?;
                        Ok(Some(node_staking))
                    },
                    None => {
                        // No data found for the given node address.
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                println!("{:?}",e);
                return Err(StakingError::DatabaseError);
            }
        }

    }
    pub fn add_staker_to_node_staking(
        node_address: String,
        address: String,
        stake: u64,
    ) -> Result<NodeStaking, StakingError> {
        let db_path = "./node/db";
        let node_path = rock_storage::open_db(db_path);
        let key = format!("node_staking:{}", node_address);
        match node_path {
            Ok(db_handle) => {
                match rock_storage::get_from_db(&db_handle, &key) {
                    Some(node_staking_json) => {
                        // Attempt to deserialize the JSON string back into a `NodeStaking` struct.
                        let mut node_staking = serde_json::from_str::<NodeStaking>(&node_staking_json)
                            .map_err(|_| StakingError::SerializationError)?;
                        node_staking.address_list.insert(address, stake);
                        node_staking.total_stake += stake; // Update the total stake   
                        // Serialize NodeStaking and NodeInfo to JSON strings
                        let node_staking_json = serde_json::to_string(&node_staking).map_err(|_| StakingError::SerializationError)?;
                        rock_storage::put_to_db(&db_handle, format!("node_staking:{}", &node_address), &node_staking_json)
                        .map_err(|_| StakingError::DatabaseError)?;
                        Ok(node_staking)
                    },
                    None => {
                        // No data found for the given node address.
                        Err(StakingError::DatabaseError)
                    }
                }
            }
            Err(e) => {
                println!("{:?}",e);
                return Err(StakingError::DatabaseError);
            }
        }

    }
    
    pub fn distribute_rewards(&mut self, total_rewards: u64) -> Result<(), StakingError> {
        if self.total_stake == 0 {
            // Avoid division by zero if there's no stake in the system
            return Err(StakingError::MinStakeNotMet);
        }

        // Calculate and distribute rewards to each address proportionally
        for (address, stake) in self.address_list.iter_mut() {
            let reward = (*stake as f64 / self.total_stake as f64) * total_rewards as f64;
            *stake += reward as u64;
        }

        // Update the total stake to include the distributed rewards
        self.total_stake += total_rewards;

        Ok(())
    }
}


