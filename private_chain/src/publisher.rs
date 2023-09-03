// publisher.rs
use std::sync::Mutex;
use lazy_static::lazy_static;
use tokio::sync::mpsc;
use crate::private_block;
#[derive(Clone)]
pub struct Publisher {
    sender: mpsc::UnboundedSender<(String, String)>,
    sender_bytes: mpsc::UnboundedSender<(String, Vec<u8>)>,
    //sender_private_block: mpsc::UnboundedSender<(String,private_block::PrivateBlock)>,
}

lazy_static! {
    static ref SINGLETON: Mutex<Option<Publisher>> = Mutex::new(None);
}

impl Publisher {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<(String, String)>, mpsc::UnboundedReceiver<(String, Vec<u8>)>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (sender_bytes, receiver_bytes) = mpsc::unbounded_channel();
        //let (sender_private_block, receiver_block_bytes) = mpsc::unbounded_channel();
        (
            Publisher { 
                sender,
                sender_bytes,
            },
            receiver,
            receiver_bytes,
        )
    }

    pub fn get() -> Option<Publisher> {
        let lock = SINGLETON.lock().unwrap();
        lock.as_ref().cloned()
    }

    pub fn set(publisher: Publisher) {
        let mut lock = SINGLETON.lock().unwrap();
        *lock = Some(publisher);
    }

    pub fn publish(&self, title: String, message: String) {
        self.sender.send((title, message)).expect("Can send publish event");
    }
    pub fn publish_block(&self, title: String, message: Vec<u8>) {
        self.sender_bytes.send((title, message)).expect("Can send publish event for bytes");
    }
    // pub fn publish_private_block(&self,title: String,block:private_block::PrivateBlock){
    //     self.sender_private_block.send((title,block)).expect("Can send private blocks");
    // }
}
