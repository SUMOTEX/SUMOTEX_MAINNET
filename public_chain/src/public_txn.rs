use crate::verkle_tree::VerkleTree;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::BTreeMap;
use crate::publisher::Publisher;
use crate::account;
use crate::gas_calculator;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;
use secp256k1::{Secp256k1, PublicKey, SecretKey, Message, Signature};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Txn{
    pub transactions: Vec<String>,
    pub hashed_txn:Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    SimpleTransfer,
    ContractCreation,
    ContractInteraction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicTxn{
    pub txn_hash: String,
    pub txn_type: TransactionType,  // Added field for transaction type
    pub nonce:u64,
    pub value: u64,
    pub gas_cost: u64, 
    pub caller_address:String,
    pub to_address:String,
    pub status:i64,
    pub timestamp:i64,
    pub signature: Vec<u8>,
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
    pub fn sign_message(&self, message_bytes: &[u8], private_key: &SecretKey) -> Result<secp256k1::Signature, secp256k1::Error> {
        let secp = Secp256k1::new();
        let message = Message::from_slice(message_bytes)?;
    
        Ok(secp.sign(&message, private_key))
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
    pub fn generate_fake_signature() -> Vec<u8> {
        vec![0u8; 64] // Assuming a 64-byte signature for illustrative purposes.
    }
    pub fn create_transactions(
        transaction_type: TransactionType, 
        caller_address:String,
        private_key:&SecretKey,
        to_address:String,
        computed_value:u64
    ) {
        let mut verkle_tree = VerkleTree::new();
        let mut transactions: HashMap<String, String>= HashMap::new();
        let current_timestamp: i64 = SystemTime::now() 
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
        // Create a string that represents the transaction data
        let txn_data = format!(
            "{}{}{}{}{}",
            caller_address,
            to_address,
            computed_value,
            current_timestamp,
            match transaction_type {
                TransactionType::SimpleTransfer => "SimpleTransfer",
                TransactionType::ContractCreation => "ContractCreation",
                TransactionType::ContractInteraction => "ContractInteraction",
            },
            // other fields can be added here if necessary
        );
        let mut hasher = Sha256::new();
        hasher.update(txn_data.as_bytes());
        let txn_hash = format!("{:x}", hasher.finalize());    
        let signature_result = account::Account::sign_message(txn_hash.as_bytes(),private_key);
        let signature_option = match signature_result {
            Ok(signature) => {
                // Convert signature to Vec<u8>
                signature.serialize_compact().to_vec()
            },
            Err(e) => {
                // Handle error, like logging or returning an error
                println!("Error: {:?}", e);
                return; // or return an Err if your function returns a Result
            },
        };
        //Replace later
        let gas_cost = 100;
        let account = account::get_account_no_swarm(&caller_address).expect("Account not found");
        let nonce = account.get_nonce();
        let mut latest_txn = PublicTxn{
            txn_type: transaction_type,
            caller_address:caller_address.clone(),
            signature:signature_option,
            to_address:to_address.clone(),
            txn_hash:txn_hash.to_string(),
            nonce,
            value:computed_value,
            status:0,
            timestamp: current_timestamp,
            gas_cost
        };
        let serialized_data = serde_json::to_string(&latest_txn).expect("can jsonify request");
        // Hash the serialized data
        let mut hasher = Sha256::new();
        hasher.update(&serialized_data);
        let hash_result = hasher.finalize();
        // Convert the hash bytes to a hexadecimal string
        let hash_hex_string = format!("{:x}", hash_result);
        verkle_tree.insert(txn_hash.as_bytes().to_vec(), hash_result.to_vec());
        let mut dictionary_data = std::collections::HashMap::new();
        dictionary_data.insert("key".to_string(), txn_hash.to_string());
        dictionary_data.insert("value".to_string(), serialized_data.to_string());
        // Serialize the dictionary data (using a suitable serialization format)
        let serialised_txn = serde_json::to_vec(&dictionary_data).unwrap();
        transactions.insert(txn_hash.to_string(),serialized_data.to_string());
        let root_hash = verkle_tree.get_root_string();
        let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
        map.insert(root_hash.clone(),transactions);
        let serialised_dictionary = serde_json::to_vec(&map).unwrap();
        println!("Broadcasting transactions to nodes");
        //behaviour.txn.transactions.push(root_hash.clone());
        if let Some(publisher) = Publisher::get(){
            publisher.publish_block("pbft_pre_prepared".to_string(),serialised_dictionary)
        }
    }
    // Stage 1: Create and Prepare Transaction
    pub fn create_and_prepare_transaction(
        transaction_type: TransactionType, 
        caller_address: String,
        to_address: String,
        computed_value: u64
    ) -> Result<(String,u64, PublicTxn), Box<dyn std::error::Error>> {
        let current_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let account = account::get_account_no_swarm(&caller_address).expect("Account not found");
        let nonce = account.get_nonce();

        let txn_data = format!(
            "{}{}{}{}",
            caller_address, to_address, computed_value, current_timestamp, // other fields if necessary
        );

        let txn_hash = Sha256::digest(txn_data.as_bytes());
        let txn_hash_hex = format!("{:x}", txn_hash);
        let gas_cost= 1000;
        let new_txn = PublicTxn {
            txn_type: transaction_type,
            caller_address,
            to_address,
            txn_hash: txn_hash_hex.clone(),
            nonce,
            value: computed_value,
            status: 0, // Placeholder
            timestamp: current_timestamp,
            signature: Vec::new(), // Placeholder
            gas_cost: 100 // Placeholder
        };

        Ok((txn_hash_hex, gas_cost, new_txn))
    }

    // Stage 2: Sign and Submit Transaction Block
    pub fn sign_and_submit_transaction(
        txn_hash_hex: String,
        mut transaction: PublicTxn,
        private_key: &SecretKey,
        publisher: &Publisher
    ) -> Result<(), Box<dyn std::error::Error>> {
        let signature = account::Account::sign_message(txn_hash_hex.as_bytes(), private_key)?
            .serialize_compact().to_vec();
        transaction.signature = signature;

        // Update the Verkle tree and prepare the transaction for broadcast
        let mut verkle_tree = VerkleTree::new();
        let serialized_data = serde_json::to_string(&transaction)?;
        let hash_result = Sha256::digest(serialized_data.as_bytes());
        verkle_tree.insert(txn_hash_hex.as_bytes().to_vec(), hash_result.to_vec());

        let mut dictionary_data = HashMap::new();
        dictionary_data.insert("key".to_string(), txn_hash_hex.clone());
        dictionary_data.insert("value".to_string(), serialized_data.clone());
        let serialised_txn = serde_json::to_vec(&dictionary_data)?;
        let root_hash = verkle_tree.get_root_string();

        let mut transactions = HashMap::new();
        transactions.insert(txn_hash_hex, serialized_data);
        let mut map = HashMap::new();
        map.insert(root_hash.clone(), transactions);
        let serialised_dictionary = serde_json::to_vec(&map)?;

        println!("Broadcasting transactions to nodes");
        publisher.publish_block("pbft_pre_prepared".to_string(), serialised_dictionary);

        Ok(())
    }

}
