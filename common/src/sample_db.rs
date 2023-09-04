extern crate rocksdb;
use rocksdb::{DB, Options,WriteBatch, IteratorMode};
//PUT
db.put(b"my key", b"my value").unwrap();

//READ
match db.get(b"my key").unwrap() {
    Some(value) => println!("retrieved value: {}", String::from_utf8(value).unwrap()),
    None => println!("value not found"),
}
//DELETE
db.delete(b"my key").unwrap();

//BATCH_WRITING
pub fn batch_writing() {
    let path = "_path_for_rocksdb_storage_with_batch";
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, path).unwrap();
    let mut batch = WriteBatch::default();
    batch.put(b"my key", b"my value");
    batch.put(b"key2", b"value2");
    batch.delete(b"my key");
    db.write(batch);  // Atomically commits the batch
}
pub fn iterating_key() {
    let path = "_path_for_rocksdb_storage_for_iter";
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, path).unwrap();

    let mut iter = db.iterator(IteratorMode::Start); // Always iterates forward
    for (key, value) in iter {
        println!("Saw key {:?} and value {:?}", key, value);
    }
}