use crate::public_txn::PublicTxn;

pub struct Mempool {
    // Assuming transactions are stored in a vector
    transactions: Vec<PublicTxn>,
}

impl Mempool {
    // Method to add a transaction to the mempool
    fn add_transaction(&mut self, txn: PublicTxn) {
        self.transactions.push(txn);
    }
}
