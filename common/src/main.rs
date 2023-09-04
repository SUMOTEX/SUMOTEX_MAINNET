// main.rs
use rocksdb::{DB, Options};
mod common;  // Declare the module
use common::SMTXBridge;  // Use the enum

fn main() {
    let value = SMTXBridge::Ping;
    println!("{:?}", value);
    let path = "_path_for_rocksdb_storage";
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, path).unwrap();
}
