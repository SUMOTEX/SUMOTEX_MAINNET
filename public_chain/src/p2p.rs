use libp2p::{
    floodsub::{Floodsub,FloodsubEvent,Topic},
    core::{identity},
    mdns::{Mdns,MdnsEvent},
    NetworkBehaviour, PeerId,
    swarm::{Swarm,NetworkBehaviourEventProcess},
};
use tokio::{
    sync::mpsc,
};
use sha2::{Digest, Sha256};
use crate::verkle_tree::VerkleTree;
use std::collections::HashMap;
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use serde_json::{Value};
use crate::public_app::App;
use crate::pbft::PBFTNode;
use crate::public_block::Block;
use crate::public_txn::Txn;
use crate::rock_storage::StoragePath;
use crate::public_block;
use crate::pbft;
use crate::rock_storage;
use crate::txn_pool;
use crate::public_txn::PublicTxn;
use crate::public_block::handle_create_block_pbft;

// main.rs
use crate::publisher::Publisher;
pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static PRIVATE_BLOCK_GENESIS_CREATION: Lazy<Topic> = Lazy::new(|| Topic::new("private_blocks_genesis_creation"));
pub static HYBRID_BLOCK_CREATION: Lazy<Topic> = Lazy::new(|| Topic::new("hybrid_block_creation"));
//For blocks
pub static BLOCK_PBFT_PREPREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("block_pbft_pre_prepared"));
pub static BLOCK_PBFT_PREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("block_pbft_prepared"));
pub static BLOCK_PBFT_COMMIT_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("block_pbft_commit"));

//Transaction mempool verifications PBFT engine
pub static TXN_PBFT_PREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("txn_pbft_prepared"));
pub static TXN_PBFT_COMMIT_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("txn_pbft_commit"));


#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub receiver: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalChainRequest {
    pub from_peer_id: String,
}

#[derive(Debug)]
pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
    Publish(String, String), // Publish a message to a topic
    PublishBlock(String,Vec<u8>),
}

#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<ChainResponse>,
    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub app: App,
    #[behaviour(ignore)]
    pub txn: Txn,
    #[behaviour(ignore)]
    pub pbft: PBFTNode,
    #[behaviour(ignore)]
    pub storage_path: StoragePath,
}

impl AppBehaviour {
    // Create an Identify service
    pub async fn new(
        app: App,
        txn:Txn,
        pbft:PBFTNode,
        storage_path:StoragePath,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        info!("About to send init event from [BEHAVIOUR]");
        let mut behaviour = Self {
            app,
            txn,
            pbft,
            storage_path,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns"),
            response_sender,
            init_sender,
        };
        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(public_block::BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_PBFT_PREPREPARED_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_PBFT_PREPARED_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_PBFT_COMMIT_TOPIC.clone());
        behaviour.floodsub.subscribe(TXN_PBFT_PREPARED_TOPIC.clone());
        behaviour.floodsub.subscribe(TXN_PBFT_COMMIT_TOPIC.clone());
        behaviour.floodsub.subscribe(PRIVATE_BLOCK_GENESIS_CREATION.clone());
        behaviour.floodsub.subscribe(HYBRID_BLOCK_CREATION.clone());
        behaviour
    }
    fn send_message(&mut self, target: PeerId, message: String) {
        println!("Message Sending {:?}",target)
        // Logic to send the message to the target PeerId
    }
}
#[derive(Debug, Clone)]
enum MyProtocolEvent {
    MessageReceived(PeerId, String),
}

impl NetworkBehaviourEventProcess<MyProtocolEvent> for AppBehaviour {
    fn inject_event(&mut self, event: MyProtocolEvent) {
        match event {
            MyProtocolEvent::MessageReceived(peer, message) => {
                println!("Received message from {}: {}", peer, message);
            }
        }
    }
}
fn extract_key_and_value(json_str: &str) -> Option<(String, String)> {
    // Parse the JSON string into a serde_json::Value
    if let Ok(parsed_json) = serde_json::from_str::<Value>(json_str) {
        // Access the "key" and "value" fields
        if let (Some(key), Some(value)) = (
            parsed_json["key"].as_str(),
            parsed_json["value"].as_str(),
        ) {
            return Some((key.to_string(), value.to_string()));
        }
    }
    None
}
fn extract_block_hash(json_str: &str) -> Option<String> {
    // Parse the JSON string into a serde_json::Value
    if let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(json_str) {
        // Extract the block hash from the JSON object
        // Adjust the key as per your JSON structure
        println!("{:?}",parsed_json.to_string());
        if let Some(hash) = parsed_json["block_hash"].as_str() {
            return Some(hash.to_string());
        }
    }
    None
}
// A fictional function to get the peer_id for a given key
pub fn get_peer_id() -> String {
    // Replace this with your actual logic to obtain the peer_id
    // For now, let's just concatenate "peer_id_" with the key
    format!("{}", PEER_ID.clone())
}
const REQUIRED_VERIFICATIONS: usize = 3; // Example value, adjust as needed
static mut LEADER: Option<String> = None;


// incoming event handler
impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            // if msg.topics[0]
            //info!("Response from {:?}:", msg);
            if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                if resp.receiver == PEER_ID.to_string() {
                    //info!("Response from {}:", msg.source);
                    resp.blocks.iter().for_each(|r| info!("{:?}", r));
                    self.app.blocks = self.app.choose_chain(self.app.blocks.clone(), resp.blocks);
                }
            } else if let Ok(resp) = serde_json::from_slice::<LocalChainRequest>(&msg.data) {
                info!("sending local chain to {}", msg.source.to_string());
                let peer_id = resp.from_peer_id;
                if PEER_ID.to_string() == peer_id {
                    if let Err(e) = self.response_sender.send(ChainResponse {
                        blocks: self.app.blocks.clone(),
                        receiver: msg.source.to_string(),
                    }) {
                        error!("error sending response via channel, {}", e);
                    }
                }
            } else if msg.topics[0]==Topic::new("create_blocks"){
                match serde_json::from_slice::<Block>(&msg.data) {
                    Ok(block) => {
                        info!("Received new block from {}", msg.source.to_string());
                        self.app.try_add_block(block.clone());
                        let block_db = self.storage_path.get_blocks();
                        let json = serde_json::to_string(&block).expect("can jsonify request");
                        //let _ = rock_storage::put_to_db(block_db, block.public_hash.clone(), &block);
                        let _ = rock_storage::put_to_db(block_db,"epoch", &json);
                    },
                    Err(err) => {
                        error!(
                            "Error deserializing block from {}: {}",
                            msg.source.to_string(),
                            err
                        );
                    }
                }
            }
            else if msg.topics[0]==Topic::new("txn_pbft_prepared"){
                let received_serialized_data =msg.data;
                // let json_string = String::from_utf8(received_serialized_data).unwrap();
                // info!("Transactions prepared: {:?}",json_string);
                match String::from_utf8(received_serialized_data.to_vec()) {
                    Ok(json_string) => {
                        println!("{:?}", json_string);
                        // Attempt to parse the string as JSON
                        match serde_json::from_str::<Value>(&json_string) {
                            Ok(outer_json) => {
                                if let Some(value) = outer_json["value"].as_str() {
                                    let unescaped_value = value.replace("\\\"", "\"");
                                    match serde_json::from_str::<PublicTxn>(&unescaped_value) {
                                        Ok(txn) => {
                                            let serialized_txn_from_msg = serde_json::to_string(&txn).unwrap();
                                            println!("Serialized from Message: {}", serialized_txn_from_msg);
                                            
                                            let msg_hash_result = Sha256::digest(serialized_txn_from_msg.as_bytes());
                                            let msg_transaction_hash = hex::encode(msg_hash_result);
                                            println!("Hash from Message: {}", msg_transaction_hash);
                                             // Retrieve the expected hash (key) from outer_json
                                            let expected_hash = outer_json["key"].as_str().unwrap_or_default();
                                            println!("Hashes {:?} NEXT: {:?}",expected_hash,msg_transaction_hash);
                                            if msg_transaction_hash == expected_hash {
                                                println!("Hashes match.");
                                                // Perform actions for a match
                                            } else {
                                                println!("Hashes do not match.");
                                                // Perform actions for a mismatch
                                            }
                                            if let Some(publisher) = Publisher::get() {
                                                publisher.publish("txn_pbft_commit".to_string(), json_string);
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("Failed to parse PublicTxn: {}", e);
                                            // Handle error appropriately
                                        }
                                    }
                                } else {
                                    println!("Key and/or Value not found in JSON.");
                                }
                            },
                            Err(e) => {
                                println!("Failed to parse JSON: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to convert bytes to string: {}", e);
                    }
                }; 
                // if let Some(publisher) = Publisher::get(){
                //     publisher.publish("txn_pbft_commit".to_string(), json_string);
                // }
            }
            else if msg.topics[0] == Topic::new("txn_pbft_commit") {
                println!("Transaction PBFT Commit");
                let received_serialized_data = msg.data;
                match String::from_utf8(received_serialized_data.to_vec()) {
                    Ok(json_string) => {
                        // First unescape: Remove extra backslashes and outer quotes
                        let unescaped_json_string = json_string.trim_matches('\"').replace("\\\\", "\\").replace("\\\"", "\"");
                        println!("Unescaped JSON String: {:?}", unescaped_json_string);
                
                        match serde_json::from_str::<Value>(&unescaped_json_string) {
                            Ok(outer_json) => {
                                // Process outer_json as before...
                                if let Some(encoded_value) = outer_json["value"].as_str() {
                                    // The value field is already a valid JSON string
                                    println!("Inner JSON String: {:?}", encoded_value);
                
                                    match serde_json::from_str::<PublicTxn>(&encoded_value) {
                                        Ok(txn) => {
                                            let mut mempool = txn_pool::Mempool::get_instance().lock().unwrap();
                                            mempool.add_transaction(txn);
                                            println!("Transaction added to Mempool.");
            
                                            // ... rest of your logic to handle txn ...
                                        },
                                        Err(e) => {
                                            eprintln!("Failed to parse PublicTxn: {}", e);
                                        }
                                    }
                                } else {
                                    println!("'value' field not found in JSON.");
                                }
                            },
                            Err(e) => {
                                eprintln!("Failed to parse Unescaped JSON: {}", e);
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to convert bytes to string: {}", e);
                    }
                };
            }
              else if msg.topics[0]==Topic::new("block_pbft_pre_prepared") {
                println!("Block Received");
                let received_serialized_data = msg.data.clone();
                let deserialized_data:  HashMap<String, HashMap<String, HashMap<String,String>>> = serde_json::from_slice(&received_serialized_data).expect("Deserialization failed");
                let the_pbft_hash = self.pbft.get_hash_id();
                if let Some((first_key, inner_value)) = deserialized_data.iter().next() {
                    unsafe {
                        LEADER = Some(first_key.to_string())
                    }
                    let serialised_dictionary = serde_json::to_vec(&deserialized_data).unwrap();
                    // Here, use `all_valid_txn_hashes` as needed
                    self.pbft.increment_verification(&the_pbft_hash);
                    if let Some(publisher) = Publisher::get() {
                            let received_serialized_data = msg.data;
                            publisher.publish_block("block_pbft_prepared".to_string(), received_serialized_data);
                    }
                } else {
                    println!("The outer HashMap is empty");
                }
            }    
            else if msg.topics[0]==Topic::new("block_pbft_prepared"){
                let received_serialized_data = msg.data;
                let deserialized_data:  HashMap<String, HashMap<String, HashMap<String,String>>> = serde_json::from_slice(&received_serialized_data).expect("Deserialization failed");
                let the_pbft_hash = self.pbft.get_hash_id();
                let mut all_transactions_valid = true;
                let mut txn_hashes_for_root = Vec::new();
                if let Some((first_key, inner_value)) = deserialized_data.iter().next() {
                    println!("Key {:?}",first_key);
                    unsafe {
                        LEADER = Some(first_key.to_string())
                    }
                    for (peer_id, inner_value) in deserialized_data.iter() {
                        for (key, inner_map) in inner_value.iter() {
                            let (valid_txn, txn_hashes) = self.txn.is_txn_valid(key.to_string(), inner_map.clone());
                            if valid_txn {
                                for (txn_hash, _) in inner_map.iter() {
                                    println!("Transaction Hashes: {:?}", txn_hash);
                                    txn_hashes_for_root.push(txn_hash);
                                }
                            } else {
                                println!("Invalid transactions detected for root hash: {:?}", key);
                                all_transactions_valid = false;
                                break; // Exit the loop if any invalid transaction is found
                            }
                        }
                    }
                    if all_transactions_valid {
                        println!("All transactions are valid, proceeding with PBFT actions");
                        // Here, use `all_valid_txn_hashes` as needed
                        self.pbft.increment_verification(&the_pbft_hash);
                        if let Some(publisher) = Publisher::get() {
                                let separator = "_xx_";
                                let serialized_txn = serde_json::to_string(&txn_hashes_for_root).unwrap_or_default();
                                println!("{:?}",txn_hashes_for_root);
                                // Serialize the list of valid transaction hashes and publish it
                                // let serialized_hashes = format!(
                                //     "{}_xx_{}",
                                //     first_key,
                                //     txn_hashes_for_root
                                //         .iter()
                                //         .map(|s| s.to_string())
                                //         .collect::<Vec<String>>()
                                //         .join(separator)
                                // );
                                let mut transactions = Vec::new();
                                for txn_hash in &txn_hashes_for_root {
                                    transactions.push(txn_hash.to_string()); 
                                }
                                let created_block = handle_create_block_pbft(self.app.clone(), transactions);
                                let json = serde_json::to_string(&created_block).expect("can jsonify request");
                                let mut mempool = txn_pool::Mempool::get_instance().lock().unwrap();
                                for txn_hash in &txn_hashes_for_root {
                                    println!("Removing transaction {}", txn_hash);
                                    mempool.remove_transaction_by_id(txn_hash.to_string());
                                }
                                publisher.publish_block("block_pbft_commit".to_string(), json.as_bytes().to_vec());
                        }
                    } else {
                        println!("Not all transactions are valid, PBFT process will not proceed.");
                    }
                } else {
                    println!("The outer HashMap is empty");
                }
            }else if msg.topics[0]==Topic::new("block_pbft_commit"){
                let local_peer_id = get_peer_id();
                println!("Local Peer ID {:?} Leader: {:?}", local_peer_id, unsafe { LEADER.as_ref() });
                //let is_leader = unsafe { LEADER.as_ref() == Some(&local_peer_id) };
                let is_leader = unsafe { LEADER.as_ref() }.map(|leader| leader == &local_peer_id).unwrap_or(false);
                    if is_leader {
                        println!("Leader is true");
                        match serde_json::from_slice::<Block>(&msg.data) {
                            Ok(block) => {
                                if let Some(publisher) = Publisher::get() {
                                    info!("Received new block from {}", msg.source.to_string());
                                    let mut mempool = txn_pool::Mempool::get_instance().lock().unwrap();
                                    for txn_hash in &block.transactions {
                                        mempool.remove_transaction_by_id(txn_hash.clone());
                                    }
                                    self.app.try_add_block(block.clone());
                                    let json = serde_json::to_string(&block).expect("can jsonify request");
                                    publisher.publish_block("create_blocks".to_string(),json.as_bytes().to_vec())
                                }

                            },
                            Err(err) => {
                                error!(
                                    "Error deserializing block from {}: {}",
                                    msg.source.to_string(),
                                    err
                                );
                            }
                        }
                    }
       
                // let received_serialized_data =msg.data;
                // let json_string = String::from_utf8(received_serialized_data).unwrap();
                // let mut raw_hashes: Vec<&str> = json_string.split("_xx_").collect();
                // let split_hashes: Vec<String> = raw_hashes
                // .into_iter()
                // .map(|element| element.replace("\"", ""))
                // .collect();
                // let peer_id = &split_hashes[0];
                // let peer_id_str = peer_id.to_string();
                // let local_peer_id = get_peer_id();
                // let txn_hashes_str = &split_hashes[1..];

                // println!("Txn {:?}",txn_hashes_str);
                // if let Some(publisher) = Publisher::get(){
                //     println!("Peer ID {:?}",peer_id_str);
                //     println!("Local Peer{:?}",local_peer_id);
                //     // Check if the local peer is the leader
                //     let is_leader = unsafe { LEADER.as_ref() == Some(&local_peer_id) };
                //     if is_leader {
                        
                //         //TODO: Add transactions security
                //         // let mut transactions = Vec::new();
                //         // for txn_hash in txn_hashes_str {
                //         //     transactions.push(txn_hash.to_string()); 
                //         // }
                //         // let created_block = handle_create_block_pbft(self.app.clone(), transactions);
                //         println!("Created Block After Validity: {:?}", created_block);
                
                //         //let json = serde_json::to_string(&created_block).expect("can jsonify request");
                //         // let block_db = self.storage_path.get_blocks();
                //         // let _ = rock_storage::put_to_db(block_db, created_block.public_hash.clone(), &json);
                //         // let _ = rock_storage::put_to_db(block_db,"epoch", &json);
                //         //self.app.blocks.push(created_block.clone());
                
                //         // let mut mempool = txn_pool::Mempool::get_instance().lock().unwrap();
                //         // for txn_hash in txn_hashes_str {
                //         //     println!("Removing transaction {}", txn_hash);
                //         //     mempool.remove_transaction_by_id(txn_hash.to_string());
                //         // }
                //        //publisher.publish_block("create_blocks".to_string(),json.as_bytes().to_vec())
                //     }
                // }
            }
            else if msg.topics[0]==Topic::new("private_blocks_genesis_creation"){
                let received_serialized_data =msg.data;
                let json_string = String::from_utf8(received_serialized_data).unwrap();
                println!("Private Genesis Block: {:?}",json_string);
                if let Some(publisher) = Publisher::get(){
                let created_block = public_block::handle_create_block_private_chain(self.app.clone(),Some(json_string),None,None);
                let json = serde_json::to_string(&created_block).expect("can jsonify request");
                let block_db = self.storage_path.get_blocks();
                let _ = rock_storage::put_to_db(block_db,created_block.public_hash.clone(),&json);
                self.app.blocks.push(created_block);
                publisher.publish_block("create_blocks".to_string(),json.as_bytes().to_vec())
                }

            } else if msg.topics[0]==Topic::new("hybrid_block_creation")  {
                let received_serialized_data =msg.data;
                let json_string = String::from_utf8(received_serialized_data).unwrap();
                println!("Private Block Transactions: {:?}",json_string);
                if let Some(publisher) = Publisher::get(){
                    let created_block = public_block::handle_create_block_private_chain(self.app.clone(),Some(json_string),None,None);
                    let json = serde_json::to_string(&created_block).expect("can jsonify request");
                    let block_db = self.storage_path.get_blocks();
                    let _ = rock_storage::put_to_db(block_db,created_block.public_hash.clone(),&json);
                    self.app.blocks.push(created_block);
                    publisher.publish_block("create_blocks".to_string(),json.as_bytes().to_vec())
                }
            }
        }
    }
}
impl NetworkBehaviourEventProcess<MdnsEvent> for AppBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub fn trigger_publish(sender: mpsc::UnboundedSender<(String, String)>, title: String, message: String) {
    sender.send((title, message)).expect("Can send publish event");
}
pub fn get_list_peers(swarm: &Swarm<AppBehaviour>) -> Vec<String> {
    info!("Validators:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn handle_print_peers(swarm: &Swarm<AppBehaviour>) {
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn handle_print_chain(swarm: &Swarm<AppBehaviour>) {
    info!("SUMOTEX Blockchain:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.blocks).expect("can jsonify blocks");
    info!("{}", pretty_json);
}
pub fn handle_print_txn(swarm: &Swarm<AppBehaviour>) {
    info!("Transactions:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().txn.transactions).expect("can jsonify transactions");
    info!("{}", pretty_json);
}
pub fn handle_print_raw_txn(swarm: &Swarm<AppBehaviour>) {
    info!("Raw Transactions:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().txn.hashed_txn).expect("can jsonify transactions");
    info!("{}", pretty_json);
}


