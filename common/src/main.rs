// main.rs
use rocksdb::{DB, Options};
mod common;  // Declare the module
mod db_connector;
use common::SMTXBridge;  // Use the enum

fn main() {
    let path = "public_block";
    let mut options = Options::default();
    options.create_if_missing(true);
    let db = DB::open(&options, path)?;

    // Use the generic function
    put_to_db(&db, "my_key", "my_value")?;
    put_to_db(&db, "another_key", vec![1, 2, 3])?;

    Ok(())
}
