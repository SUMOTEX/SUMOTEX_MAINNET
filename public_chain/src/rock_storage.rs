use rocksdb::{DB,Error, Options,IteratorMode,SingleThreaded,DBWithThreadMode};
use std::io::{self, Write};
use std::fmt::Debug;
use std::str;

#[derive(Debug)]
pub struct StoragePath {
    pub blocks: DB,
    pub transactions: DB,
    pub account:DB,
    pub contract:DB
}

impl StoragePath {
    pub fn new(block_path:DB,txn_path:DB,account_path:DB,contract_path:DB) -> Self {
        Self {
            blocks:block_path,
            transactions:txn_path,
            account:account_path,
            contract:contract_path
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

pub fn put_to_db<K: AsRef<[u8]>, V: ValueHandler>(db: &DB, key: K, value: &V) -> Result<(), Error> {
    db.put(key, value.to_bytes())
}

pub fn get_from_db<K: AsRef<[u8]>>(db: &DB, key: K) -> Option<String> {
    match db.get(key).unwrap() {
        Some(bytes) => Some(String::from_utf8(bytes).unwrap_or_else(|_| "Invalid UTF-8".to_string())),
        None => None,
    }
}

// pub fn update_in_db<K: AsRef<[u8]>>(db: &DB, key: K, append_str: &str) {
//     // Retrieve existing value
//     let mut existing_value = match db.get(&key)? {
//         Some(bytes) => str::from_utf8(&bytes).to_string()
//         //None => return Err(Error::new("Key not found".to_string())),
//     };

//     // Modify the existing value
//     existing_value.push_str(append_str);

//     // Store the modified value back into the database
//     db.put(key, existing_value.as_bytes());
// }

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

pub fn create_storage(path: &str)-> DB{
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, path).unwrap();
    db
}

pub fn store_wasm_in_db(db: &DB, key: &str, wasm_filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Load the WASM file as a byte array
    let wasm_binary = fs::read(wasm_filepath)?;
    // Store in RocksDB
    db.put(key, &wasm_binary)?;
    Ok(())
}

pub fn get_wasm_from_db(db: &DB, key: &str) -> Result<Option<Vec<u8>>, rocksdb::Error> {
    db.get(key)
}
