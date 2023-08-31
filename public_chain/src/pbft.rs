use sha2::{Sha256,Digest};
use serde::{Deserialize, Serialize};
use libp2p::{
    floodsub::{Topic},
    swarm::{Swarm},
};
use std::collections::HashMap;
use log::{info};
use rand::Rng;
use crate::verkle_tree::VerkleTree;
use once_cell::sync::Lazy;
use rand::thread_rng;
use rand::distributions::Alphanumeric;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::p2p::AppBehaviour;
use crate::public_txn::PublicTxn;

pub static PBFT_PREPREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("pbft_pre_prepared"));

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Message {
    PrePrepare(u64), // View number, Content
    Prepare(u64),       // View number, Sequence number
    Commit(u64),        // View number, Sequence number
}
pub struct PBFTNode {
    id: String,
    verification_hash:String,
    view: u64,
    sequence: u64, //The current stage of the PBFT
    state: HashMap<u64, String>, // Sequence number -> Content
    root_hash:String,
    txn: Vec<String>,
}

pub fn get_total_pbft_view(swarm: &Swarm<AppBehaviour>)->u64 {
    let view_value = swarm.behaviour().pbft.view;
    view_value
}
pub fn pbft_pre_message_handler(cmd:&str,swarm:  &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("create txn") {
        let behaviour =swarm.behaviour_mut();
        let mut i: i64 =0;
        let mut verkle_tree = VerkleTree::new();
        let mut transactions: HashMap<String, String>= HashMap::new();
        while i<5 {
            let r = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .collect::<Vec<_>>();
            let s = String::from_utf8_lossy(&r);
            let current_timestamp: i64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            let mut latest_txn = PublicTxn{
                txn_hash:s.to_string(),
                nonce:i,
                value:data.to_owned(),
                status:1,
                timestamp: current_timestamp
            };
            let serialized_data = serde_json::to_string(&latest_txn).expect("can jsonify request");
            // Hash the serialized data
            let mut hasher = Sha256::new();
            hasher.update(&serialized_data);
            let hash_result = hasher.finalize();
             // Convert the hash bytes to a hexadecimal string
            let hash_hex_string = format!("{:x}", hash_result);
            i = i+1;
            verkle_tree.insert(s.as_bytes().to_vec(), hash_result.to_vec());
            let mut dictionary_data = std::collections::HashMap::new();
            dictionary_data.insert("key".to_string(), s.to_string());
            dictionary_data.insert("value".to_string(), serialized_data.to_string());
            // Serialize the dictionary data (using a suitable serialization format)
            let serialised_txn = serde_json::to_vec(&dictionary_data).unwrap();
            transactions.insert(s.to_string(),serialized_data.to_string());
            //behaviour.floodsub.publish(TXN_TOPIC.clone(),s.to_string());
        }
        let root_hash = verkle_tree.get_root_string();
        let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
        map.insert(root_hash.clone(),transactions);
        let serialised_dictionary = serde_json::to_vec(&map).unwrap();
        info!("Broadcasting Transactions to nodes");
        //behaviour.txn.transactions.push(root_hash.clone());
        behaviour
            .floodsub
            .publish(PBFT_PREPREPARED_TOPIC.clone(), serialised_dictionary);
    }

}

impl PBFTNode {
    pub fn new(id: String)-> Self{
        // Initialize libp2p transport
        // ...
        //todo: Change the hash to not random, based on txn and a few security measure.
        let random_data: [u8; 32] = rand::thread_rng().gen();  // Generate 32 random bytes
        let mut hasher = hex::encode(random_data);
        let hex = hasher.to_string();
        Self {
            id,
            verification_hash: hex,
            view: 0,
            root_hash:"".to_string(),
            txn:vec!["".to_string()],
            sequence: 0,
            state: HashMap::new(),
        }
    }
    pub fn pre_prepare(&mut self, root_hash: String,txn:Vec<String>) {
        self.root_hash=root_hash;
        self.txn=txn;
    }

    pub fn prepare(&mut self, value_hash: String) {
        // Implement the Prepare phase logic
        // ...
        println!("{:?}",value_hash);
        self.view= self.view +1;
    }
    pub fn number_of_view(self){
        println!("{:?}",self.view);
    }

    pub fn commit(&mut self, value_hash: String) {
        // Implement the Commit phase logic
        // ...
    }
    pub fn reply(&self, sender: u64, response: String) {
        // Implement the logic to send a reply message to the sender
        // ...
    }

    pub fn get_txn(&mut self,id:String)->(String,Vec<String>){
        return(self.root_hash.clone(),self.txn.clone());

    }
    pub fn get_hash_id(&self) -> String {
        self.verification_hash.clone()
    }
}