use crate::verkle_tree::VerkleTree;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::BTreeMap;
use crate::publisher::Publisher;
use crate::account;
use crate::gas_calculator;
use crate::rock_storage;
use crate::txn_pool;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;
use secp256k1::{Signature, SecretKey};
use rocket::error;
use serde_json::Value;


fn deserialize_string_to_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct StringOrU128Visitor;

    impl<'de> serde::de::Visitor<'de> for StringOrU128Visitor {
        type Value = u128;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or an integer")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value as u128)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            value.parse::<u128>().map_err(serde::de::Error::custom)
        }
    }

    deserializer.deserialize_any(StringOrU128Visitor)
}

pub fn get_node_pub_account() -> String {
    let db_path = "./node/db";
    let node_path = match rock_storage::open_db(db_path) {
        Ok(path) => path,
        Err(e) => {
        // Handle the error, maybe log it, and then decide what to do next
        panic!("Failed to open database: {:?}", e); // or use some default value or error handling logic
        }
    };
    let node_account = rock_storage::get_from_db(&node_path, "node_id".to_string());
    return node_account.expect("fail to convert to string");
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Txn{
    pub transactions: Vec<String>,
    pub hashed_txn:Vec<String>,
}

#[derive(Debug,PartialEq, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    SimpleTransfer,
    ContractCreation,
    ContractInteraction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivateTxn{
    pub txn_hash: String,
    pub txn_type: TransactionType,  // Added field for transaction type
    pub nonce:i64,
    #[serde(deserialize_with = "deserialize_string_to_u128")]
    pub value: u128,
    #[serde(deserialize_with = "deserialize_string_to_u128")]
    pub gas_cost: u128, 
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

impl TransactionType {
    fn as_str(&self) -> &'static str {
        match self {
            TransactionType::SimpleTransfer => "SimpleTransfer",
            TransactionType::ContractCreation => "ContractCreation",
            TransactionType::ContractInteraction => "ContractInteraction",
        }
    }
}

impl PrivateTxn {
    pub fn set_status(&mut self, new_status: i64) {
        self.status = new_status;
    }
    pub fn set_signature(&mut self, new_signature: Vec<u8>) {
        self.signature = new_signature;
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
                let deserialized_data:PrivateTxn = serde_json::from_str(&inner_value).expect("Deserialization failed");
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
                return (true,array_of_txn);
            }else{
                return (false, Vec::new());
            }
            //let the_outcome:bool= verkle_tree.node_exists_with_root(hash_array,);
    }   

    pub fn get_transaction_by_id(txn_hash: &str) -> Result<PrivateTxn, Box<dyn std::error::Error>> {
        let db_path = "./transactions/db";
        let db_handle = rock_storage::open_db(db_path)?;

        let txn_data = rock_storage::get_from_db(&db_handle, txn_hash.to_string())
            .ok_or("Transaction not found")?; // Handle missing transactions appropriately
 
        let transaction: PrivateTxn = serde_json::from_str(&txn_data)?;

        Ok(transaction)
    }
    pub fn get_transaction_if_processing(txn_hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let db_path = "./transactions/db";
        let db_handle = rock_storage::open_db(db_path)?;

        let txn_data = rock_storage::get_from_db(&db_handle, txn_hash.to_string())
            .ok_or("Transaction not found")?; // Handle missing transactions appropriately
 
        let transaction: PrivateTxn = serde_json::from_str(&txn_data)?;
        if transaction.status==2 {
            Ok(true)
        }else{
            Ok(false)
        }
    }
    // Stage 1: Create and Prepare Transaction
    pub fn create_and_prepare_transaction(
        transaction_type: TransactionType, 
        caller_address: String,
        to_address: String,
        computed_value: u128
    ) -> Result<(String,u128, PrivateTxn), Box<dyn std::error::Error>> {
        let current_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        match transaction_type {
            TransactionType::ContractCreation => {
            //TransactionType::ContractCreation => {
                // Handle contract-specific transactions
                Self::handle_contract_transaction(transaction_type, &caller_address, &to_address, computed_value, current_timestamp)
            },
            _ => {
                // For other transaction types
                let account = match account::get_account_no_swarm(&caller_address) {
                    Ok(Some(acc)) => acc,
                    Ok(None) => return Err("Account not found".into()), // or handle this case as needed
                    Err(e) => return Err(e.into()), // error while fetching account
                };
                
                let nonce = account.get_nonce();
    
                let txn_data = format!(
                    "{}{}{}{}",
                    caller_address, to_address, computed_value, current_timestamp
                );
    
                let txn_hash = Sha256::digest(txn_data.as_bytes());
                let txn_hash_hex = format!("{:x}", txn_hash);
                let serialized_txn = serde_json::to_string(&txn_data);
                let path = "./transactions/db";
                // Open the database and handle the Result
                let db_handle = rock_storage::open_db(path).map_err(|_| "Failed to open database")?;
            
                let gas_cost = computed_value;  // Example gas cost, adjust as needed
    
                let new_txn = PrivateTxn {
                    txn_type: transaction_type,
                    caller_address,
                    to_address,
                    txn_hash: txn_hash_hex.clone(),
                    nonce:nonce +1,
                    value: computed_value ,
                    status: 0, // Placeholder
                    timestamp: current_timestamp,
                    signature: Vec::new(), // Placeholder
                    gas_cost, // Placeholder
                };
                let serialized_txn: Result<String, serde_json::Error> = serde_json::to_string(&new_txn);
                match serialized_txn {
                    Ok(json_string) => {
                        let transaction = rock_storage::put_to_db(&db_handle, txn_hash_hex.clone(),&json_string);
                        Ok((txn_hash_hex, gas_cost, new_txn))
                    },
                    Err(e) => {
                        // Handle the error, for example, by logging or panicking
                        panic!("Failed to serialize transaction: {}", e);
                        return Err(e.into());
                    }
                }
            }
        }
    }

    // Stage 2: Sign and Submit Transaction Block
    pub fn sign_and_submit_transaction(
        public_key:&String,
        txn_hash_hex: String,
        private_key: &SecretKey,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Fetch the transaction details based on the transaction hash
        let path = "./transactions/db";
        // Open the database and handle the Result
        let db_handle = rock_storage::open_db(path).map_err(|_| "Failed to open database")?;

        let mut transaction_serialize_string = rock_storage::get_from_db(&db_handle, txn_hash_hex.clone())
        .ok_or("Transaction not found")?;  // Replace with an
        println!("{}",transaction_serialize_string);
        let deserialized_txn: Result<PrivateTxn, serde_json::Error> = serde_json::from_str(&transaction_serialize_string);
        match deserialized_txn {
            Ok(txn) => {
                let hash_bytes = hex::decode(txn_hash_hex.clone())?;
                let hash_array: [u8; 32] = hash_bytes.try_into().map_err(|_| "Invalid hash length")?;
                let signature_bytes = account::Account::sign_message(&hash_array, private_key)?
                .serialize_compact().to_vec();
                let signature = match Signature::from_compact(signature_bytes.as_slice()) {
                    Ok(sig) => sig,
                    Err(e) => {
                        eprintln!("Failed to convert the signature");
                        return Err(e.into());
                    }
                };
                let verified_signature =  account::Account::verify_signature(&public_key,&hash_array,&signature);
                if verified_signature.is_ok() && verified_signature.unwrap() {

                    // Update the transaction status
                    let mut updated_txn = txn.clone();
                    updated_txn.set_status(1);
                    let signature_bytes = signature.serialize_compact();
                    updated_txn.set_signature(signature_bytes.to_vec());
                    Self::update_transaction_status(&txn.txn_hash,1);
                    rock_storage::put_to_db(&db_handle, txn.txn_hash.to_string(), &serde_json::to_string(&updated_txn)?)?;
                    let serialized_data = serde_json::to_string(&updated_txn)?;

                    println!("Serialized Transaction Data: {}", serialized_data);  
                    // Hash the serialized transaction data using SHA-256
                    let hash_result = Sha256::digest(serialized_data.as_bytes());
                    
                    // Calculate the transaction hash as a hexadecimal string
                    let transaction_hash = hex::encode(hash_result);
                    
                    // Create a dictionary for broadcasting
                    let mut dictionary_data = HashMap::new();
                    dictionary_data.insert("key".to_string(), transaction_hash.to_string());
                    let value_json = serde_json::json!(updated_txn);
                    // Insert the JSON object into the dictionary
                    dictionary_data.insert("value".to_string(), value_json.to_string());

                    // Serialize the dictionary to JSON
                    let serialised_dictionary_json = serde_json::json!(dictionary_data).to_string();
                    
                    println!("Broadcast transactions");
                    if let Some(publisher) = Publisher::get(){
                        let serialised_dictionary_bytes = serialised_dictionary_json.as_bytes().to_vec();
                        publisher.publish_block("txn_pbft_prepared".to_string(), serialised_dictionary_bytes);
                    }
                    
                } else {
                    // If the signature verification fails, handle the error accordingly
                    eprintln!("Signature verification failed");
                    // You can return an error or take other appropriate actions
                }
            }
            Err(e) => {
                // Handle the error, for example, by logging or panicking
                eprintln!("Failed to deserialize transaction: {}", e);
            }
        }
        Ok(())
    }
    // Method to update the status of a specific transaction
    //Update 1 = txn signed
    //Update 2 = processing
    //Update 3 = block confirmed
    pub fn update_transaction_status(
        txn_hash: &str, 
        new_status: i64
    ) -> Result<(), Box<dyn std::error::Error>> {
        let db_path = "./transactions/db";
        let db_handle = rock_storage::open_db(db_path)?;

        let txn_data = rock_storage::get_from_db(&db_handle, txn_hash.to_string())
            .ok_or("Transaction not found")?; // Handle missing transactions appropriately

        let mut transaction: PrivateTxn = serde_json::from_str(&txn_data)?;
        println!("Transaction {:?}",transaction);
        // Update the transaction status
        transaction.set_status(new_status);

        // Serialize and save the updated transaction
        rock_storage::put_to_db(&db_handle, txn_hash.to_string(), &serde_json::to_string(&transaction)?)?;
        if new_status==3{
            println!("Transaction update requested");
            Self::handle_post_complete_block(transaction);
        }
        Ok(())
    }
    pub fn handle_post_complete_block(txn: PrivateTxn) -> Result<(), Box<dyn std::error::Error>> {
        
        if txn.txn_type == TransactionType::SimpleTransfer {
            println!("Handling SimpleTransfer");
            let node_account_key = get_node_pub_account();
            account::Account::transfer(&txn.caller_address, &txn.to_address, txn.value);
            account::Account::transfer(&txn.caller_address, &node_account_key, txn.gas_cost);
            println!("SimpleTransfer handled successfully.");
        }
    
        Ok(())
    }

    fn handle_contract_transaction(
        transaction_type: TransactionType,
        caller_address: &str,
        to_address: &str,
        computed_value: u128,
        current_timestamp: i64
    ) -> Result<(String, u128, PrivateTxn), Box<dyn std::error::Error>> {
        // Handle the contract-specific logic here
        // For example, you might have different steps or calculations for contract transactions
    
        // After processing, create the transaction hash, calculate the gas cost, and prepare the transaction object
        let txn_data = format!(
            "{}{}{}{}{}",
            caller_address,
            to_address,
            computed_value,
            current_timestamp,
            transaction_type.as_str() // Convert the enum to a string representation or similar
        );
        let serialized_txn = serde_json::to_string(&txn_data);
        let path = "./transactions/db";
        // Open the database and handle the Result
        let db_handle = rock_storage::open_db(path).map_err(|_| "Failed to open database")?;
    
        let txn_hash = Sha256::digest(txn_data.as_bytes());
        let txn_hash_hex = format!("{:x}", txn_hash);
        let gas_cost = computed_value; // This is an example function call
        println!("Caller: {:?}",caller_address);
        let account = match account::get_account_no_swarm(&caller_address) {
            Ok(Some(acc)) => acc,
            Ok(None) => return Err("Account not found".into()), // or handle this case as needed
            Err(e) => return Err(e.into()), // error while fetching account
        };
        
        let nonce = account.get_nonce();
        let new_txn = PrivateTxn {
            txn_type: transaction_type,
            caller_address: caller_address.to_string(),
            to_address: to_address.to_string(),
            txn_hash: txn_hash_hex.clone(),
            nonce: nonce+1, // You need to fetch or calculate the correct nonce
            value: computed_value,
            status: 0, // Placeholder
            timestamp: current_timestamp,
            signature: Vec::new(), // Placeholder
            gas_cost,
        };
        let serialized_txn: Result<String, serde_json::Error> = serde_json::to_string(&new_txn);
        match serialized_txn {
            Ok(json_string) => {
                let transaction = rock_storage::put_to_db(&db_handle, txn_hash_hex.clone(),&json_string);
                Ok((txn_hash_hex, gas_cost, new_txn))
            },
            Err(e) => {
                // Handle the error, for example, by logging or panicking
                panic!("Failed to serialize transaction: {}", e);
                return Err(e.into());
            }
        }

    }
    pub fn get_transactions_by_caller(
        caller_address: &str,
    ) -> Result<Vec<PrivateTxn>, Box<dyn std::error::Error>> {
        // Open the database handle
        let path = "./transactions/db";
        let transaction_path = rock_storage::open_db(path);
        match transaction_path {
            Ok(db_handle) => {
                // Retrieve the vector of tuples
                let result_tuples = rock_storage::get_all_from_db(&db_handle);
            
                // Iterate through the database to find transactions with the specified caller_address
                let mut transactions = Vec::new();
                for result in result_tuples {
                    // Handle the error at each iteration
                    match result {
                        (txn_hash, _) => {
                            if let Some(txn_data) = rock_storage::get_from_db(&db_handle, txn_hash.clone()) {
                                if let Ok(transaction) = serde_json::from_str::<PublicTxn>(&txn_data) {
                                    // Check if the caller_address matches
                                    if transaction.caller_address == caller_address {
                                        transactions.push(transaction);
                                    }
                                } else {
                                    // Handle deserialization error
                                    error!("Error deserializing transaction data for hash: {:?}", txn_hash);
                                }
                            } else {
                                // Handle missing transaction data
                                error!("Transaction data not found for hash: {:?}", txn_hash);
                            }
                        }
                    }
                }
                Ok(transactions) 
            }
            Err(e) => {
                return Err(e.into());
            }
        }
        
    }
    pub fn get_transactions_by_sender(
        sender_address: &str,
    ) -> Result<Vec<PrivateTxn>, Box<dyn std::error::Error>> {
        // Open the database handle
        let path = "./transactions/db";
        let transaction_path = rock_storage::open_db(path);
        match transaction_path {
            Ok(db_handle) => {
                // Retrieve the vector of tuples
                let result_tuples = rock_storage::get_all_from_db(&db_handle);
                // Iterate through the database to find transactions with the specified caller_address
                let mut transactions = Vec::new();
                for result in result_tuples {
                    // Handle the error at each iteration
                    match result {
                        (txn_hash, _) => {
                            if let Some(txn_data) = rock_storage::get_from_db(&db_handle, txn_hash.clone()) {
                                if let Ok(transaction) = serde_json::from_str::<PrivateTxn>(&txn_data) {
                                    // Check if the caller_address matches
                                    if transaction.to_address == sender_address {
                                        transactions.push(transaction);
                                    }
                                } else {
                                    // Handle deserialization error
                                    error!("Error deserializing transaction data for hash: {:?}", txn_hash);
                                }
                            } else {
                                // Handle missing transaction data
                                error!("Transaction data not found for hash: {:?}", txn_hash);
                            }
                        }
                    }
                }
                Ok(transactions) 
            }
            Err(e) => {
                return Err(e.into());
            }
        }
        
    }

}
