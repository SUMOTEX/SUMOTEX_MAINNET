use sha2::{Sha256, Digest};
use super::{App,Txn,PublicTxn, PBFTNode,Block};
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
use crate::verkle_tree::VerkleTree;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
// main.rs
use crate::Publisher;

pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
pub static TXN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("transactions"));
pub static PBFT_PREPREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("pbft_pre_prepared"));
pub static PBFT_PREPARED_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("pbft_prepared"));
pub static PBFT_COMMIT_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("pbft_commit"));


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
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(TXN_TOPIC.clone());
        behaviour.floodsub.subscribe(PBFT_PREPREPARED_TOPIC.clone());
        behaviour.floodsub.subscribe(PBFT_PREPARED_TOPIC.clone());
        behaviour.floodsub.subscribe(PBFT_COMMIT_TOPIC.clone());
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
            } else if msg.topics[0]==Topic::new("blocks"){
                if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {
                    info!("received new block from {}", msg.source.to_string());
                    self.app.try_add_block(block);
                }
            } else if msg.topics[0]==Topic::new("pbft_pre_prepared") {
                let received_serialized_data =msg.data;
                let deserialized_data: HashMap<String, HashMap<String, String>> = serde_json::from_slice(&received_serialized_data).expect("Deserialization failed");
                let the_pbft_hash = self.pbft.get_hash_id();
                println!("The NODE: {:?}",the_pbft_hash);
                info!("The new txn from {:?}", deserialized_data);
                for (key, inner_map) in deserialized_data.iter() {
                    //TODO ADD the hash to database.
                    
                    println!("The Mapping Inner Txn: {:?}",inner_map);
                    let (valid_txn,txn_hashes) = self.txn.is_txn_valid(key.to_string(),inner_map.clone());
                    if valid_txn {
                        let created_block=self.pbft.pre_prepare(key.to_string(),txn_hashes.clone());
                        if let Some(publisher) = Publisher::get(){
                            publisher.publish("pbft_prepared".to_string(), the_pbft_hash.to_string());
                            self.pbft.prepare("PBFT Valid and prepare".to_string());
                        }
                    }
                }
            }
            else if msg.topics[0]==Topic::new("pbft_prepared"){
                let received_serialized_data =msg.data;
                let json_string = String::from_utf8(received_serialized_data).unwrap();
                info!("RECEIVED PBFT PREPARED: {:?}",json_string);
                if let Some(publisher) = Publisher::get(){
                    publisher.publish("pbft_commit".to_string(), json_string.to_string());
                }

            }else if msg.topics[0]==Topic::new("pbft_commit"){
                let received_serialized_data =msg.data;
                let json_string = String::from_utf8(received_serialized_data).unwrap();
                self.pbft.commit("COMMIT READY".to_string());
                if let Some(publisher) = Publisher::get(){
                    let (root,txn) = self.pbft.get_txn(json_string);
                    let created_block=handle_create_block_pbft(self.app.clone(),root,txn);
                    println!("The Created Block After Validity: {:?}",created_block);
                    let json = serde_json::to_string(&created_block).expect("can jsonify request");
                    self.app.blocks.push(created_block);
                    publisher.publish("blocks".to_string(),json)
                }
            }
            else if msg.topics[0]==Topic::new("transactions")  {
                // let received_serialized_data:HashMap<String, Vec<String>> =serde_json::from_slice(&msg.data).expect("Deserialization failed");
                // println!("Source: {:?}",received_serialized_data);
                //self.txn.try_add_hash_txn(received_serialized_data);
                // let validity = self.txn.is_txn_valid();
                // if validity{
                //     println!("hello, txn valid")
                // }
                // info!("SerializeTxn: {:?}", deserialized_data["key"]);
                // if(self.txn.is_txn_valid(&deserialized_data["key"],&deserialized_data["value"])){
                //     println!("Valid");
                // };
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
pub fn get_total_pbft_view(swarm: &Swarm<AppBehaviour>)->u64 {
    let view_value = swarm.behaviour().pbft.view;
    view_value
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

pub fn handle_create_block(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("create b") {
        let behaviour = swarm.behaviour_mut();
        let latest_block = behaviour
            .app
            .blocks
            .last()
            .expect("there is at least one block");
        let block = Block::new(
            latest_block.id + 1,
            latest_block.public_hash.clone(),
            //TODO txn
            ["TEST BLOCK CREATION WITH TXN".to_string()].to_vec()
        );
        let json = serde_json::to_string(&block).expect("can jsonify request");
        behaviour.app.blocks.push(block);
        info!("broadcasting new block");
        behaviour
            .floodsub
            .publish(BLOCK_TOPIC.clone(), json.as_bytes());
    }
}
pub fn handle_finalised_block(swarm: &mut Swarm<AppBehaviour>, block:Block){
    let behaviour = swarm.behaviour_mut();
    let json = serde_json::to_string(&block).expect("can jsonify request");
    behaviour.app.blocks.push(block);
    info!("broadcasting new block");
    behaviour
        .floodsub
        .publish(BLOCK_TOPIC.clone(), json.as_bytes());
}
pub fn handle_create_block_pbft(app:App,root_hash:String,txn:Vec<String>)-> Block{
    let app = app.blocks.last().expect("There should be at least one block");
    let latest_block = app;
    let block = Block::new(
        latest_block.id +1,
        latest_block.public_hash.clone(),
        txn
    );
    let json = serde_json::to_string(&block).expect("can jsonify request");
    block
}

pub fn pbft_pre_message_handler(cmd:&str,swarm:  &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("create txn") {
        let behaviour =swarm.behaviour_mut();
        let mut i: i64 =0;
        let mut verkle_tree = VerkleTree::new();
        let mut transactions: HashMap<String, String>= HashMap::new();
        while i<5 {
            let r = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .collect::<Vec<_>>();
            let s = String::from_utf8_lossy(&r);
            let current_timestamp: i64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            let mut latest_txn = PublicTxn{
                txn_hash:s.to_string(),
                nonce:i,
                value:data.to_owned(),
                status:1,
                timestamp: current_timestamp
            };
            let serialized_data = serde_json::to_string(&latest_txn).expect("can jsonify request");
            // Hash the serialized data
            let mut hasher = Sha256::new();
            hasher.update(&serialized_data);
            let hash_result = hasher.finalize();
             // Convert the hash bytes to a hexadecimal string
            let hash_hex_string = format!("{:x}", hash_result);
            i = i+1;
            verkle_tree.insert(s.as_bytes().to_vec(), hash_result.to_vec());
            let mut dictionary_data = std::collections::HashMap::new();
            dictionary_data.insert("key".to_string(), s.to_string());
            dictionary_data.insert("value".to_string(), serialized_data.to_string());
            // Serialize the dictionary data (using a suitable serialization format)
            let serialised_txn = serde_json::to_vec(&dictionary_data).unwrap();
            transactions.insert(s.to_string(),serialized_data.to_string());
            //behaviour.floodsub.publish(TXN_TOPIC.clone(),s.to_string());
        }
        let root_hash = verkle_tree.get_root_string();
        let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
        map.insert(root_hash.clone(),transactions);
        let serialised_dictionary = serde_json::to_vec(&map).unwrap();
        info!("Broadcasting Transactions to nodes");
        //behaviour.txn.transactions.push(root_hash.clone());
        behaviour
            .floodsub
            .publish(PBFT_PREPREPARED_TOPIC.clone(), serialised_dictionary);
    }

}
