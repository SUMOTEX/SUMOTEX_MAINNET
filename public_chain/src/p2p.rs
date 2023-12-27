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
const REQUIRED_VERIFICATIONS: usize = 3; // Example value, adjust as needed


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
            } else if msg.topics[0]==Topic::new("blocks"){
                match serde_json::from_slice::<Block>(&msg.data) {
                    Ok(block) => {
                        info!("Received new block from {}", msg.source.to_string());
                        self.app.try_add_block(block);
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
            //   else if msg.topics[0]==Topic::new("block_pbft_pre_prepared") {
            //     println!("Block Received");
            //     if let Some(publisher) = Publisher::get() {
            //         let serialised_dictionary_bytes = msg.data.to_vec();
            //         match String::from_utf8(serialised_dictionary_bytes) {
            //             Ok(serialized_string) => {
            //                 publisher.publish("block_pbft_prepared".to_string(), serialized_string);
            //             }
            //             Err(e) => {
            //                 println!("Failed to convert bytes to string: {}", e);
            //             }
            //         }
            //     }
            // }    
            else if msg.topics[0]==Topic::new("block_pbft_pre_prepared"){
                let received_serialized_data = msg.data;
                let deserialized_data: HashMap<String, HashMap<String, String>> = serde_json::from_slice(&received_serialized_data).expect("Deserialization failed");
                let the_pbft_hash = self.pbft.get_hash_id();
                println!("The Node: {:?}", the_pbft_hash);
                println!("Deserialized data: {:?}", deserialized_data);
                let mut all_transactions_valid = true;
                let mut txn_hashes_for_root = Vec::new();
                for (key, inner_map) in deserialized_data.iter() {
                    
                    let (valid_txn, txn_hashes) = self.txn.is_txn_valid(key.to_string(), inner_map.clone());
                    if valid_txn {
                        println!("Valid transactions for root hash: {:?}", key);
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
                if all_transactions_valid {
                    println!("All transactions are valid, proceeding with PBFT actions");
                    // Here, use `all_valid_txn_hashes` as needed
                    self.pbft.increment_verification(&the_pbft_hash);
                    if let Some(publisher) = Publisher::get() {
                        // Serialize the list of valid transaction hashes and publish it
                        let serialized_hashes = serde_json::to_string(&txn_hashes_for_root).unwrap_or_default();
                        println!("Serialize Hashes: {:?}", serialized_hashes);
                        publisher.publish("block_pbft_commit".to_string(), serialized_hashes);
                    }
                } else {
                    println!("Not all transactions are valid, PBFT process will not proceed.");
                }
                // Check if the block has been verified the required number of times
                // if self.pbft.is_verified(&json_string, REQUIRED_VERIFICATIONS) {
                //     // The block has been verified enough times, proceed to commit
                // }
            }else if msg.topics[0]==Topic::new("block_pbft_commit"){
                let received_serialized_data =msg.data;
                let json_string = String::from_utf8(received_serialized_data).unwrap();
                if let Some(publisher) = Publisher::get(){
                    //TODO: Add transactions security
                    // Deserialize transactions based on txn_hashes
                    match serde_json::from_str::<String>(&json_string) {
                    Ok(inner_json_string) => {
                        match serde_json::from_str::<Vec<String>>(&inner_json_string) {
                       
                            Ok(txn_hashes) => {
                                let mut transactions = Vec::new();
                                for txn_hash in &txn_hashes {
                                    transactions.push(txn_hash.to_string()); 
                                }
                                let created_block = handle_create_block_pbft(self.app.clone(), transactions);
                                println!("Created Block After Validity: {:?}", created_block);
                        
                                let json = serde_json::to_string(&created_block).expect("can jsonify request");
                                let block_db = self.storage_path.get_blocks();
                                let _ = rock_storage::put_to_db(block_db, created_block.public_hash.clone(), &json);
                                self.app.blocks.push(created_block.clone());
                        
                                let mut mempool = txn_pool::Mempool::get_instance().lock().unwrap();
                                for txn_hash in txn_hashes {
                                    println!("Removing transaction {}", txn_hash);
                                    mempool.remove_transaction_by_id(txn_hash);
                                }
                        
                                publisher.publish_block("blocks".to_string(),json.as_bytes().to_vec())
                            },
                            Err(e) => {
                                // Handle the error if the JSON string could not be parsed
                                println!("Failed to parse transaction hashes: {:?}", e);
                            }
                        }
                    },
                    Err(e) => {
                        // Handle the error if the outer JSON string could not be parsed
                        println!("Failed to parse outer transaction hashes: {:?}", e);
                    }
                }
                }
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
                publisher.publish_block("blocks".to_string(),json.as_bytes().to_vec())
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
                    publisher.publish_block("blocks".to_string(),json.as_bytes().to_vec())
                }
            }
            // else if msg.topics[0]==Topic::new("transactions")  {
            //     let received_serialized_data =msg.data;
            //     let json_string = String::from_utf8(received_serialized_data).unwrap();
            //     println!("Contract creation Transactions: {:?}",json_string);
            //     if let Some(publisher) = Publisher::get(){
            //         let (root,txn) = self.pbft.get_txn(json_string);
            //         let created_block = public_block::handle_create_block_pbft(self.app.clone(),root,txn);
            //         let json = serde_json::to_string(&created_block).expect("can jsonify request");
            //         let block_db = self.storage_path.get_blocks();
            //         let _ = rock_storage::put_to_db(block_db,created_block.public_hash.clone(),&json);
            //         self.app.blocks.push(created_block);
            //         publisher.publish_block("blocks".to_string(),json.as_bytes().to_vec())
            //     }
            // }
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


