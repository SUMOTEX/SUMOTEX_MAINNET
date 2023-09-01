use chrono::prelude::*;
use log::{error};
use crate::private_block;

#[derive(Debug,Clone)]
pub struct PrivateApp {
    pub blocks: Vec<private_block::PrivateBlock>,
}


impl PrivateApp {
    pub fn new() -> Self {
        Self { blocks: vec![]}
    }
    
    pub fn genesis(&mut self,acc:String)->private_block::PrivateBlock {
        let genesis_block = private_block::PrivateBlock {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("00Genesis"),
            private_hash:(String::from("00002816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c99")),
            root_account: Some(acc),
            transactions:(vec!["".to_string()].into()),
            nonce: 1,
            public_hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c99".to_string(),
        };
        self.blocks.push(genesis_block.clone());
        return genesis_block.clone();
    }
    pub fn try_add_genesis(&mut self,acc:String) {
        let genesis_block = private_block::PrivateBlock {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("00Genesis"),
            private_hash:(String::from("00002816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c99")),
            root_account:Some(acc),
            transactions:(vec!["".to_string()].into()),
            nonce: 1,
            public_hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c99".to_string(),
        };
        self.blocks.push(genesis_block.clone());
    }
    pub fn try_add_block(&mut self, block: private_block::PrivateBlock) {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if private_block::PrivateBlock::is_block_valid(&block, latest_block) {
            self.blocks.push(block);
        } else {
            error!("could not add block - invalid");
        }
    }
    pub fn is_chain_valid(&self, chain: &[private_block::PrivateBlock]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            //let block_instance = public_block::Block::new();
            if !private_block::PrivateBlock::is_block_valid(second, first) {
                return false;
            }
        }
        true
    }
    // We always choose the longest valid chain
    pub fn choose_chain(&mut self, local: Vec<private_block::PrivateBlock>, remote: Vec<private_block::PrivateBlock>) -> Vec<private_block::PrivateBlock> {
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
}