use chrono::prelude::*;
use libp2p::{
    floodsub::{Topic},
    swarm::{Swarm},
};
use log::{ info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use once_cell::sync::Lazy;
use crate::p2p::AppBehaviour;
use crate::app::App;
use std::collections::HashMap;
use crate::verkle_tree::VerkleTree;
use crate::rock_storage;
use crate::txn_pool;
use crate::p2p;
use crate::txn::Txn;
use crate::publisher::Publisher;
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("create_blocks"));


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub public_hash: String,
    pub previous_hash: String,
    pub private_node:Option<String>,
    pub private_hash:Option<String>,
    pub transactions:Option<Vec<String>>, // determine txn
    pub timestamp: i64,
    pub nonce: u64,
    pub node_verifier: Option<Vec<String>>
}

const DIFFICULTY_PREFIX: &str = "00";
static mut LEADER: Option<String> = None;

pub fn hash_to_binary_representation(hash: &[u8]) -> String {
    let mut res: String = String::default();
    for c in hash {
        res.push_str(&format!("{:b}", c));
    }
    res
}
pub fn calculate_hash(id: u64, timestamp: i64, previous_hash: &str, nonce: u64) -> Vec<u8> {
    let data = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "timestamp": timestamp,
        "nonce": nonce
    });
    let mut hasher = Sha256::new();
    hasher.update(data.to_string().as_bytes());
    hasher.finalize().as_slice().to_owned()
}

pub fn mine_block(id: u64, timestamp: i64, previous_hash: &str) -> (u64, String) {
    info!("mining block...");
    let mut nonce = 0;

    loop {
        if nonce % 100000 == 0 {
            info!("nonce: {}", nonce);
        }
        let hash = calculate_hash(id, timestamp, previous_hash, nonce);
        let binary_hash = hash_to_binary_representation(&hash);
        if binary_hash.starts_with(DIFFICULTY_PREFIX) {
            info!(
                "mined! nonce: {}, hash: {}, binary hash: {}",
                nonce,
                hex::encode(&hash),
                binary_hash
            );
            return (nonce, hex::encode(hash));
        }
        nonce += 1;
    }
}

pub fn pbft_pre_message_block_create_scheduler()->Result<(), Box<dyn std::error::Error>> {
        let mut verkle_tree = VerkleTree::new();
        let mut transactions: HashMap<String, String>= HashMap::new();
        // Fetch transactions from the mempool
        //println!("Block requested");
        let mempool_lock = txn_pool::Mempool::get_instance().lock().unwrap();
        let mempool_transactions = mempool_lock.get_transactions_with_status(5,1); // Assuming this method exists and returns a list of transactions
        if let Some(publisher) = Publisher::get(){
            if mempool_transactions.is_empty() {
                return Ok(());
            }else{
                let mut processing_transactions = Vec::new();
                let mut non_processing_transactions = Vec::new();        
                for txn in mempool_transactions {
                    match Txn::get_transaction_if_processing(&txn.txn_hash) {
                        Ok(is_processing) => {
                            if is_processing {
                                processing_transactions.push(txn);
                            } else {
                                non_processing_transactions.push(txn);
                            }
                        }
                        Err(err) => {
                            // Handle the error, log it or return an error as appropriate
                            return Err(err.into());
                        }
                    }
                }
                if non_processing_transactions.is_empty(){
                    return Ok(());
                }else {
                    for txn in non_processing_transactions {
                        println!("Transactions: {:?}",txn);
                        // Process each transaction
                        let serialized_data = serde_json::to_string(&txn).expect("can jsonify request");
                        let mut hasher = Sha256::new();
                        hasher.update(&serialized_data);
                        let hash_result = hasher.finalize();
                        let hash_hex_string = format!("{:x}", hash_result);
                        // Insert transaction into verkle tree and prepare for broadcast
                        verkle_tree.insert(txn.txn_hash.as_bytes().to_vec(), hash_result.to_vec());
                        let mut dictionary_data = std::collections::HashMap::new();
                        dictionary_data.insert("key".to_string(), txn.txn_hash.clone());
                        dictionary_data.insert("value".to_string(), serialized_data.clone());
                        transactions.insert(txn.txn_hash.clone(), serialized_data);
                        Txn::update_transaction_status(&txn.txn_hash,2);
        
                    }
                    let peer_id = p2p::get_peer_id();
                    let root_hash = verkle_tree.get_root_string();
                    let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
                    map.insert(root_hash.clone(), transactions.clone());
                    let mut map_with_peer_id: HashMap<String, HashMap<String, HashMap<String,String>>> = HashMap::new();
                    map_with_peer_id.insert(peer_id.to_string(), map);
                    let serialised_dictionary = serde_json::to_vec(&map_with_peer_id).unwrap();
                    println!("Broadcasting PBFT blocks...");
        
                    let serialised_dictionary_bytes = serialised_dictionary.to_vec();
                    publisher.publish_block("block_pbft_pre_prepared".to_string(), serialised_dictionary_bytes);
                }

            }

        }
        Ok(())
    }
pub fn handle_create_block_pbft(app: App, transactions: Vec<String>,leader:&String) -> Block {
    let app = app.blocks.last().expect("There should be at least one block");
    let latest_block = app;
    let block = Block::new(
        latest_block.id + 1,
        latest_block.public_hash.clone(),
        transactions,
        None,
        None,
        None,
        [leader.to_string()].to_vec(),
    );
    block
}

pub fn handle_create_block_private_chain(app:App,private_hash:Option<String>,private_node:Option<String>,txn:Option<Vec<String>>,root:Option<String>)-> Block{
    let app = app.blocks.last().expect("There should be at least one block");
    let latest_block = app;
    let transaction = txn.unwrap_or_else(|| vec!["".to_string()]);
    let root_acc = root.unwrap_or_else(|| "".to_string()); // Use default if None
    let p_hash = private_hash.unwrap_or_else(||"".to_string());
    let p_node = private_node.unwrap_or_else(||"".to_string());
    let block = Block::new(
        latest_block.id +1,
        latest_block.public_hash.clone(),
        transaction,
        Some(p_node.clone()),
        Some(p_hash.clone()),
        Some(root_acc),
        [" ".to_string()].to_vec(),
    );
    let json = serde_json::to_string(&block).expect("can jsonify request");

    block
}
pub fn get_latest_block_hash()-> Result<Block, Box<dyn std::error::Error>>{
    let path = "./blockchain/db";
    let block_path = rock_storage::open_db(path);

    match block_path {
        Ok(db_handle) => {
            match rock_storage::get_from_db(&db_handle, "latest_block") {
                Some(data) => {
                    let block: Block = serde_json::from_str(&data)?;
                    Ok(block)
                }
                None => Err("Latest block not found".to_string().into()),
            }
        }
        Err(e) => Err(e.to_string().into()),
    }
}

impl Block {
    pub fn new(id: u64, previous_hash: String, txn:Vec<String>,private_node:Option<String>, private_hash: Option<String>,root:Option<String>,verified_node:Vec<String>) -> Self {
        let now = Utc::now();
        let txn_item = txn;
        let root_acc = root.unwrap_or_else(|| "".to_string()); // Use default if None
        let private_hash = private_hash.unwrap_or_else(|| "".to_string()); // Use default if None
        let private_node = private_node.unwrap_or_else(|| "".to_string()); // Use default if None
        let (nonce, public_hash) = mine_block(id, now.timestamp(), &previous_hash);
        Self {
            id,
            public_hash,
            previous_hash,
            private_node:Some(private_node),
            private_hash: Some(private_hash),
            //root_account:Some(root_acc),
            transactions:Some(txn_item),
            timestamp: now.timestamp(),
            nonce,
            node_verifier: Some(verified_node)
        }
    }

    pub fn is_block_valid(block: &Block, previous_block: &Block) -> bool {
        if block.previous_hash != previous_block.public_hash {
            warn!("Public block with id: {} has wrong previous hash", block.id);
            return false;
        } else if !hash_to_binary_representation(
            &hex::decode(&block.public_hash).expect("can decode from hex"),
        )
        .starts_with(DIFFICULTY_PREFIX)
        {
            warn!("Public block with id: {} has invalid difficulty", block.id);
            return false;
        } else if block.id != previous_block.id + 1 {
            warn!(
                "Public block with id: {} is not the next block after the latest: {}",
                block.id, previous_block.id
            );
            return false;
        } else if hex::encode(calculate_hash(
            block.id,
            block.timestamp,
            &block.previous_hash,
            block.nonce,
        )) != block.public_hash
        {
            warn!("Public block with id: {} has invalid hash", block.id);
            return false;
        }
        true
    }
}
