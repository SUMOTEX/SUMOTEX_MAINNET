use secp256k1::{Secp256k1, PublicKey, SecretKey};
use rocksdb::{DB};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use secp256k1::Message;
use secp256k1::Error as Secp256k1Error;
use crate::rock_storage;

pub fn generate_keypair()->(PublicKey,SecretKey) {
    let secp = Secp256k1::new();
    let mut rng = secp256k1::rand::thread_rng();
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);
    (public_key,secret_key)
}

// Account structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    public_address: String,
    balance: f64,   
    nonce: u64,
    contract_address: Option<Vec<String>>,
    owned_tokens:  Option<HashMap<String, Vec<u64>>>, // Contract address to list of token IDs
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
    fn new() -> Self {
        let (public_key,private_key)=generate_keypair();
        Account {
            public_address:public_key.to_string(),
            balance: 0.0,
            nonce: 1,
            contract_address: Some(Vec::new()),
            owned_tokens: None::<HashMap<String, Vec<u64>>>, // Initially, the account does not own any tokens
        }
    }

    // Deposit an amount into the account
    fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    // Withdraw an amount from the account, returns true if successful
    fn withdraw(&mut self, amount: f64) -> bool {
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
        &self,
        message_bytes: &[u8],
        signature: &secp256k1::Signature,
    ) -> Result<bool, SigningError> {
        let secp = Secp256k1::new();
        let message = Message::from_slice(message_bytes).map_err(|_| SigningError::MessageCreationError)?;
        let public_key = PublicKey::from_slice(&hex::decode(&self.public_address).unwrap()).unwrap(); // This assumes public_address is hex encoded
        Ok(secp.verify(&message, signature, &public_key).is_ok())
    }
    // Add a token to the account's ownership list
    pub fn add_token(&mut self, contract_address: &str, token_id: u64) {
        if self.owned_tokens.is_none() {
            self.owned_tokens = Some(HashMap::new()); // Initialize the HashMap if it's None
        }
        let token_list = self.owned_tokens.as_mut().unwrap().entry(contract_address.to_string()).or_insert_with(Vec::new);
        
    }
    pub fn get_account(account_key: &str, db_handle: &DB) -> Option<Account> {
        let account_data = rock_storage::get_from_db(db_handle, account_key);
        account_data.and_then(|data| serde_json::from_str(&data).ok())
    }

    pub fn transfer(sender_key: &str, receiver_key: &str, amount: f64) -> Result<(), &'static str> {
        let path = "./account/db";
        // Open the database and handle the Result
        let db_handle = rock_storage::open_db(path).map_err(|_| "Failed to open database")?;
    
        let mut sender_account = Self::get_account(sender_key, &db_handle)
            .ok_or("Sender account not found")?;
    
        if sender_account.balance < amount {
            return Err("Insufficient balance");
        }
    
        let mut receiver_account = Self::get_account(receiver_key, &db_handle)
            .ok_or("Receiver account not found")?;
    
        sender_account.balance -= amount;
        receiver_account.balance += amount;
    
        // Assuming save_account expects a reference to a DB handle
        save_account(&sender_account, &db_handle).map_err(|_| "Failed to save sender account")?;
        save_account(&receiver_account, &db_handle).map_err(|_| "Failed to save receiver account")?;
    
        Ok(())
    }
    pub fn get_nonce(&self) -> u64 {
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
    let account = Account::new();
    let serialized_data = serde_json::to_string(&account)?;

    match rock_storage::put_to_db(&account_db, public_key.clone().to_string(), &serialized_data) {
        Ok(_) => println!("Account stored successfully"),
        Err(e) => eprintln!("Failed to store account: {:?}", e),
    }

    println!("Public Acc: {}", public_key); // Avoid logging private key
    Ok((public_key.to_string(), private_key.to_string()))
}


pub fn get_balance(public_key: &str) -> Result<f64, Box<dyn std::error::Error>> {
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

pub fn get_account_no_swarm(account_key: &str) -> Option<Account> {
    let path = "./account/db";
    let account_path = rock_storage::open_db(path);

    match account_path {
        Ok(db_handle) => {
            let account_data = rock_storage::get_from_db(&db_handle, account_key.to_string());
            account_data.and_then(|data| serde_json::from_str(&data).ok())
        }
        Err(_) => None, // Return None in case of error
    }
}


fn save_account(account: &Account, db_handle: &DB) -> Result<(), &'static str> {
    let serialized_data = serde_json::to_string(account).map_err(|_| "Failed to serialize account")?;
    rock_storage::put_to_db(db_handle, account.public_address.clone(), &serialized_data)
        .map_err(|_| "Failed to save account")
}

