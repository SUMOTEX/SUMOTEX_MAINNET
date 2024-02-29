use sha2::{Sha256};
use serde::{Deserialize, Serialize};
use libp2p::{
    floodsub::{Topic},
    swarm::{Swarm},
};
use std::collections::HashSet;
use std::collections::HashMap;
use rand::Rng;
use crate::verkle_tree::VerkleTree;
use crate::publisher::Publisher;
use once_cell::sync::Lazy;
use rand::thread_rng;
use rand::distributions::Alphanumeric;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::p2p::AppBehaviour;
use crate::txn::PublicTxn;
use crate::txn::TransactionType;



#[derive(Debug, Clone, Serialize, Deserialize)]
enum Message {
    PrePrepare(u64), 
    Prepare(u64),      
    Commit(u64),        
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PBFTNode {
    id: String,
    verification_hash:String,
    view: u64,
    sequence: u64, //The current stage of the PBFT
    state: HashMap<u64, String>, // Sequence number -> Content
    root_hash: String,
    txn: Vec<String>,
    processed_hashes: HashSet<String>,
    verification_counts: HashMap<String, usize>
}
pub fn generate_fake_signature() -> Vec<u8> {
    vec![0u8; 64] // Assuming a 64-byte signature for illustrative purposes.
}
fn get_transaction_type() -> TransactionType {
    // Some logic to determine the transaction type
    TransactionType::ContractInteraction
}

// pub fn pbft_pre_message_handler(cmd:&str,swarm:  &mut Swarm<AppBehaviour>) {
//     if let Some(data) = cmd.strip_prefix("create txn") {
//         let behaviour =swarm.behaviour_mut();
//         let mut i: i64 =0;
//         let mut verkle_tree = VerkleTree::new();
//         let mut transactions: HashMap<String, String>= HashMap::new();
//         while i<5 {
//             let r = thread_rng()
//             .sample_iter(&Alphanumeric)
//             .take(20)
//             .collect::<Vec<_>>();
//             let s = String::from_utf8_lossy(&r);
//             let current_timestamp: i64 = SystemTime::now()
//             .duration_since(UNIX_EPOCH)
//             .unwrap()
//             .as_secs() as i64;
//             let txn_type = get_transaction_type();
//             let mut latest_txn = PublicTxn{
//                 txn_hash:s.to_string(),
//                 txn_type:txn_type,
//                 gas_cost:0,
//                 caller_address:"sample_caller".to_string(),
//                 to_address:"sample_to".to_string(),
//                 signature:(generate_fake_signature()),
//                 nonce:i as u64,
//                 value:123,
//                 status:1,
//                 timestamp: current_timestamp
//             };
//             let serialized_data = serde_json::to_string(&latest_txn).expect("can jsonify request");
//             // Hash the serialized data
//             let mut hasher = Sha256::new();
//             hasher.update(&serialized_data);
//             let hash_result = hasher.finalize();
//              // Convert the hash bytes to a hexadecimal string
//             let hash_hex_string = format!("{:x}", hash_result);
//             i = i+1;
//             verkle_tree.insert(s.as_bytes().to_vec(), hash_result.to_vec());
//             let mut dictionary_data = std::collections::HashMap::new();
//             dictionary_data.insert("key".to_string(), s.to_string());
//             dictionary_data.insert("value".to_string(), serialized_data.to_string());
//             // Serialize the dictionary data (using a suitable serialization format)
//             let serialised_txn = serde_json::to_vec(&dictionary_data).unwrap();
//             transactions.insert(s.to_string(),serialized_data.to_string());
//             //behaviour.floodsub.publish(TXN_TOPIC.clone(),s.to_string());
//         }
//         let root_hash = verkle_tree.get_root_string();
//         let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
//         map.insert(root_hash.clone(),transactions);
//         let serialised_dictionary = serde_json::to_vec(&map).unwrap();
//         println!("Broadcasting Transactions to nodes");
//         //behaviour.txn.transactions.push(root_hash.clone());
//         behaviour
//             .floodsub
//             .publish(BLOCK_PBFT_PREPREPARED_TOPIC.clone(), serialised_dictionary);
//     }
// }

impl PBFTNode {
    pub fn new(id:String) -> Self {
        let random_data: [u8; 32] = rand::thread_rng().gen();  // Generate 32 random bytes
        let mut hasher = hex::encode(random_data);
        let hex = hasher.to_string();
        Self {
            id,
            verification_hash: hex,
            view: 0,
            root_hash: String::new(),
            txn: Vec::new(),
            processed_hashes: HashSet::new(),
            sequence: 0,
            state: HashMap::new(),
            verification_counts: HashMap::new()
        }
    }

    pub fn pre_prepare(&mut self, root_hash: String, txn: Vec<String>) {
        // Check if the root_hash has already been processed
        if self.processed_hashes.contains(&root_hash) {
            // If it has been processed, do nothing (idempotent behavior)
            println!("Root hash {} has already been processed. Skipping.", root_hash);
            return;
        }

        // Update the state as it's a new root_hash
        self.root_hash = root_hash.clone();
        self.txn = txn;

        // Mark this root_hash as processed
        self.processed_hashes.insert(root_hash);
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

    pub fn get_txn(&mut self,id:String)->(String,Vec<String>){
        return(self.root_hash.clone(),self.txn.clone());

    }
    pub fn get_hash_id(&self) -> String {
        self.verification_hash.clone()
    }
    pub fn increment_verification(&mut self, block_hash: &str) {
        let count = self.verification_counts.entry(block_hash.to_string()).or_insert(0);
        *count += 1;
    }

    // Check if the block has reached the required number of verifications
    pub fn is_verified(&self, block_hash: &str, required_verifications: usize) -> bool {
        let result = self.verification_counts.get(block_hash).map_or(false, |&count| {
            println!("Verification count for block {}: {} (Required: {})", block_hash, count, required_verifications);
            count >= required_verifications
        });
        result
    }
}