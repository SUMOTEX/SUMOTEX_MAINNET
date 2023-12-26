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
use crate::public_app::App;
use std::collections::HashMap;
use crate::verkle_tree::VerkleTree;
use crate::rock_storage;
use crate::txn_pool;
use crate::publisher::Publisher;
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub public_hash: String,
    pub previous_hash: String,
    pub private_hash:Option<String>,
    pub transactions:Option<Vec<String>>, // determine txn
    pub timestamp: i64,
    pub nonce: u64,
}

const DIFFICULTY_PREFIX: &str = "00";

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

pub fn pbft_pre_message_block_create_scheduler() {
        println!("Call PBFT");
        let mut verkle_tree = VerkleTree::new();
        let mut transactions: HashMap<String, String>= HashMap::new();
        // Fetch transactions from the mempool
        let mempool_lock = txn_pool::Mempool::get_instance().lock().unwrap();
        let mempool_transactions = mempool_lock.get_transactions(5); // Assuming this method exists and returns a list of transactions
        for txn in mempool_transactions {
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
            println!("Transactions: {:?}",txn);
        }
        let root_hash = verkle_tree.get_root_string();
        let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
        map.insert(root_hash.clone(), transactions.clone());
        let serialised_dictionary = serde_json::to_vec(&map).unwrap();
        println!("root_hash: {:?}",root_hash);
        println!("Txn: {:?}",transactions.clone());
        println!("Broadcasting pbft blocks...");
        if let Some(publisher) = Publisher::get(){
            let serialised_dictionary_bytes = serialised_dictionary.to_vec();
            publisher.publish_block("block_pbft_pre_prepared".to_string(), serialised_dictionary_bytes);
        }
    }
pub fn handle_create_block(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("create b") {
        let behaviour = swarm.behaviour_mut();
        let latest_block = behaviour
            .app
            .blocks
            .last()
            .expect("there is at least one block");
        let block = Block::new(
            latest_block.id + 1,
            latest_block.public_hash.clone(),
            //TODO txn
            [" ".to_string()].to_vec(),
            None,
            None
        );
        let json = serde_json::to_string(&block).expect("can jsonify request");
        let block_db = behaviour.storage_path.get_blocks();
        rock_storage::put_to_db(block_db,latest_block.public_hash.clone(),&json);
        let the_item: Option<String> = rock_storage::get_from_db(block_db,latest_block.public_hash.clone());
        //println!("Stored block {:?}",the_item);
        behaviour.app.blocks.push(block);
        info!("broadcasting new block");
        
        //rock_storage::put_to_db(latest_block.id+1,);
        behaviour
            .floodsub
            .publish(BLOCK_TOPIC.clone(), json.as_bytes());
    }
}

pub fn handle_finalised_block(swarm: &mut Swarm<AppBehaviour>, block:Block){
    let behaviour = swarm.behaviour_mut();
    let json = serde_json::to_string(&block).expect("can jsonify request");
    let block_db = behaviour.storage_path.get_blocks();
    rock_storage::put_to_db(block_db,block.public_hash.clone(),&json);
    let the_item: Option<String> = rock_storage::get_from_db(block_db,block.public_hash.clone());
    behaviour.app.blocks.push(block);
    info!("Broadcasting new block");
    behaviour
        .floodsub
        .publish(BLOCK_TOPIC.clone(), json.as_bytes());
}

pub fn handle_create_block_pbft(app: App, transactions: Vec<String>) -> Block {
    let app = app.blocks.last().expect("There should be at least one block");
    let latest_block = app;
    let block = Block::new(
        latest_block.id + 1,
        latest_block.public_hash.clone(),
        transactions,
        None,
        None
    );
    block
}

pub fn handle_create_block_private_chain(app:App,private_hash:Option<String>,txn:Option<Vec<String>>,root:Option<String>)-> Block{
    let app = app.blocks.last().expect("There should be at least one block");
    let latest_block = app;
    let transaction = txn.unwrap_or_else(|| vec!["".to_string()]);
    let root_acc = root.unwrap_or_else(|| "".to_string()); // Use default if None
    let p_hash = private_hash.unwrap_or_else(||"".to_string());
    let block = Block::new(
        latest_block.id +1,
        latest_block.public_hash.clone(),
        transaction,
        Some(p_hash.clone()),
        Some(root_acc)
    );
    let json = serde_json::to_string(&block).expect("can jsonify request");

    block
}

impl Block {
    pub fn new(id: u64, previous_hash: String, txn:Vec<String>, private_hash: Option<String>,root:Option<String>) -> Self {
        let now = Utc::now();
        let txn_item = txn;
        let root_acc = root.unwrap_or_else(|| "".to_string()); // Use default if None
        let private_hash = private_hash.unwrap_or_else(|| "".to_string()); // Use default if None
        let (nonce, public_hash) = mine_block(id, now.timestamp(), &previous_hash);
        Self {
            id,
            public_hash,
            previous_hash,
            private_hash: Some(private_hash),
            //root_account:Some(root_acc),
            transactions:Some(txn_item),
            timestamp: now.timestamp(),
            nonce,
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
