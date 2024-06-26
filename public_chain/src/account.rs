use secp256k1::{Secp256k1, PublicKey, SecretKey};
use rocksdb::{DB};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use secp256k1::Message;
use secp256k1::Error as Secp256k1Error;
use crate::rock_storage;
use crate::public_txn::TransactionType;
use crate::public_txn;
use crate::publisher::Publisher;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate_keypair()->(PublicKey,SecretKey) {
    let secp = Secp256k1::new();
    let mut rng = secp256k1::rand::thread_rng();
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);
    (public_key,secret_key)
}

// Account structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub public_address: String,
    pub balance: u128,   
    pub nonce: i64,
    pub contract_address: Option<Vec<String>>,
    pub owned_tokens:  Option<HashMap<String, Vec<u64>>>, // Contract address to list of token IDs
}
#[derive(Debug)]
pub enum SigningError {
    Secp256k1Error(Secp256k1Error),
    MessageCreationError,
}

impl From<Secp256k1Error> for SigningError {
    fn from(err: Secp256k1Error) -> Self {
        SigningError::Secp256k1Error(err)
    }
}

impl Account {
    // Creates a new account
    fn new(public_key:PublicKey,timestamp:u64) -> Self {
        Account {
            public_address:public_key.to_string(),
            //TEST
            //PROD: TODO: Change to 0
            balance: 1000000000000000000,
            nonce: 1,
            contract_address: Some(Vec::new()),
            owned_tokens: None::<HashMap<String, Vec<u64>>>, // Initially, the account does not own any tokens
        }
    }

    // Deposit an amount into the account
    fn deposit(&mut self, amount: u128) {
        self.balance += amount;
    }

    // Withdraw an amount from the account, returns true if successful
    fn withdraw(&mut self, amount: u128) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }
    // Increment the nonce, typically called when a transaction is made
    fn increment_nonce(&mut self) {
        self.nonce += 1;
    }
    // Signs a message with the account's private key
    pub fn sign_message( message_bytes: &[u8], private_key: &SecretKey) -> Result<secp256k1::Signature, secp256k1::Error> {
        let secp = Secp256k1::new();
        let message = Message::from_slice(message_bytes)?;
        Ok(secp.sign(&message, private_key))
    } 
    // Verifies the signature of a message
    pub fn verify_signature(
        public_address:&String,
        message_bytes: &[u8],
        signature: &secp256k1::Signature,
    ) -> Result<bool, SigningError> {
        let secp = Secp256k1::new();
        let message = Message::from_slice(message_bytes).map_err(|_| SigningError::MessageCreationError)?;
        let public_key = PublicKey::from_slice(&hex::decode(public_address).unwrap()).unwrap(); // This assumes public_address is hex encoded
        Ok(secp.verify(&message, signature, &public_key).is_ok())
    }
    // Add a token to the account's ownership list
    pub fn add_token(&mut self, contract_address: &str, _token_id: u64) {
        if self.owned_tokens.is_none() {
            self.owned_tokens = Some(HashMap::new()); // Initialize the HashMap if it's None
        }
        let _token_list = self.owned_tokens.as_mut().unwrap().entry(contract_address.to_string()).or_insert_with(Vec::new);
        
    }
    pub fn get_account(pub_key: &str, db_handle: &DB) -> Option<Account> {
        let account_data = rock_storage::get_from_db(db_handle, pub_key.to_string());
        if let Some(data) = &account_data {
            println!("Retrieved account data: {}", data);
        }
        account_data.and_then(|data| serde_json::from_str(&data).ok())
    }
    pub fn transfer(sender_key: &str, receiver_key: &str, amount: u128) -> Result<(), Box<dyn std::error::Error>> {
        // Step 1: Open the database and handle the Result
        let db_handle = open_account_db().map_err(|_| "Failed to open database")?;
        println!("Database opened successfully");
    
        // Step 2: Get sender account
        let mut sender_account = Self::get_account(sender_key, &db_handle)
            .ok_or("Sender account not found")?;
        println!("Sender key: {}, Retrieved sender account: {:?}", sender_key, sender_account);

        // Step 3: Check if sender has enough balance
        if sender_account.balance < amount {
            return Err("Insufficient balance".into());
        }
    
        // Step 4: Get receiver account
        let mut receiver_account = Self::get_account(receiver_key, &db_handle)
            .ok_or("Receiver account not found")?;
        println!("Receiver account retrieved: {:?}", receiver_account);
    
        // Step 5: Perform the transfer
        sender_account.balance -= amount;
        receiver_account.balance += amount;
    
        // Step 6: Save sender account changes
        save_account(&sender_account, &db_handle).map_err(|_| "Failed to save sender account")?;
        println!("Sender account updated: {:?}", sender_account);
    
        // Step 7: Save receiver account changes
        save_account(&receiver_account, &db_handle).map_err(|_| "Failed to save receiver account")?;
        println!("Receiver account updated: {:?}", receiver_account);
    
        println!("Transfer successful");
    
        Ok(())
    }
    
    pub fn get_nonce(&self) -> i64 {
        self.nonce
    }
    
}
fn open_account_db() -> Result<DB, Box<dyn std::error::Error>> {
    let path = "./account/db";
    rock_storage::open_db(path).map_err(|e| e.into())
}

pub fn create_account() -> Result<(String, String), Box<dyn std::error::Error>> {
    let account_db = open_account_db()?;
    
    let (public_key, private_key) = generate_keypair();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let account = Account::new(public_key,timestamp);
    let serialized_data = serde_json::to_string(&account)?;

    match rock_storage::put_to_db(&account_db, public_key.clone().to_string(), &serialized_data) {
        Ok(_) => println!("Account stored successfully"),
        Err(e) => eprintln!("Failed to store account: {:?}", e),
    }
    if let Some(publisher) = Publisher::get(){
        let serialized_data_bytes = serialized_data.as_bytes().to_vec();
        println!("Publishing account creation");
        publisher.publish_block("account_creation".to_string(), serialized_data_bytes);
    }
    Ok((public_key.to_string(), private_key.to_string()))

}


pub fn get_balance(public_key: &str) -> Result<u128, Box<dyn std::error::Error>> {
    let path = "./account/db";
    let account_path = rock_storage::open_db(path);

    match account_path {
        Ok(db_handle) => {
            match rock_storage::get_from_db(&db_handle, public_key.to_string()) {
                Some(data) => {
                    let account: Account = serde_json::from_str(&data)?;
                    Ok(account.balance)
                },
                None => {
                    Err("Account not found".into())
                }
            }
        }
        Err(e) => {
            Err(e.into())
        }
    }
}
pub fn get_account_by_private_key(private_key_str: &str) -> Result<Account, Box<dyn std::error::Error>> {
    // Parse the private key from a string (ensure this is securely handled)
    let private_key = SecretKey::from_str(private_key_str)?;

    // Derive the public key from the private key
    let secp = Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, &private_key);

    // Convert public key to a string or address format as needed
    let public_key_str = public_key.to_string();
    // Lookup the account by its public key or derived address
    let account = lookup_account_by_public_key(&public_key_str)?;

    Ok(account)
}
pub fn lookup_account_by_public_key(_public_key_str: &str) -> Result<Account, Box<dyn std::error::Error>> {
    // Your logic here to find and return the account based on the public key string or derived address
    Err("Not implemented".into())
}
pub fn get_account_no_swarm(account_key: &str) -> Result<Option<Account>, Box<dyn std::error::Error>> {
    let path = "./account/db";
    let account_path = rock_storage::open_db(path)?;

    match rock_storage::get_from_db(&account_path, account_key) {
        Some(data) => {
            // Try to deserialize the account data
            match serde_json::from_str::<Account>(&data) {
                Ok(account) => Ok(Some(account)), // Account found and successfully deserialized
                Err(e) => Err(e.into()), // Error during deserialization
            }
        }
        None => Ok(None), // Account not found
    }
}
pub fn account_exists(account_key: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let path = "./account/db";
    let account_path = rock_storage::open_db(path)?;

    match rock_storage::get_from_db(&account_path, account_key.to_string()) {
        Some(_) => Ok(true),  // Account found
        None => Ok(false),    // Account not found
    }
}
fn save_account(account: &Account, db_handle: &DB) -> Result<(), &'static str> {
    let serialized_data = serde_json::to_string(account).map_err(|_| "Failed to serialize account")?;
    rock_storage::put_to_db(db_handle, account.public_address.clone(), &serialized_data)
        .map_err(|_| "Failed to save account")
}

