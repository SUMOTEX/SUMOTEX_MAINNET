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
            ["TEST BLOCK CREATION WITH TXN".to_string()].to_vec(),
            None
        );
        let json = serde_json::to_string(&block).expect("can jsonify request");
        behaviour.app.blocks.push(block);
        info!("broadcasting new block");
        behaviour
            .floodsub
            .publish(BLOCK_TOPIC.clone(), json.as_bytes());
    }
}
pub fn handle_finalised_block(swarm: &mut Swarm<AppBehaviour>, block:Block){
    let behaviour = swarm.behaviour_mut();
    let json = serde_json::to_string(&block).expect("can jsonify request");
    behaviour.app.blocks.push(block);
    info!("broadcasting new block");
    behaviour
        .floodsub
        .publish(BLOCK_TOPIC.clone(), json.as_bytes());
}
pub fn handle_create_block_pbft(app:App,root_hash:String,txn:Vec<String>)-> Block{
    let app = app.blocks.last().expect("There should be at least one block");
    let latest_block = app;
    let block = Block::new(
        latest_block.id +1,
        latest_block.public_hash.clone(),
        txn,
        None
    );
    block
}
pub fn handle_create_block_private_chain(app:App,private_hash:String,txn:Option<Vec<String>>)-> Block{
    let app = app.blocks.last().expect("There should be at least one block");
    let transaction = txn.unwrap_or_else(|| vec!["txn".to_string()]);
    let latest_block = app;
    let block = Block::new(
        latest_block.id +1,
        latest_block.public_hash.clone(),
        transaction,
        None
    );
    block

}

impl Block {
    pub fn new(id: u64, previous_hash: String, txn:Vec<String>, private_hash: Option<String>) -> Self {
        let now = Utc::now();
        let txn_item = txn;
        let private_hash = private_hash.unwrap_or_else(|| "PrivateHash".to_string()); // Use default if None
        let (nonce, public_hash) = mine_block(id, now.timestamp(), &previous_hash);
        Self {
            id,
            public_hash,
            private_hash: Some(private_hash),
            timestamp: now.timestamp(),
            previous_hash,
            transactions:Some(txn_item),
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
