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
use crate::public_txn;
use crate::pbft;
use crate::rock_storage;
use crate::txn_pool;
use crate::staking;
use crate::account;
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
pub static ACCOUNT_CREATION_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("account_creation"));

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
        behaviour.floodsub.subscribe(ACCOUNT_CREATION_TOPIC.clone());
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
        if let Some(hash) = parsed_json["block_hash"].as_str() {
            return Some(hash.to_string());
        }
    }
    None
}
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
                        let path = "./public_blockchain/db";
                        // Open the database and handle the Result
                        let block_db = match rock_storage::open_db(path) {
                            Ok(db) => db,
                            Err(_) => {
                                eprintln!("Failed to open database");
                                return; // This exits the `inject_event` function early.
                            }
                        };
                        let json = serde_json::to_string(&block).expect("can jsonify request");
                        let _ = rock_storage::put_to_db(&block_db, block.public_hash.clone(), &json);
                        let _ = rock_storage::put_to_db(&block_db,"latest_block", &json);
                        let mut mempool = txn_pool::Mempool::get_instance().lock().unwrap();
                        for txn in block.transactions{
                            if let Some(first_txn_id) = txn.first().cloned() {
                                let _ = Txn::update_transaction_status(&first_txn_id,3);
                                mempool.remove_transaction_by_id(first_txn_id);
                            } 
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
            else if msg.topics[0]==Topic::new("txn_pbft_prepared"){
                let received_serialized_data =msg.data;
                // let json_string = String::from_utf8(received_serialized_data).unwrap();
                // info!("Transactions prepared: {:?}",json_string);
                match String::from_utf8(received_serialized_data.to_vec()) {
                    Ok(json_string) => {
                        // Attempt to parse the string as JSON
                        match serde_json::from_str::<Value>(&json_string) {
                            Ok(outer_json) => {
                                if let Some(value) = outer_json["value"].as_str() {
                                    let unescaped_value = value.replace("\\\"", "\"");
                                    match serde_json::from_str::<PublicTxn>(&unescaped_value) {
                                        Ok(txn) => {
                                            let serialized_txn_from_msg = serde_json::to_string(&txn).unwrap();
                                           
                                            let msg_hash_result = Sha256::digest(serialized_txn_from_msg.as_bytes());
                                            let msg_transaction_hash = hex::encode(msg_hash_result);
                                             // Retrieve the expected hash (key) from outer_json
                                            let expected_hash = outer_json["key"].as_str().unwrap_or_default();
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
            }
            else if msg.topics[0] == Topic::new("txn_pbft_commit") {
                let received_serialized_data = msg.data;
                match String::from_utf8(received_serialized_data.to_vec()) {
                    Ok(json_string) => {
                        // First unescape: Remove extra backslashes and outer quotes
                        let unescaped_json_string = json_string.trim_matches('\"').replace("\\\\", "\\").replace("\\\"", "\"");
                        match serde_json::from_str::<Value>(&unescaped_json_string) {
                            Ok(outer_json) => {
                                // Process outer_json as before...
                                if let Some(encoded_value) = outer_json["value"].as_str() {
                                    // The value field is already a valid JSON string
                
                                    match serde_json::from_str::<PublicTxn>(&encoded_value) {
                                        Ok(txn) => {
                                            let mut mempool = txn_pool::Mempool::get_instance().lock().unwrap();
                                            mempool.add_transaction(txn.clone());
                                            println!("Transaction added to Mempool: {:?}",txn.clone().txn_hash.to_string());
            
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
            else if msg.topics[0]==Topic::new("block_pbft_pre_prepared"){
                let received_serialized_data = msg.data;
                let deserialized_data:  HashMap<String, HashMap<String, HashMap<String,String>>> = serde_json::from_slice(&received_serialized_data).expect("Deserialization failed");
                let the_pbft_hash = self.pbft.get_hash_id();
                let mut all_transactions_valid = true;
                let mut txn_hashes_for_root = Vec::new();
                if let Some((first_key, inner_value)) = deserialized_data.iter().next() {
                    println!("The Stored Peer {:?}",first_key);
                    unsafe {
                        LEADER = Some(first_key.to_string());
                        println!("Leader set to: {:?}", LEADER);
                    }
                    for (peer_id, inner_value) in deserialized_data.iter() {
                        for (key, inner_map) in inner_value.iter() {
                            let (valid_txn, txn_hashes) = self.txn.is_txn_valid(key.to_string(), inner_map.clone());
                            if valid_txn {
                                for (txn_hash, _) in inner_map.iter() {
                                    txn_hashes_for_root.push(txn_hash);
                                    Txn::update_transaction_status(&txn_hash,2);
                                }
                            } else {
                                println!("Invalid transactions detected for root hash: {:?}", key);
                                all_transactions_valid = false;
                                break; // Exit the loop if any invalid transaction is found
                            }
                        }
                    }
                    if all_transactions_valid {
                        // Here, use `all_valid_txn_hashes` as needed
                        self.pbft.increment_verification(&the_pbft_hash);
                        if let Some(publisher) = Publisher::get() {
                                let serialized_txn = serde_json::to_string(&txn_hashes_for_root).unwrap_or_default();
                                let mut transactions = Vec::new();
                                for txn_hash in &txn_hashes_for_root {
                                    transactions.push(txn_hash.to_string()); 
                                }
                                let local_peer_id = get_peer_id();
                                let is_leader = unsafe { LEADER.as_ref() }.map(|leader| leader == &local_peer_id).unwrap_or(false);
                                let the_leader = unsafe { LEADER.as_ref() }.unwrap();
                                //if is_leader{
                                    let created_block = handle_create_block_pbft(self.app.clone(), transactions,the_leader);
                                    let json = serde_json::to_string(&created_block).expect("can jsonify request");
                                    //self.app.try_add_block(created_block.clone());
                                    publisher.publish_block("block_pbft_commit".to_string(),json.as_bytes().to_vec())
                                //}
                        }
                    } else {
                        println!("Not all transactions are valid, Block creation process will not proceed.");
                    }   
                } else {
                    println!("The outer HashMap is empty");
                }
            }else if msg.topics[0]==Topic::new("block_pbft_commit"){
                let node_path = rock_storage::open_db("./node/db");
                let local_peer_id = get_peer_id();
                // println!("Local Peer ID {:?} Leader: {:?}", local_peer_id, unsafe { LEADER.as_ref() });
                //let is_leader = unsafe { LEADER.as_ref() == Some(&local_peer_id) };
                let is_leader = unsafe { LEADER.as_ref() }.map(|leader| leader == &local_peer_id).unwrap_or(false);
                        match serde_json::from_slice::<Block>(&msg.data) {
                            Ok(block) => {
                                if let Some(publisher) = Publisher::get() {
                                    info!("Received new block from {}", msg.source.to_string());
                                    let mut mempool = txn_pool::Mempool::get_instance().lock().unwrap();
                                    self.app.try_add_block(block.clone());
                                    let path = "./public_blockchain/db";
                                    // Open the database and handle the Result
                                    let block_db = match rock_storage::open_db(path) {
                                        Ok(db) => db,
                                        Err(_) => {
                                            eprintln!("Failed to open database");
                                            return; // This exits the `inject_event` function early.
                                        }
                                    };
                                    let json = serde_json::to_string(&block).expect("can jsonify request");
                                    let _ = rock_storage::put_to_db(&block_db, block.public_hash.clone(), &json);
                                    let _ = rock_storage::put_to_db(&block_db,"latest_block", &json);
                                    let mut total_gas_cost: u128 = 0; // Initialize total gas cost
                                    for txn_hash in &block.transactions {
                                        if let Some(first_txn_id) = txn_hash.first().cloned() {
                                            println!("Transaction ID:{:?}",first_txn_id);
                                            let txn_detail = Txn::get_transaction_by_id(&first_txn_id).map_err(|_| "");
                                            println!("Gas:{:?}",txn_detail.unwrap().gas_cost);
                                            total_gas_cost += txn_detail.gas_cost;
                                            Txn::update_transaction_status(&first_txn_id,3);
                                            mempool.remove_transaction_by_id(first_txn_id);
                                        } 
                                    }
                                    let node_path = rock_storage::open_db("./node/db");
                                    match node_path {
                                        Ok(db_handle) => {
                                            match rock_storage::get_from_db(&db_handle, "node_id") {
                                                Some(id)=> {
                                                    match staking::NodeStaking::add_to_rewards(id, total_gas_cost) {
                                                        Ok(_) => Ok(()),
                                                        Err(_) =>Err({}) // Assuming you need to convert StakingError to the function's error type
                                                    }
                                                },
                                                None => Err({ // Handle case where the id is not found
                                                })
                                            };
                                        }
                                        Err(e) => {
                                           
                                        }
                                    }

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
            else if msg.topics[0]==Topic::new("private_blocks_genesis_creation"){
                let received_serialized_data =msg.data;
                let json_string = String::from_utf8(received_serialized_data).unwrap();
                if let Some(publisher) = Publisher::get(){
                let created_block = public_block::handle_create_block_private_chain(self.app.clone(),Some(json_string),None,None,None);
                let json = serde_json::to_string(&created_block).expect("can jsonify request");
                let path = "./public_blockchain/db";
                // Open the database and handle the Result
                let block_db = match rock_storage::open_db(path) {
                    Ok(db) => db,
                    Err(_) => {
                        eprintln!("Failed to open database");
                        return; // This exits the `inject_event` function early.
                    }
                };
                let _ = rock_storage::put_to_db(&block_db,created_block.public_hash.clone(),&json);
                self.app.blocks.push(created_block);
                publisher.publish_block("create_blocks".to_string(),json.as_bytes().to_vec())
                }
            } else if msg.topics[0]==Topic::new("hybrid_block_creation")  {
                let received_serialized_data =msg.data;
                let json_string = String::from_utf8(received_serialized_data).unwrap();
                if let Some(publisher) = Publisher::get(){
                    let created_block = public_block::handle_create_block_private_chain(self.app.clone(),Some(json_string),None,None,None);
                    let json = serde_json::to_string(&created_block).expect("can jsonify request");
                    let path = "./public_blockchain/db";
                    // Open the database and handle the Result
                    let block_db = match rock_storage::open_db(path) {
                        Ok(db) => db,
                        Err(_) => {
                            eprintln!("Failed to open database");
                            return; // This exits the `inject_event` function early.
                        }
                    };
                    let _ = rock_storage::put_to_db(&block_db,created_block.public_hash.clone(),&json);
                    self.app.blocks.push(created_block);
                    publisher.publish_block("create_blocks".to_string(),json.as_bytes().to_vec())
                }
            }else if msg.topics[0]==Topic::new("account_creation"){
                let received_serialized_data =msg.data;
                match serde_json::from_slice::<account::Account>(&received_serialized_data) {
                    Ok(acc) => {
                        let path = "./account/db";
                        // Open the database and handle the Result
                        let acc_db = match rock_storage::open_db(path) {
                            Ok(db) => db,
                            Err(_) => {
                                eprintln!("Failed to open database for wallet");
                                return; // This exits the `inject_event` function early.
                            }
                        };
                        let serialized_data = serde_json::to_string(&acc).expect("JSONIFY request");
                        let _ = rock_storage::put_to_db(&acc_db,acc.public_address,&serialized_data);
                    },
                    Err(err) => {
                        error!(
                            "Error creating account on another nodes"
                        );
                    }
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
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.blocks).expect("can jsonify blocks");
    info!("{}", pretty_json);
}
pub fn handle_print_txn(swarm: &Swarm<AppBehaviour>) {
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().txn.transactions).expect("can jsonify transactions");
    info!("{}", pretty_json);
}
