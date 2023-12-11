use crate::public_txn::PublicTxn;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

pub struct Mempool {
    // Assuming transactions are stored in a vector
    transactions: Vec<PublicTxn>,
}

impl Mempool {
    pub fn new() -> Self {
        Mempool {
            transactions: Vec::new(),
        }
    }
    // Method to add a transaction to the mempool
    pub fn add_transaction(&mut self, txn: PublicTxn) {
        self.transactions.push(txn);
    }

    // Method to get a global instance of Mempool
    pub fn get_instance() -> Arc<Mutex<Mempool>> {
        lazy_static! {
            static ref INSTANCE: Arc<Mutex<Mempool>> = Arc::new(Mutex::new(Mempool::new()));
        }
        INSTANCE.clone()
    }
}
