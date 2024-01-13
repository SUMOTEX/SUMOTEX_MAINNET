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
    pub fn add_transaction(&mut self, txn: PublicTxn) {
        if !self.transactions.iter().any(|t| t.txn_hash == txn.txn_hash) {
            self.transactions.push(txn);
        }
    }
    pub fn update_transaction_status(&mut self, id: &str, new_status: i64)->bool {
        for txn in &mut self.transactions {
            if txn.txn_hash == id {
                txn.status = new_status;
                return true;
            }else{
                return false;
            }
        }
        return false;
    }
    pub fn get_transactions_with_status(&self, count: usize, target_status: i64) -> Vec<&PublicTxn> {
        let mut result = Vec::new();
    
        for txn in self.transactions.iter().take(count) {
            if txn.status == target_status {
                result.push(txn);
            }
        }
        result
    }

    // Optionally, a method to remove transactions after they are processed
    pub fn remove_transactions(&mut self, count: usize) {
        self.transactions.drain(0..std::cmp::min(count, self.transactions.len()));
    }
    pub fn remove_transaction_by_id(&mut self, id: String) -> Option<PublicTxn> {
        if let Some(pos) = self.transactions.iter().position(|txn| txn.txn_hash == id) {
            Some(self.transactions.remove(pos))
        } else {
            None
        }
    }
    // Singleton access method
    pub fn get_instance() -> &'static Mutex<Mempool> {
        lazy_static! {
            static ref INSTANCE: Mutex<Mempool> = Mutex::new(Mempool::new());
        }
        &INSTANCE
    }
}
