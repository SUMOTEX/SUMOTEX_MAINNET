use chrono::prelude::*;
use libp2p::{
    floodsub::{Topic},
};
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountRoot {
    pub public_address: String,
    pub nodes:Vec<String>, // Nodes
    pub balance: f64,
    pub transactions:Vec<String>, //Transactions hash
    pub network_name:Option<String>,
    pub created_timestamp: i64,
    pub nonce: u64,
}
pub fn generate_public_address() -> String {
    format!("{}",rand::random::<u64>())

}

impl AccountRoot {
    pub fn new() -> Self {
        let now = Utc::now();
        let timestamp: i64 = now.timestamp();
        AccountRoot {
            public_address: generate_public_address(),
            balance: 1000000.0,
            nonce: 1,
            nodes:vec![],
            transactions:vec![],
            network_name:None,
            created_timestamp:timestamp
        }
    }

    pub fn verify_validator(id:u64)->bool {
        true
    }
    pub fn add_network_name(&mut self,name:String){
        self.network_name=Some(name.to_string());
    }
    pub fn get_pub_address(&self)->String{
        return self.public_address.clone()
    }

}
