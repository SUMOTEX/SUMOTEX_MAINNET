use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use secp256k1::Error as Secp256k1Error;
use crate::rock_storage;

const MIN_STAKE: u64 = 1_400_000; // Minimum stake of 1.4 million

// Staking structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeStaking {
    node_address: String,
    total_stake: u64,
    address_list: HashMap<String, u64>, // Add balances field
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeInfo {
    node_address: String,
    last_active: u64, // Timestamp of last activity
    is_active: bool,  // Represents if the node is considered active
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
    InitialStakeNotIncluded,
    Secp256k1Error(Secp256k1Error),
}
impl From<Secp256k1Error> for SigningError {
    fn from(err: Secp256k1Error) -> Self {
        SigningError::Secp256k1Error(err)
    }
}

impl NodeInfo {

    pub fn upsert_node_info(db: &DB, node_info: &NodeInfo) -> Result<(), StakingError> {
        let serialized_info = serde_json::to_string(node_info)?;
        db.put(node_info.node_address.as_bytes(), serialized_info.as_bytes())?;
        Ok(())
    }
    pub fn get_node_info(db: &DB, node_address: &str) -> Result<Option<NodeInfo>, StakingError> {
        match db.get(node_address.as_bytes())? {
            Some(bytes) => {
                let node_info = serde_json::from_slice(&bytes)?;
                Ok(Some(node_info))
            },
            None => Ok(None),
        }
    }
    pub fn list_nodes(db: &DB) -> Result<Vec<NodeInfo>, StakingError> {
        let mut nodes = Vec::new();
        let iter = db.iterator(IteratorMode::Start); // Use appropriate iterator mode for your needs
    
        for item in iter {
            if let Ok((_, value)) = item {
                let node_info: NodeInfo = serde_json::from_slice(&value)?;
                nodes.push(node_info);
            }
        }
        Ok(nodes)
    }
    pub fn mark_active(db: &DB, node_address: &str) -> Result<(), StakingError> {
        if let Some(mut node_info) = Self::get_node_info(db, node_address)? {
            node_info.last_active = current_unix_timestamp();
            node_info.is_active = true; // Mark the node as active
            Self::upsert_node_info(db, &node_info)?;
        }
        Ok(())
    }
}

impl NodeStaking {
    pub fn new(node_address: String, initial_stake: u64, mut address_list: HashMap<String, u64>) -> Result<Self, StakingError> {
        let path = "./node/db";
        // Open the database and handle the Result
        let db_handle = rock_storage::open_db(path).map_err(|_| "Failed to open database")?;

        // Add the initial stake to the address list
        if address_list.contains_key(&node_address) {
            // Handle the case where the initial stake key already exists, which shouldn't normally happen
            return Err(StakingError::InitialStakeNotIncluded);
        }
        address_list.insert(node_address, initial_stake);

        // Calculate the total stake by summing the values in the address list
        let total_stake: u64 = address_list.values().sum();
        
        if total_stake < MIN_STAKE {
            return Err(StakingError::MinStakeNotMet);
        }
        let new_node_staking = NodeStaking {
            node_address: node_address.clone(),
            total_stake,
            address_list,
        };
        let json_string = serde_json::to_string(&new_node_staking)?;
        rock_storage::put_to_db(&db_handle, node_address, &json_string)?;
        Ok(new_node_staking)
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


