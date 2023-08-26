extern crate ring;

use ring::signature::{Ed25519KeyPair, KeyPair, Signature, VerificationAlgorithm, ED25519};
use std::collections::HashMap;
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
    // pub caller_address:u64,
    // pub to_address:u64,
    // pub sig:u64,
    pub status:i64,
    pub timestamp:i64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializeTxn{
    pub txn_hash: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RootTxn{
    pub root_txn_hash: String,
}
// Transaction structure (example)
struct Transaction {
    sender: String,
    recipient: String,
    amount: u64,
    signature: Vec<u8>, // Ed25519 signature
}

// Blockchain state (example)
struct Blockchain {
    accounts: HashMap<String, Ed25519KeyPair>,
}
 // Verify transaction signature
pub  fn verify_transaction(&self, transaction: &Transaction) -> bool {
    if let Some(key_pair) = self.accounts.get(&transaction.sender) {
        let public_key = key_pair.public_key();
        let signature = Signature::new(ED25519, &transaction.signature).unwrap();
        signature.verify(&public_key, transaction.sender.as_bytes()).is_ok()
    } else {
        false
    }
}
fn main() {
    // Create a blockchain with account keys
    let mut accounts = HashMap::new();
    let key_pair = Ed25519KeyPair::generate_pkcs8();
    let public_key_bytes = key_pair.public_key().as_ref().to_vec();
    accounts.insert(hex::encode(public_key_bytes), key_pair);

    let blockchain = Blockchain { accounts };

    // Create and verify a transaction
    let transaction = Transaction {
        sender: hex::encode(key_pair.public_key().as_ref()),
        recipient: String::from("recipient"),
        amount: 100,
        signature: Vec::new(), // Replace with actual signature
    };

    let is_valid = blockchain.verify_transaction(&transaction);

    if is_valid {
        println!("Transaction is valid.");
    } else {
        println!("Transaction is invalid.");
    }
}