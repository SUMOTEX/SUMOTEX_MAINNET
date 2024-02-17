use chrono::prelude::*;
use log::{info,warn};
use log::{error};
use crate::public_block;
use crate::rock_storage;

#[derive(Debug,Clone)]
pub struct App {
    pub blocks: Vec<public_block::Block>,
}

impl App {
    pub fn new() -> Self {
        Self { blocks: vec![]}
    }
    pub fn initialize_from_storage(&mut self) -> Result<(), Box<dyn std::error::Error>> {

        // Try to load the latest block from storage
        match public_block::get_latest_block_hash() {
            Ok(latest_block) => {
                info!("Resuming from block id: {}", latest_block.id);
                // Add the logic to load the entire chain from the storage
                self.blocks.push(latest_block);
            },
            Err(e) => {
                warn!("Could not retrieve the latest block: {:?}", e);
                self.genesis();  // Creating a genesis block if loading failed
            }
        }
        Ok(())
    }
    pub fn get_blocks(&self)->Vec<public_block::Block>{
        self.blocks.clone()
    }
    pub fn genesis(&mut self) {
        let genesis_block = public_block::Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("00Genesis"),
            private_node:Some(String::from("")),
            private_hash:Some(String::from("")),
            //root_account:Some(String::from("")),
            transactions:Some(vec!["".to_string()].into()),
            nonce: 1,
            public_hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
            node_verifier:Some(vec!["".to_string()].into()),

        };
        let db_path = "./public_blockchain/db";
        let db_handle = match rock_storage::open_db(db_path) {
            Ok(handle) => handle,
            Err(_) => {
                error!("Failed to open database");
                return; // or handle the error as appropriate
            }
        };
        let json = serde_json::to_string(&genesis_block).expect("can jsonify request");
        let _ = rock_storage::put_to_db(&db_handle,"latest_block", &json);
        self.blocks.push(genesis_block);
    }
    pub fn try_add_block(&mut self, block: public_block::Block) {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if public_block::Block::is_block_valid(&block, latest_block) {
            self.blocks.push(block);
        } else {
            error!("could not add block - invalid");
        }
    }
    pub fn is_chain_valid(&self, chain: &[public_block::Block]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            //let block_instance = public_block::Block::new();
            if !public_block::Block::is_block_valid(second, first) {
                return false;
            }
        }
        true
    }
    // We always choose the longest valid chain
    pub fn choose_chain(&mut self, local: Vec<public_block::Block>, remote: Vec<public_block::Block>) -> Vec<public_block::Block> {
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);
        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else {
                remote
            }
        } else if is_remote_valid && !is_local_valid {
            remote
        } else if !is_remote_valid && is_local_valid {
            local
        } else {
            panic!("local and remote chains are both invalid");
        }
    }
    pub fn send_message(&mut self, peer_id: String, message: String) {
        println!("Mainnet Peer ID: {:?} Message: {:?}",peer_id.to_string(),message);
        // Implement the logic here to send the message to the desired peer.
        // This would typically involve queuing the message and having your
        // protocol handler process the message from the queue.
    }
}

