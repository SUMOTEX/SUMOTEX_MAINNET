use libp2p::{
    identify::{Identify,IdentifyEvent, IdentifyConfig},
    kad::{Kademlia,KademliaEvent, KademliaConfig},
    core::{identity},
    NetworkBehaviour, PeerId,
    swarm::{Swarm,NetworkBehaviourEventProcess},
};
use libp2p::gossipsub::{Gossipsub,MessageId,GossipsubMessage, GossipsubConfig,GossipsubConfigBuilder,ValidationMode, GossipsubEvent, IdentTopic as Topic, MessageAuthenticity};
use libp2p::kad::store::MemoryStore;
use tokio::{
    sync::mpsc,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use sha2::{Digest, Sha256};
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
static PRIVATE_BLOCK_GENESIS_CREATION: Lazy<Topic> = Lazy::new(|| Topic::new("private_blocks_genesis_creation"));
static HYBRID_BLOCK_CREATION: Lazy<Topic> = Lazy::new(|| Topic::new("hybrid_block_creation"));

// For blocks
static BLOCK_PBFT_PREPREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("block_pbft_pre_prepared"));
static BLOCK_PBFT_PREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("block_pbft_prepared"));
static BLOCK_PBFT_COMMIT_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("block_pbft_commit"));

// Transaction mempool verifications PBFT engine
static TXN_PBFT_PREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("txn_pbft_prepared"));
static TXN_PBFT_COMMIT_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("txn_pbft_commit"));
static ACCOUNT_CREATION_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("account_creation"));

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
#[derive(Debug)]
pub enum AppEvent {
    Gossipsub(GossipsubEvent),

    // More specific Gossipsub message handling
    AccountCreation {
        propagation_source: PeerId,
        message_id: MessageId,
        data: Vec<u8>,
    },
    CreateBlocks {
        propagation_source: PeerId,
        message_id: MessageId,
        data: Vec<u8>,
    },
    TxnPbftPrepared {
        propagation_source: PeerId,
        message_id: MessageId,
        data: Vec<u8>,
    },
    TxnPbftCommit {
        propagation_source: PeerId,
        message_id: MessageId,
        data: Vec<u8>,
    },
    BlockPbftPrePrepared {
        propagation_source: PeerId,
        message_id: MessageId,
        data: Vec<u8>,
    },
    BlockPbftCommit {
        propagation_source: PeerId,
        message_id: MessageId,
        data: Vec<u8>,
    },
    PrivateBlocksGenesisCreation {
        propagation_source: PeerId,
        message_id: MessageId,
        data: Vec<u8>,
    },
    HybridBlockCreation {
        propagation_source: PeerId,
        message_id: MessageId,
        data: Vec<u8>,
    },
    Kademlia(KademliaEvent),
    Identify(IdentifyEvent)
    // Add variants for other event types as needed
    // For example: Kademlia(KademliaEvent),
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "AppEvent", event_process = false)] // event_process = false tells derive macro not to expect automatic event processing
pub struct AppBehaviour {
    pub gossipsub: Gossipsub,
    pub kademlia: Kademlia<MemoryStore>,
    pub identify: Identify,
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
        gossipsub:Gossipsub,
        app: App,
        txn:Txn,
        pbft:PBFTNode,
        storage_path:StoragePath,
        kademlia: Kademlia<MemoryStore>, // Include Kademlia here
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {

        let identify = Identify::new(
            IdentifyConfig::new("/sumotex/1.0.0".to_string(), KEYS.public()).with_agent_version("SUMOTEX v1.0".to_string())
        );
        let mut behaviour = Self {
            app,
            txn,
            pbft,
            storage_path,
            gossipsub,
            kademlia,
            identify,
            response_sender,
            init_sender,
        };
        behaviour
    }
    pub fn process_create_blocks(&mut self, serialized_data: &[u8]) -> Result<(), String> {
        match serde_json::from_slice::<Block>(serialized_data) {
            Ok(block) => {
                //info!("Received new block from {}", message.source.to_string());
                self.app.try_add_block(block.clone());
                let path = "./public_blockchain/db";
                // Open the database and handle the Result
                let block_db = match rock_storage::open_db(path) {
                    Ok(db) => db,
                    Err(e) => {
                        eprintln!("Failed to open database");
                        return Err(format!("Failed to write to database: {:?}", e));
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
                Ok(())
            },
            Err(err) => {
                error!(
                    "Error deserializing block from: {}",
                    err
                );
                Err("No data map found".to_string())
            }
        }
    
    }
    pub fn process_txn_pbft_prepared(&mut self, serialized_data: &[u8]) -> Result<(), String> {
        let received_serialized_data = serialized_data;
        match String::from_utf8(received_serialized_data.to_vec()) {
            Ok(json_string) => {
                match serde_json::from_str::<Value>(&json_string) {
                    Ok(outer_json) => {
                        if let Some(value) = outer_json["value"].as_str() {
                            let unescaped_value = value.replace("\\\"", "\"");
                            match serde_json::from_str::<PublicTxn>(&unescaped_value) {
                                Ok(txn) => {
                                    let serialized_txn_from_msg = serde_json::to_string(&txn)
                                        .map_err(|e| format!("Failed to serialize transaction: {}", e))?;
    
                                    let msg_hash_result = Sha256::digest(serialized_txn_from_msg.as_bytes());
                                    let msg_transaction_hash = hex::encode(msg_hash_result);
                                    let expected_hash = outer_json["key"].as_str().unwrap_or_default();
    
                                    if msg_transaction_hash == expected_hash {
                                        if let Some(publisher) = Publisher::get() {
                                            publisher.publish("txn_pbft_commit".to_string(), json_string);
                                        }
                                        Ok(())
                                    } else {
                                        println!("Hashes do not match.");
                                        Err("Hash mismatch".to_string())
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Failed to parse PublicTxn: {}", e);
                                    Err(format!("Failed to parse PublicTxn: {}", e))
                                }
                            }
                        } else {
                            println!("Key and/or Value not found in JSON.");
                            Err("Key and/or Value not found in JSON.".to_string())
                        }
                    },
                    Err(e) => {
                        println!("Failed to parse JSON: {}", e);
                        Err(format!("Failed to parse JSON: {}", e))
                    }
                }
            },
            Err(e) => {
                eprintln!("Failed to convert bytes to string: {}", e);
                Err(format!("Failed to convert bytes to string: {}", e))
            }
        }
    }
    pub fn process_txn_pbft_commit(&mut self, serialized_data: &[u8]) -> Result<(), String> {
        let received_serialized_data = serialized_data;
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
                                    println!("Transaction added to Mempool: {:?}", txn.clone().txn_hash.to_string());
                                    Ok(()) // Ensure this is the returned value from the function
                                },
                                Err(e) => {
                                    eprintln!("Failed to parse PublicTxn: {}", e);
                                    Err("Failed to parse PublicTxn".to_string())
                                }
                            }
                        } else {
                            println!("'value' field not found in JSON.");
                            Err("'value' field not found in JSON.".to_string())
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to parse Unescaped JSON: {}", e);
                        Err("Failed to parse Unescaped JSON".to_string())
                    }
                }
            },
            Err(e) => {
                eprintln!("Failed to convert bytes to string: {}", e);
                Err("Failed to convert bytes to string".to_string())
            }
        } // Removed semicolon here
    }
    
    pub fn process_block_pbft_pre_prepared(&mut self, serialized_data: &[u8]) -> Result<(), String> {
        let received_serialized_data = serialized_data;
        let deserialized_data:  HashMap<String, HashMap<String, HashMap<String,String>>> = serde_json::from_slice(&received_serialized_data).expect("Deserialization failed");
        let the_pbft_hash = self.pbft.get_hash_id();
        let mut all_transactions_valid = true;
        let mut txn_hashes_for_root = Vec::new();
        if let Some((first_key, inner_value)) = deserialized_data.iter().next() {
            unsafe {
                LEADER = Some(first_key.to_string());
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
                        publisher.publish_block("block_pbft_commit".to_string(),json.as_bytes().to_vec());
                        Ok(())
                        //}
                }else{
                    Err("No data map found".to_string())
                }
            } else {
                println!("Not all transactions are valid, Block creation process will not proceed.");
                Err("No data map found".to_string())
            }   
        } else {
            println!("The outer HashMap is empty");
            Err("No data map found".to_string())
        }
    }
    pub fn process_block_pbft_commit(&mut self, serialized_data: &[u8]) -> Result<(), String> {
        let _node_path = rock_storage::open_db("./node/db");
        let local_peer_id = get_peer_id();
        let _is_leader = unsafe { LEADER.as_ref() }.map(|leader| leader == &local_peer_id).unwrap_or(false);
        
        match serde_json::from_slice::<Block>(serialized_data) {
            Ok(block) => {
                if let Some(publisher) = Publisher::get() {
                    let mut mempool = txn_pool::Mempool::get_instance().lock().unwrap();
                    self.app.try_add_block(block.clone());
                    let path = "./public_blockchain/db";
                    
                    let block_db = match rock_storage::open_db(path) {
                        Ok(db) => db,
                        Err(e) => {
                            eprintln!("Failed to open database");
                            return Err(format!("Failed to open database: {:?}", e));
                        }
                    };
    
                    let json = match serde_json::to_string(&block) {
                        Ok(json) => json,
                        Err(e) => return Err(format!("Failed to serialize block: {:?}", e)),
                    };
    
                    if let Err(e) = rock_storage::put_to_db(&block_db, block.public_hash.clone(), &json) {
                        return Err(format!("Failed to write block to database: {}", e));
                    }
    
                    if let Err(e) = rock_storage::put_to_db(&block_db, "latest_block", &json) {
                        return Err(format!("Failed to update latest block in database: {}", e));
                    }
    
                    let mut total_gas_cost: u128 = 0;
                    for txn_hash in &block.transactions {
                        if let Some(first_txn_id) = txn_hash.first().cloned() {
                            match Txn::get_transaction_by_id(&first_txn_id) {
                                Ok(txn_detail) => {
                                    total_gas_cost += txn_detail.gas_cost;
                                    Txn::update_transaction_status(&first_txn_id, 3);
                                    mempool.remove_transaction_by_id(first_txn_id);
                                },
                                Err(e) => return Err(format!("Failed to retrieve transaction details: {}", e)),
                            }
                        } 
                    }
    
                    publisher.publish_block("create_blocks".to_string(), json.as_bytes().to_vec());
                    Ok(())
                } else {
                    Err("Publisher not found".to_string())
                }
            },
            Err(e) => Err(format!("Error deserializing block: {}", e)),
        }
    }
    
    pub fn process_private_blocks_genesis_creation(&mut self, serialized_data: &[u8]) -> Result<(), String> {
        let json_string = match String::from_utf8(serialized_data.to_vec()) {
            Ok(string) => string,
            Err(e) => return Err(format!("Failed to convert bytes to string: {}", e))
        };
    
        if let Some(publisher) = Publisher::get() {
            let created_block = public_block::handle_create_block_private_chain(self.app.clone(), Some(json_string), None, None, None);
            let json = match serde_json::to_string(&created_block) {
                Ok(json) => json,
                Err(e) => return Err(format!("Failed to serialize block: {}", e))
            };
    
            let path = "./public_blockchain/db";
            match rock_storage::open_db(path) {
                Ok(db) => {
                    if let Err(e) = rock_storage::put_to_db(&db, created_block.public_hash.clone(), &json) {
                        return Err(format!("Failed to write to database: {}", e));
                    }
                    if let Err(e) = rock_storage::put_to_db(&db, "latest_block", &json) {
                        return Err(format!("Failed to update latest block: {}", e));
                    }
                },
                Err(e) => return Err(format!("Failed to open database: {}", e))
            }
            publisher.publish_block("create_blocks".to_string(), json.as_bytes().to_vec());
            Ok(())
        } else {
            Err("Publisher not found".to_string())
        }
    }
    pub fn process_hybrid_block_creation(&mut self, serialized_data: &[u8]) -> Result<(), String> {
        let json_string = match String::from_utf8(serialized_data.to_vec()) {
            Ok(json) => json,
            Err(e) => return Err(format!("Failed to decode UTF-8 data: {}", e)),
        };
    
        if let Some(publisher) = Publisher::get() {
            let created_block = public_block::handle_create_block_private_chain(self.app.clone(), Some(json_string), None, None, None);
            let json = match serde_json::to_string(&created_block) {
                Ok(j) => j,
                Err(e) => return Err(format!("Failed to serialize data: {}", e)),
            };
    
            let path = "./public_blockchain/db";
            let block_db = match rock_storage::open_db(path) {
                Ok(db) => db,
                Err(e) => return Err(format!("Failed to open database: {}", e)),
            };
    
            if let Err(e) = rock_storage::put_to_db(&block_db, created_block.public_hash.clone(), &json) {
                return Err(format!("Failed to write block to database: {}", e));
            }
            self.app.blocks.push(created_block);
            publisher.publish_block("create_blocks".to_string(), json.as_bytes().to_vec());
            Ok(())
        } else {
            Err("Publisher not found".to_string())
        }
    }
    
    pub fn process_account_creation_event(&mut self,serialized_data: &[u8]) -> Result<(), String> {
        match serde_json::from_slice::<account::Account>(serialized_data) {
            Ok(acc) => {
                let path = "./account/db";
                println!("Account creation called");
                match rock_storage::open_db(path) {
                    Ok(db) => {
                        match serde_json::to_string(&acc) {
                            Ok(serialized_data) => {
                                if let Err(e) = rock_storage::put_to_db(&db, &acc.public_address, &serialized_data) {
                                    eprintln!("Failed to write to database: {:?}", e);
                                    return Err(format!("Failed to write to database: {:?}", e));
                                }
                            },
                            Err(e) => {
                                eprintln!("Failed to serialize account data: {:?}", e);
                                return Err(format!("Failed to serialize account data: {:?}", e));
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to open database: {:?}", e);
                        return Err(format!("Failed to open database: {:?}", e));
                    }
                }
            },
            Err(e) => {
                error!("Error deserializing account data: {:?}", e);
                return Err(format!("Error deserializing account data: {:?}", e));
            }
        }
        Ok(())
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

impl NetworkBehaviourEventProcess<AppEvent> for AppBehaviour {
    fn inject_event(&mut self, event: AppEvent) {
        println!("Processing event: {:?}", event);
        match event {
            AppEvent::AccountCreation { propagation_source, message_id, data } => {
                // Process Account Creation
                println!("Received account creation message from {}", propagation_source);
                self.process_account_creation_event(&data);
            },
            AppEvent::CreateBlocks { propagation_source, message_id, data } => {
                // Process Create Blocks
                println!("Received create blocks message from {}", propagation_source);
                self.process_create_blocks(&data);
            },
            AppEvent::TxnPbftPrepared { propagation_source, message_id, data } => {
                // Process Transaction PBFT Prepared
                println!("Received txn PBFT prepared message from {}", propagation_source);
                self.process_txn_pbft_prepared(&data);
            },
            AppEvent::TxnPbftCommit { propagation_source, message_id, data } => {
                // Process Transaction PBFT Commit
                println!("Received txn PBFT commit message from {}", propagation_source);
                self.process_txn_pbft_commit(&data);
            },
            AppEvent::BlockPbftPrePrepared { propagation_source, message_id, data } => {
                // Process Block PBFT Pre-Prepared
                println!("Received block PBFT pre-prepared message from {}", propagation_source);
                self.process_block_pbft_pre_prepared(&data);
            },
            AppEvent::BlockPbftCommit { propagation_source, message_id, data } => {
                // Process Block PBFT Commit
                println!("Received block PBFT commit message from {}", propagation_source);
                self.process_block_pbft_commit(&data);
            },
            AppEvent::PrivateBlocksGenesisCreation { propagation_source, message_id, data } => {
                // Process Private Blocks Genesis Creation
                println!("Received private blocks genesis creation message from {}", propagation_source);
                self.process_private_blocks_genesis_creation(&data);
            },
            AppEvent::HybridBlockCreation { propagation_source, message_id, data } => {
                // Process Hybrid Block Creation
                println!("Received hybrid block creation message from {}", propagation_source);
                self.process_hybrid_block_creation(&data);
            },
            AppEvent::Gossipsub(event) => {
                // Handle other generic Gossipsub events not covered by specific cases
                println!("Received general Gossipsub event");
            },
            AppEvent::Kademlia(event) => {
                // Process Kademlia events
                println!("Received Kademlia event");
            },
            AppEvent::Identify(event) => {
                // Process Identify events
                println!("Received Identify event");
            },
            AppEvent::Gossipsub(_) => todo!()
        }
    }
}

impl From<GossipsubEvent> for AppEvent {
    fn from(event: GossipsubEvent) -> Self {
        match event {
            GossipsubEvent::Message { propagation_source, ref message_id, ref message } => {
                println!("Message ID: {:?}",message_id);
                println!("Message Data: {:?}",message.data);
                match message.topic.as_str() {
                    "account_creation" => AppEvent::AccountCreation {
                        propagation_source,
                        message_id: message_id.clone(),
                        data: message.data.clone(),
                    },
                    "create_blocks" => AppEvent::CreateBlocks {
                        propagation_source,
                        message_id: message_id.clone(),
                        data: message.data.clone(),
                    },
                    "txn_pbft_prepared" => AppEvent::TxnPbftPrepared {
                        propagation_source,
                        message_id: message_id.clone(),
                        data: message.data.clone(),
                    },
                    "txn_pbft_commit" => AppEvent::TxnPbftCommit {
                        propagation_source,
                        message_id: message_id.clone(),
                        data: message.data.clone(),
                    },
                    "block_pbft_pre_prepared" => AppEvent::BlockPbftPrePrepared {
                        propagation_source,
                        message_id: message_id.clone(),
                        data: message.data.clone(),
                    },
                    "block_pbft_commit" => AppEvent::BlockPbftCommit {
                        propagation_source,
                        message_id: message_id.clone(),
                        data: message.data.clone(),
                    },
                    "private_blocks_genesis_creation" => AppEvent::PrivateBlocksGenesisCreation {
                        propagation_source,
                        message_id: message_id.clone(),
                        data: message.data.clone(),
                    },
                    "hybrid_block_creation" => AppEvent::HybridBlockCreation {
                        propagation_source,
                        message_id: message_id.clone(),
                        data: message.data.clone(),
                    },
                    _ => AppEvent::Gossipsub(event), // Fallback for other or unknown topics
                }
            },
            _ => AppEvent::Gossipsub(event), // Default for other kinds of Gossipsub events
        }
    }
}

pub fn trigger_publish(sender: mpsc::UnboundedSender<(String, String)>, title: String, message: String) {
    sender.send((title, message)).expect("Can send publish event");
}

impl From<KademliaEvent> for AppEvent {
    fn from(event: KademliaEvent) -> Self {
        AppEvent::Kademlia(event)
    }
}
impl From<IdentifyEvent> for AppEvent {
    fn from(event: IdentifyEvent) -> Self {
        AppEvent::Identify(event)
    }
}