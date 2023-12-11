use rocksdb::{DB,Error, Options,IteratorMode};
use std::fmt::Debug;
use std::str;
use std::fs;
use std::path::Path;
use std::sync::{RwLock, Arc};

#[derive(Debug)]
pub struct StoragePath {
    pub blocks: DB,
    pub transactions: DB,
    pub account:DB,
    pub contract:DB
}
const BLOCKS_DB_PATH: &str = "./blocks/db";
const TRANSACTIONS_DB_PATH: &str = "./transactions/db";
const ACCOUNT_DB_PATH: &str = "./account/db";
const CONTRACT_DB_PATH: &str = "./contract/db";

impl StoragePath {
    pub fn new(block_path:DB,txn_path:DB,account_path:DB,contract_path:DB) -> Self {
        let blocks = match create_storage(BLOCKS_DB_PATH) {
            Ok(db) => db,
            Err(e) => {
                // Handle error (e.g., log it)
                // You might want to return an error or a default value
                panic!("Failed to create blocks storage: {}", e);
            }
        };
    
        let transactions = match create_storage(TRANSACTIONS_DB_PATH) {
            Ok(db) => db,
            Err(e) => {
                // Handle error
                panic!("Failed to create transactions storage: {}", e);
            }
        };
    
        let account = match create_storage(ACCOUNT_DB_PATH) {
            Ok(db) => db,
            Err(e) => {
                // Handle error
                panic!("Failed to create account storage: {}", e);
            }
        };
    
        let contract = match create_storage(CONTRACT_DB_PATH) {
            Ok(db) => db,
            Err(e) => {
                // Handle error
                panic!("Failed to create contract storage: {}", e);
            }
        };
    
        Self {
            blocks,
            transactions,
            account,
            contract,
        }
    }
    pub fn get_blocks(&self) -> &DB {
        &self.blocks
    }

    pub fn get_transactions(&self) -> &DB {
        &self.transactions
    }

    pub fn get_account(&self) -> &DB {
        &self.account
    }

    pub fn get_contract(&self) -> &DB {
        &self.contract
    }
}

// Define a trait to handle generic value types
pub trait ValueHandler: Debug {
    fn from_bytes(bytes: Vec<u8>) -> Self;
    fn to_bytes(&self) -> Vec<u8>;
}

// Implement the trait for `String`
impl ValueHandler for String {
    fn from_bytes(bytes: Vec<u8>) -> Self {
        String::from_utf8(bytes).unwrap_or_else(|_| "Invalid UTF-8".to_string())
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}
impl ValueHandler for Vec<u8> {
    fn from_bytes(bytes: Vec<u8>) -> Self {
        bytes
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.clone()
    }
}

pub fn put_to_db<K: AsRef<[u8]>, V: ValueHandler>(db: &DB, key: K, value: &V) -> Result<(), Error> {
    db.put(key, value.to_bytes())
}

pub fn get_from_db<K: AsRef<[u8]>>(db: &DB, key: K) -> Option<String> {
    match db.get(key).unwrap() {
        Some(bytes) => Some(String::from_utf8(bytes).unwrap_or_else(|_| "Invalid UTF-8".to_string())),
        None => None,
    }
}
pub fn get_from_db_vector<K: AsRef<[u8]>>(db: &DB, key: K) -> Option<Vec<u8>> {
    match db.get(key).unwrap() {
        Some(bytes) => Some(bytes),
        None => None,
    }
}

pub fn open_db(db_path: &str) -> Result<DB, Error> {
    let mut opts = Options::default();
    opts.create_if_missing(true); // Creates the database if it does not exist
    let db = DB::open(&opts, db_path)?;
    Ok(db)
}

pub fn get_all_from_db(db: &DB) -> Vec<(String, String)> {
    // Create a new vector to hold our key-value pairs
    let mut results = Vec::new();

    // Create an iterator over the whole database
    let mut iter = db.iterator(IteratorMode::Start);

    for (key, value) in iter {
        let key_str = String::from_utf8(key.to_vec()).unwrap();
        let value_str = String::from_utf8(value.to_vec()).unwrap();
        results.push((key_str, value_str));
    }

    results
}

pub fn create_storage(path: &str)-> Result<DB, Error>{
    let mut opts = Options::default();
    opts.create_if_missing(true);
    match DB::open(&opts, path) {
        Ok(db) => Ok(db),
        Err(e) => {
            eprintln!("Failed to open database: {:?}", e);
            Err(e.into())
            // Handle the error, possibly by creating the missing file or directory, 
            // or by taking other appropriate actions.
            // ...
        }
    }
}
pub fn open_storage(path: &str) -> Result<DB, Box<Error>> {
    // Create an instance of Options, used to configure the database
    let mut opts = Options::default();
    opts.create_if_missing(false); // Do not create a new database if it doesn't exist

    // Attempt to open the database
    match DB::open(&opts, path) {
        Ok(db) => Ok(db),
        Err(e) => Err(Box::new(e)),
    }
}
fn path_exists(filepath: &str) -> bool {
    let path = Path::new(filepath);
    path.exists() && path.is_file()
}
pub fn store_wasm_in_db(db: &DB, key: &str, wasm_filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Load the WASM file as a byte array
    if !path_exists(wasm_filepath) {
        println!("No path");
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Invalid file path")));
    }
    let wasm_binary = fs::read(wasm_filepath)?;
    println!("Binary: {:?}",wasm_binary);
    // Store in RocksDB
    db.put(key, &wasm_binary)?;
    Ok(())
}

pub fn get_wasm_from_db(db: &DB, key: &str) -> Result<Option<Vec<u8>>, rocksdb::Error> {
    db.get(key)
}

struct Storage {
    data: Arc<RwLock<Vec<u8>>>,
}

impl Storage {
    fn new() -> Self {
        Storage {
            data: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn read_data(&self) -> Vec<u8> {
        let data = self.data.read().unwrap();
        data.clone()
    }

    fn write_data(&self, new_data: Vec<u8>) {
        let mut data = self.data.write().unwrap();
        *data = new_data;
    }
}
