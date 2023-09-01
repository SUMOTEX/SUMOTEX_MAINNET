use libp2p::{
    floodsub::{Floodsub,FloodsubEvent,Topic},
    core::{identity},
    mdns::{Mdns,MdnsEvent},
     PeerId,
    swarm::{Swarm,NetworkBehaviourEventProcess},
};
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{
   Kademlia, KademliaEvent,
     QueryResult
};
use libp2p::Multiaddr;
use tokio::{
    sync::mpsc,
};
use std::io;
use libp2p::NetworkBehaviour;
use std::collections::HashMap;
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use crate::private_transactions::Txn;
use crate::private_block;
use crate::private_pbft::PRIVATE_PBFT_PREPREPARED_TOPIC;
use crate::private_block::handle_create_block_pbft;
use crate::private_app::PrivateApp;
use crate::pbft::PBFTNode;
use crate::private_block::PrivateBlock;
use crate::account_root::AccountRoot;
// main.rs
use crate::publisher::Publisher;
pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static GENESIS_BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("genesis_private_block"));
pub static PRIVATE_TXN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("private_transactions"));
pub static PRIVATE_PBFT_PREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("private_pbft_prepared"));
pub static PRIVATE_PBFT_COMMIT_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("private_pbft_commit"));


#[derive(Debug, Serialize, Deserialize)]
pub struct PrivateChainResponse {
    pub blocks: Vec<PrivateBlock>,
    pub receiver: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrivateLocalChainRequest {
    pub from_peer_id: String,
}

pub enum EventType {
    LocalChainResponse(PrivateChainResponse),
    Input(String),
    Init,
    Publish(String, String), // Publish a message to a topic
    PublishBlock(String,Vec<u8>),
    Kademlia(KademliaEvent),
}

#[derive(NetworkBehaviour)]
pub struct PrivateAppBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<PrivateChainResponse>,
    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub private_tx: mpsc::UnboundedSender<String>,
    #[behaviour(ignore)]
    pub app: PrivateApp,
    #[behaviour(ignore)]
    pub txn: Txn,
    #[behaviour(ignore)]
    pub pbft: PBFTNode,
    #[behaviour(ignore)]
    pub account_r: AccountRoot,
    pub kademlia: Kademlia<MemoryStore>,
}

impl PrivateAppBehaviour {
    // Create an Identify service
    pub async fn new(
        app: PrivateApp,
        txn:Txn,
        pbft:PBFTNode,
        account_r:AccountRoot,
        kademlia: Kademlia<MemoryStore>,
        response_sender: mpsc::UnboundedSender<PrivateChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
        private_tx: mpsc::UnboundedSender<String>,
        
    ) -> Self {
        let mut behaviour = Self {
            app,
            txn,
            pbft,
            account_r,
            kademlia,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns"),
            response_sender,
            init_sender,
            private_tx
        };
        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(private_block::PRIVATE_BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(GENESIS_BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(PRIVATE_TXN_TOPIC.clone());
        behaviour.floodsub.subscribe(PRIVATE_PBFT_PREPREPARED_TOPIC.clone());
        behaviour.floodsub.subscribe(PRIVATE_PBFT_PREPARED_TOPIC.clone());
        behaviour.floodsub.subscribe(PRIVATE_PBFT_COMMIT_TOPIC.clone());
        behaviour
    }
    fn send_message(&mut self, peer_id: PeerId, message: String) {
        // Here, you'll need to integrate with your actual protocol implementation to send the message.
        // Depending on how you implemented your protocol, this might involve
        // sending the message directly, or queuing it to be sent when polled.
    }

}

impl NetworkBehaviourEventProcess<KademliaEvent> for PrivateAppBehaviour {
    fn inject_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::RoutingUpdated { peer, is_new_peer, addresses, bucket_range, old_peer } => {
                // Handle the event here
                println!("Routing table updated: Added/Updated peer {}", peer);
                if is_new_peer {
                    println!("This is a new peer");
                }
                println!("Peer Addresses: {:?}", addresses);
                println!("Bucket Range: {:?}", bucket_range);
                if let Some(old_peer) = old_peer {
                    println!("Old Peer: {}", old_peer);
                }
            }
            KademliaEvent::OutboundQueryCompleted { id, result, .. } => {
                match result {
                    QueryResult::GetRecord(result) => {
                        match result {
                            Ok(ok_result) => {
                                // Successfully got records from the DHT.
                                for record in ok_result.records {
                                    println!("Successfully retrieved a record: {:?}", record);
                                    // You can process the record further here.
                                }
                            },
                            Err(err) => {
                                println!("Failed to retrieve record: {:?}", err);
                                // You can take steps to handle or recover from the error here.
                            }
                        }
                    },
                    QueryResult::PutRecord(result) => {
                        if let Err(err) = result {
                            println!("Failed to put record: {:?}", err);
                            // Handle or recover from the error.
                        } else {
                            println!("Successfully put the record.");
                        }
                    },
                    QueryResult::GetClosestPeers(result) => {
                        match result {
                            Ok(ok_result) => {
                                println!("Any Peers {:?}",ok_result);
                                for peer_id in ok_result.peers {
                                    println!("Closest Peers {:?}",peer_id);
                                    // Initiate a connection to each of the closest peers
                                    connect_to_peer(peer_id);
                                }
                            },
                            Err(err) => {
                                eprintln!("Error when trying to get closest peers: {:?}", err);
                            }
                        }
                    },
                    QueryResult::Bootstrap(result) => {
                        if let Err(err) = result {
                            println!("Failed to bootstrap: {:?}", err);
                        } else {
                            println!("Successfully bootstrapped.");
                        }
                    },
                    // Handle other query results here
                    _ => {}
                }
            },
            // You can handle other types of KademliaEvents here.
            _ => {
                println!("Received an unhandled Kademlia event: {:?}", event);
            }
        }
    }
}
pub fn connect_to_peer(peer_id: PeerId) {
    // Create a Multiaddr from the Peer ID
    let multiaddr: Multiaddr = format!("/p2p/{}", peer_id.to_base58()).parse().expect("Invalid multiaddr");
    
    // Dial the peer
    // if Swarm::dial(&mut swarm, &peer_id).is_err() {
    //     println!("Failed to connect to {:?}", peer_id);
    // } else {
    //     println!("Successfully initiated connection to {:?}", peer_id);
    // }
}

// incoming event handler
impl NetworkBehaviourEventProcess<FloodsubEvent> for PrivateAppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            // if msg.topics[0]
            info!("Response from {:?}:", msg);
            if let Ok(resp) = serde_json::from_slice::<PrivateChainResponse>(&msg.data) {
                if resp.receiver == PEER_ID.to_string() {
                    info!("Response from {}:", msg.source);
                    resp.blocks.iter().for_each(|r| info!("{:?}", r));
                    self.app.blocks = self.app.choose_chain(self.app.blocks.clone(), resp.blocks);
                }
            } else if let Ok(resp) = serde_json::from_slice::<PrivateLocalChainRequest>(&msg.data) {
                info!("sending local chain to {}", msg.source.to_string());
                let peer_id = resp.from_peer_id;
                if PEER_ID.to_string() == peer_id {
                    if let Err(e) = self.response_sender.send(PrivateChainResponse {
                        blocks: self.app.blocks.clone(),
                        receiver: msg.source.to_string(),
                    }) {
                        error!("error sending response via channel, {}", e);
                    }
                }
            } else if msg.topics[0]==Topic::new("genesis_private_block"){
                match serde_json::from_slice::<PrivateBlock>(&msg.data) {
                    Ok(block) => {
                        info!("Received new GENESIS block from {}", msg.source.to_string());
                        self.app.try_add_genesis()
                    },
                    Err(err) => {
                        error!(
                            "Error deserializing GENESIS block from {}: {}",
                            msg.source.to_string(),
                            err
                        );
                    }
                }
            }
            else if msg.topics[0]==Topic::new("private_blocks"){
                println!("add_private_block");
                match serde_json::from_slice::<PrivateBlock>(&msg.data) {
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
            } else if msg.topics[0]==Topic::new("private_pbft_pre_prepared") {
                let received_serialized_data =msg.data;
                let deserialized_data: HashMap<String, HashMap<String, String>> = serde_json::from_slice(&received_serialized_data).expect("Deserialization failed");
                let the_pbft_hash = self.pbft.get_hash_id();
                println!("Preparing PBFT");
                info!("New private transaction from {:?}", deserialized_data);
                for (key, inner_map) in deserialized_data.iter() {
                    //TODO ADD the hash to database.
                    let (valid_txn,txn_hashes) = self.txn.is_txn_valid(key.to_string(),inner_map.clone());
                    if valid_txn {
                        let created_block=self.pbft.pre_prepare(key.to_string(),txn_hashes.clone());
                        if let Some(publisher) = Publisher::get(){
                            publisher.publish("private_pbft_prepared".to_string(), the_pbft_hash.to_string());
                            self.pbft.prepare("PBFT Valid and prepare".to_string());
                        }
                    }                                                                                                               
                }
            }
            else if msg.topics[0]==Topic::new("private_pbft_prepared"){
                let received_serialized_data =msg.data;
                let json_string = String::from_utf8(received_serialized_data).unwrap();
                info!("RECEIVED PBFT PREPARED: {:?}",json_string);
                if let Some(publisher) = Publisher::get(){
                    publisher.publish("private_pbft_commit".to_string(), json_string.to_string());
                }

            }else if msg.topics[0]==Topic::new("private_pbft_commit"){
                let received_serialized_data =msg.data;
                let json_string = String::from_utf8(received_serialized_data).unwrap();
                self.pbft.commit("COMMIT READY".to_string());
                println!("COMMIT_PBFT");
                if let Some(publisher) = Publisher::get(){
                    let (root,txn) = self.pbft.get_txn(json_string);
                    let created_block=handle_create_block_pbft(self.app.clone(),root,txn);
                    println!("The Created Block After Validity: {:?}",created_block);
                    let json = serde_json::to_string(&created_block).expect("can jsonify request");
                    self.app.blocks.push(created_block);
                    println!("Private Blocks {:?}",self.app.blocks);
       
                    // In the private swarm event loop or somewhere in your `create_private_swarm` function
                    // assuming private_tx is passed as an argument or captured from the environment
                    println!("Publish Block");
                    self.private_tx.send("add_private_block".to_string()).unwrap();
                    // match self.private_tx.send("add_private_block".to_string()) {
                    //     Ok(_) => println!("Successfully sent message"),
                    //     Err(e) => eprintln!("Failed to send message: {}", e),
                    // }
                    publisher.publish_block("private_blocks".to_string(),json.as_bytes().to_vec())
                }
            }
            else if msg.topics[0]==Topic::new("private_transactions")  {
            }
        }
    }
}
impl NetworkBehaviourEventProcess<MdnsEvent> for PrivateAppBehaviour {
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
    sender.send((title, message)).expect("Can send publish private event");
}
pub fn get_list_peers(swarm: &Swarm<PrivateAppBehaviour>) -> Vec<String> {
    info!("Private Validators:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn handle_print_private_peers(swarm: &Swarm<PrivateAppBehaviour>) {
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn handle_print_chain(swarm: &Swarm<PrivateAppBehaviour>) {
    info!("SUMOTEX Private Blockchain:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.blocks).expect("can jsonify blocks");
    info!("{}", pretty_json);
}
pub fn handle_print_private_txn(swarm: &Swarm<PrivateAppBehaviour>) {
    info!("Private Transactions:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().txn.transactions).expect("can jsonify transactions");
    info!("{}", pretty_json);
}
pub fn handle_print_raw_txn(swarm: &Swarm<PrivateAppBehaviour>) {
    info!("Private Raw Transactions:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().txn.hashed_txn).expect("can jsonify transactions");
    info!("{}", pretty_json);
}

pub fn handle_print_peers(swarm: &Swarm<PrivateAppBehaviour>) {
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}
pub fn handle_response(data: Vec<u8>) {
    // Convert the data into a String
    if let Ok(response_str) = String::from_utf8(data) {
        // Print or process the response string
        println!("Received response: {}", response_str);
        
        // Based on the response string, you might perform other actions
        // For instance, you could:
        // - Update some data structures
        // - Trigger other network events or requests
        // - Notify the user or other parts of your system
        // ... etc.
        
    } else {
        eprintln!("Failed to decode the response data into a string.");
    }
}
pub fn handle_start_chain(swarm: &mut Swarm<PrivateAppBehaviour>){
    let mut steps = 0;
    let mut chain_name = String::new();
    let mut start_block = String::new();
    loop{
        if steps==0{
            println!("Enter your Chain name: ");
            io::stdin().read_line(&mut chain_name).expect("Failed to read line");
        }else if steps==1 {
            println!("Start your Genesis Block (Y/N): ");
            io::stdin().read_line(&mut start_block).expect("Failed to read line");
            println!("{}",start_block);
            if start_block.trim()=="y".to_string(){
                println!("Starting...");
                println!("Setting up Genesis Block for Chain: {}",chain_name);
                println!("Setup completed...start using today!");
                swarm.behaviour_mut().app.genesis();
                let the_genesis_block =swarm.behaviour_mut().app.blocks.last().expect("there is at least one block");
                match serde_json::to_vec::<PrivateBlock>(the_genesis_block) {
                    Ok(block) => {
                        info!("Block: {:?}", block);
                        info!("Generating Genesis block for chain: {}", chain_name);
                        swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(GENESIS_BLOCK_TOPIC.clone(), block);
                    },
                    Err(err) => {
                        error!(
                            "Error deserializing Genesis Block, {}",
                            err
                        );
                    }
                }

                break;
            }else {
                println!("Looks like you have decided to not continue with the setup, try again later!");
                break;
            }
        }
      
        if chain_name.is_empty() {
            println!("You entered an empty string!");
           
        } else {
            println!("Chain name: {}", chain_name);
            let peers_list = get_list_peers(swarm);
            println!("Checking on number of Private Validator... looks like you have {}",peers_list.len());
            if  peers_list.len()>=2 {
                println!("Looks like you have sufficient private validator");
                steps+=1;
            }else{
                println!("Looks like you have insufficient private validator, setup at least 2 of them and try again!");
                break;
            }


        }

    }

}