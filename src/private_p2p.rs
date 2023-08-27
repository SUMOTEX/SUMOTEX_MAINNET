use sha2::{Sha256};
use super::{App,Txn,PublicTxn, pbft::PBFTNode,public_block::Block};
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
use std::collections::HashMap;
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::private_block;
use crate::private_pbft;
use crate::private_block::handle_create_block_pbft;

// main.rs
use crate::Publisher;
pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static PRIVATE_TXN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("private_transactions"));
pub static PRIVATE_PBFT_PREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("private_pbft_prepared"));
pub static PRIVATE_PBFT_COMMIT_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("private_pbft_commit"));


#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub receiver: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalChainRequest {
    pub from_peer_id: String,
}

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
}

impl AppBehaviour {
    // Create an Identify service
    pub async fn new(
        app: App,
        txn:Txn,
        pbft:PBFTNode,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            app,
            txn,
            pbft,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns"),
            response_sender,
            init_sender,
        };
        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(private_block::BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(TXN_TOPIC.clone());
        behaviour.floodsub.subscribe(private_pbft::PBFT_PREPREPARED_TOPIC.clone());
        behaviour.floodsub.subscribe(PRIVATE_PBFT_PREPARED_TOPIC.clone());
        behaviour.floodsub.subscribe(PRIVATE_PBFT_COMMIT_TOPIC.clone());
        behaviour
    }
}
// incoming event handler
impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            // if msg.topics[0]
            info!("Response from {:?}:", msg);
            if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                if resp.receiver == PEER_ID.to_string() {
                    info!("Response from {}:", msg.source);
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
            } else if msg.topics[0]==Topic::new("private_blocks"){
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
            } else if msg.topics[0]==Topic::new("private_pbft_pre_prepared") {
                let received_serialized_data =msg.data;
                let deserialized_data: HashMap<String, HashMap<String, String>> = serde_json::from_slice(&received_serialized_data).expect("Deserialization failed");
                let the_pbft_hash = self.pbft.get_hash_id();
                println!("The NODE: {:?}",the_pbft_hash);
                info!("The new txn from {:?}", deserialized_data);
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
                if let Some(publisher) = Publisher::get(){
                    let (root,txn) = self.pbft.get_txn(json_string);
                    let created_block=handle_create_block_pbft(self.app.clone(),root,txn);
                    println!("The Created Block After Validity: {:?}",created_block);
                    let json = serde_json::to_string(&created_block).expect("can jsonify request");
                    self.app.blocks.push(created_block);
                    println!("BLOCKS {:?}",self.app.blocks);
                    publisher.publish_block("private_blocks".to_string(),json.as_bytes().to_vec())
                }
            }
            else if msg.topics[0]==Topic::new("private_transactions")  {
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
    sender.send((title, message)).expect("Can send publish private event");
}
pub fn get_list_peers(swarm: &Swarm<AppBehaviour>) -> Vec<String> {
    info!("Private Validators:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn handle_print_private_peers(swarm: &Swarm<AppBehaviour>) {
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn handle_print_chain(swarm: &Swarm<AppBehaviour>) {
    info!("SUMOTEX Private Blockchain:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.blocks).expect("can jsonify blocks");
    info!("{}", pretty_json);
}
pub fn handle_print_private_txn(swarm: &Swarm<AppBehaviour>) {
    info!("Private Transactions:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().txn.transactions).expect("can jsonify transactions");
    info!("{}", pretty_json);
}
pub fn handle_print_raw_txn(swarm: &Swarm<AppBehaviour>) {
    info!("Private Raw Transactions:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().txn.hashed_txn).expect("can jsonify transactions");
    info!("{}", pretty_json);
}


