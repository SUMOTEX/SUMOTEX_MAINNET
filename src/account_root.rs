use chrono::prelude::*;
use super::{App};
use libp2p::{
    floodsub::{Topic},
    swarm::{Swarm},
};
use log::{ info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use once_cell::sync::Lazy;
use crate::p2p::AppBehaviour;

pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountRoot {
    pub id: u64,
    pub nodes:Vec<String>, // Nodes
    pub transactions:Vec<String>, //Transactions hash
    pub created_timestamp: i64,
    pub nonce: u64,
}

impl RootAccount {

    pub fn new(id: u64, previous_hash: String, txn:Vec<String>) -> Self {
        let now = Utc::now();
    }

}
