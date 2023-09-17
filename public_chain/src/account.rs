use std::collections::HashMap;
use secp256k1::{Secp256k1, PublicKey, SecretKey};
use libp2p::{
    floodsub::{Topic},
    swarm::{Swarm},
};
use serde::{Deserialize, Serialize};
use crate::p2p::AppBehaviour;
use crate::rock_storage;
use crate::public_txn::PublicTxn;

pub fn generate_keypair()->(PublicKey,SecretKey) {
    let secp = Secp256k1::new();
    let mut rng = secp256k1::rand::thread_rng();
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);
    (public_key,secret_key)
}

// Account structure
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Account {
    public_address: String,
    balance: f64,   
    nonce: u64,
    contract_address: Option<Vec<String>>,
}

impl Account {
    // Creates a new account
    fn new() -> Self {
        let (public_key,private_key)=generate_keypair();
        Account {
            public_address:public_key.to_string(),
            balance: 1000000.0,
            nonce: 1,
            contract_address: Some(Vec::new())
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
}

pub fn create_account(cmd:&str,swarm:  &mut Swarm<AppBehaviour>) {
    let acc_path = swarm.behaviour().storage_path.get_account();
    let (public_key,private_key) = generate_keypair();
    let account = Account::new();
    let address = account.public_address.clone();
    let serialized_data = serde_json::to_string(&account).expect("can jsonify request");
    let _ = rock_storage::put_to_db(acc_path,address.clone().to_string(),&serialized_data);
    let put_item = rock_storage::get_from_db(acc_path,address.to_string());
    println!("Public Acc: {:?} created",put_item);
    println!("Private Key: {:?} created",private_key);
    println!("Keep your private key safe, its only displayed once. ");
    // let test_sign_signature = PublicTxn::sign_transaction(b"Txn",&private_key);
    // println!("Test Sign Txn {:?}",test_sign_signature);
    // let verified_txn = PublicTxn::verify_transaction(b"Txn",&test_sign_signature,&public_key);
    // println!("Verified Txn Outcome {:?}",verified_txn);
}
pub fn get_account(cmd:&str,swarm:  &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("acc d ") {
        println!("Account Public Key{:?}",data.to_string());
        let acc_path = swarm.behaviour().storage_path.get_account();
        let the_account = rock_storage::get_from_db(acc_path,data.to_string());
        println!("{:?}",the_account);
    }

}

