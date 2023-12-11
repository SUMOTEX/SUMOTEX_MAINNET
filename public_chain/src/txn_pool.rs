use crate::public_txn::PublicTxn;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use rocksdb::{DB, Options};

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
    pub fn add_transaction(&mut self, txn: PublicTxn) {
        self.transactions.push(txn);
    }

    pub fn get_transactions(&self, count: usize) -> &[PublicTxn] {
        let end = std::cmp::min(count, self.transactions.len());
        &self.transactions[0..end]
    }

    // Optionally, a method to remove transactions after they are processed
    pub fn remove_transactions(&mut self, count: usize) {
        self.transactions.drain(0..std::cmp::min(count, self.transactions.len()));
    }
    // Singleton access method
    pub fn get_instance() -> &'static Mutex<Mempool> {
        lazy_static! {
            static ref INSTANCE: Mutex<Mempool> = Mutex::new(Mempool::new());
        }
        &INSTANCE
    }
}
