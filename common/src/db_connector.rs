extern crate rocksdb;
use rocksdb::{DB, Options,WriteBatch, IteratorMode};

fn put_to_db<K: AsRef<[u8]>, V: AsRef<[u8]>>(db: &DB, key: K, value: V) -> Result<(), rocksdb::Error> {
    db.put(key, value)
}