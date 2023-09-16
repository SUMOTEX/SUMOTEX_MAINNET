use crate::verkle_tree::VerkleTree;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::BTreeMap;
use secp256k1::{Secp256k1, PublicKey, SecretKey, Message, Signature};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Txn{
    pub transactions: Vec<String>,
    pub hashed_txn:Vec<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicTxn{
    pub txn_hash: String,
    pub nonce:i64,
    // pub version:String,
    pub value: String,
    // pub gas_limit: u64,
    pub caller_address:u64,
    pub to_address:u64,
    pub status:i64,
    pub timestamp:i64,
    pub signature: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializeTxn{
    pub txn_hash: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RootTxn{
    pub root_txn_hash: String,
}


impl PublicTxn {
    pub fn sign_transaction(transaction_data: &[u8], secret_key: &SecretKey) -> Signature {
        let secp = Secp256k1::new();
    
        // Hash the transaction data to create a digest
        let mut hasher = Sha256::new();
        hasher.update(transaction_data);
        let digest = hasher.finalize();
    
        // Create a message object from the digest
        let msg = Message::from_slice(&digest).expect("32 bytes, within curve order");
    
        // Sign the message with the secret key
        let sig = secp.sign(&msg, secret_key);
        sig
    }
    
    pub fn verify_transaction(transaction_data: &[u8], signature: &Signature, public_key: &PublicKey) -> bool {
        let secp = Secp256k1::new();
    
        // Hash the transaction data to create a digest
        let mut hasher = Sha256::new();
        hasher.update(transaction_data);
        let digest = hasher.finalize();
    
        // Create a message object from the digest
        let msg = Message::from_slice(&digest).expect("32 bytes, within curve order");
    
        // Verify the message with the public key
        secp.verify(&msg, signature, public_key).is_ok()
    }
}

impl Txn {
    pub fn new() -> Self {
        Self { transactions: vec![],hashed_txn: vec![] }
    }

    pub fn try_add_root_txn(&mut self, txn: String) {
        self.transactions.push(txn);
    }
    
    pub fn try_add_hash_txn(&mut self, txn: String) {
        //self.hashed_txn.push(txn);
    }

    pub fn is_txn_valid(&mut self,root_hash:String, txn_hash: HashMap<String, String>) -> (bool,Vec<String>) {
            //println!("{:?}",theTxn.timestamp);
            //TODO: To do verification on the transactions and store in another place.
            let mut verkle_tree = VerkleTree::new();
            let mut array_of_txn:Vec<String>=Vec::new();
            let mut hashed_root= hex::decode(&root_hash).expect("Failed to decode hex");
            let hash_array: [u8; 32] = hashed_root.try_into().expect("Slice has incorrect length");
            let mut sorted_items = BTreeMap::new();
            for (inner_key, inner_value) in txn_hash.iter() {
                let deserialized_data:PublicTxn = serde_json::from_str(&inner_value).expect("Deserialization failed");
                sorted_items.insert(deserialized_data.nonce, deserialized_data);
            }
            for (_, item) in sorted_items.iter() {
                println!("{:#?}", item.txn_hash);
                let serialized_data = serde_json::to_string(&item).expect("can jsonify request");
                // Hash the serialized data
                let mut hasher = Sha256::new();
                hasher.update(&serialized_data);
                let hash_result = hasher.finalize();
                array_of_txn.push(item.txn_hash.to_string());
                verkle_tree.insert(item.txn_hash.as_bytes().to_vec(), hash_result.to_vec());
            }
            let the_root = verkle_tree.get_root_string();
            if root_hash==the_root{
                //let mut swarm_guard = p2p::lock_swarm().unwrap();
                //p2p::prepared_message_handler();
                return (true,array_of_txn);
            }else{
                return (false, Vec::new());
            }
            //let the_outcome:bool= verkle_tree.node_exists_with_root(hash_array,);
    }   

}
