use chrono::prelude::*;
use libp2p::{
    core::upgrade,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm},
    tcp::TokioTcpConfig,
    Transport,
};
use crate::verkle_tree::VerkleTree;
use log::{error, info};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};
use std::collections::HashMap;
use std::collections::BTreeMap;
use libp2p::futures::StreamExt;
use rand::Rng;
mod verkle_tree;
mod p2p;
mod swarm;
mod publisher;
mod public_block;
mod pbft;
use publisher::Publisher;



pub struct Txn{
    pub transactions: Vec<String>,
    pub hashed_txn:Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicTxn{
    pub txn_hash: String,
    pub nonce:i64,
    // pub version:String,
    pub value: String,
    // pub gas_limit: u64,
    // pub caller_address:u64,
    // pub to_address:u64,
    // pub sig:u64,
    pub status:i64,
    pub timestamp:i64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializeTxn{
    pub txn_hash: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RootTxn{
    pub root_txn_hash: String,
}
#[derive(Debug,Clone)]
pub struct App {
    pub blocks: Vec<public_block::Block>,
}






fn hash_to_binary_representation(hash: &[u8]) -> String {
    let mut res: String = String::default();
    for c in hash {
        res.push_str(&format!("{:b}", c));
    }
    res
}
impl Txn {
    fn new() -> Self {
        Self { transactions: vec![],hashed_txn: vec![] }
    }
    fn try_add_root_txn(&mut self, txn: String) {
        self.transactions.push(txn);
    }
    fn try_add_hash_txn(&mut self, txn: String) {
        //self.hashed_txn.push(txn);
    }
    fn is_txn_valid(&mut self,root_hash:String, txn_hash: HashMap<String, String>) -> (bool,Vec<String>) {
            //println!("{:?}",theTxn.timestamp);
            //TODO: To do verification on the transactions and store in another place.
            let mut verkle_tree = VerkleTree::new();
            let mut array_of_txn:Vec<String>=Vec::new();
            let mut hashed_root= hex::decode(&root_hash).expect("Failed to decode hex");
            let hash_array: [u8; 32] = hashed_root.try_into().expect("Slice has incorrect length");
            let mut sorted_items = BTreeMap::new();
            for (inner_key, inner_value) in txn_hash.iter() {
                let deserialized_data:PublicTxn = serde_json::from_str(&inner_value).expect("Deserialization failed");
                sorted_items.insert(deserialized_data.nonce, deserialized_data);
            }
            for (_, item) in sorted_items.iter() {
                println!("{:#?}", item.txn_hash);
                let serialized_data = serde_json::to_string(&item).expect("can jsonify request");
                // Hash the serialized data
                let mut hasher = Sha256::new();
                hasher.update(&serialized_data);
                let hash_result = hasher.finalize();
                array_of_txn.push(item.txn_hash.to_string());
                verkle_tree.insert(item.txn_hash.as_bytes().to_vec(), hash_result.to_vec());
            }
            let the_root = verkle_tree.get_root_string();
            if root_hash==the_root{
                //let mut swarm_guard = p2p::lock_swarm().unwrap();
                //p2p::prepared_message_handler();
                return (true,array_of_txn);
            }else{
                return (false, Vec::new());
            }
            //let the_outcome:bool= verkle_tree.node_exists_with_root(hash_array,);
    }   

}

impl App {
    fn new() -> Self {
        Self { blocks: vec![]}
    }
    
    pub fn genesis(&mut self) {
        let genesis_block = public_block::Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("00Genesis"),
            transactions:(vec!["".to_string()]),
            nonce: 1,
            public_hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.blocks.push(genesis_block);
    }
    pub fn try_add_block(&mut self, block: public_block::Block) {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if public_block::Block::is_block_valid(&block, latest_block) {
            self.blocks.push(block);
        } else {
            error!("could not add block - invalid");
        }
    }
    pub fn is_chain_valid(&self, chain: &[public_block::Block]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            //let block_instance = public_block::Block::new();
            if !public_block::Block::is_block_valid(second, first) {
                return false;
            }
        }
        true
    }
    // We always choose the longest valid chain
    fn choose_chain(&mut self, local: Vec<public_block::Block>, remote: Vec<public_block::Block>) -> Vec<public_block::Block> {
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);
        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else {
                remote
            }
        } else if is_remote_valid && !is_local_valid {
            remote
        } else if !is_remote_valid && is_local_valid {
            local
        } else {
            panic!("local and remote chains are both invalid");
        }
    }
}



#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Peer Id: {}", p2p::PEER_ID.clone());
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();
    let (init_sender, mut init_rcv) = mpsc::unbounded_channel();
    let (publisher, mut publish_receiver, mut publish_bytes_receiver): (Publisher, mpsc::UnboundedReceiver<(String, String)>, mpsc::UnboundedReceiver<(String, Vec<u8>)>) = Publisher::new();
    Publisher::set(publisher);
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&p2p::KEYS)
        .expect("can create auth keys");

    // Create and initialize your swarm here
    info!("Peer Id: {}", p2p::PEER_ID.clone());
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
        
    let behaviour = p2p::AppBehaviour::new(App::new(),Txn::new(),pbft::PBFTNode::new(p2p::PEER_ID.clone().to_string()), response_sender, init_sender.clone()).await;
    let mut swarm = swarm::create_swarm().await;

    let mut stdin = BufReader::new(stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");
    spawn(async move {
        sleep(Duration::from_secs(1)).await;
        info!("sending init event");
        init_sender.send(true).expect("can send init event");
    });

    loop {
        let evt = {
            select! {
                line = stdin.next_line() => Some(p2p::EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                response = response_rcv.recv() => {
                    Some(p2p::EventType::LocalChainResponse(response.expect("response exists")))
                },
                _init = init_rcv.recv() => {
                    Some(p2p::EventType::Init)
                }
                event = swarm.select_next_some() => {
                    //info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
                publish = publish_receiver.recv() => {
                    let (title, message) = publish.clone().expect("Publish exists");
                    info!("Publish Swarm Event: {:?}", title);
                    Some(p2p::EventType::Publish(title, message))
                },
                publish_block = publish_bytes_receiver.recv()=>{
                    let (title, message) = publish_block.clone().expect("Publish Block exists");
                    Some(p2p::EventType::PublishBlock(title, message.into()))
                }
             }
        };

        if let Some(event) = evt {
            match event {
                p2p::EventType::Init => {
                    let peers = p2p::get_list_peers(&swarm);
                    swarm.behaviour_mut().app.genesis();
                    info!("Connected nodes: {}", peers.len());
                    if !peers.is_empty() {
                        let req = p2p::LocalChainRequest {
                            from_peer_id: peers
                                .iter()
                                .last()
                                .expect("at least one peer")
                                .to_string(),
                        };

                        let json = serde_json::to_string(&req).expect("can jsonify request");
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
                }
                p2p::EventType::LocalChainResponse(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                }
                p2p::EventType::Publish(title,message)=>{
                    let title_json = serde_json::to_string(&title).expect("can jsonify title");
                    let topic_str = title_json.trim_matches('"');
                    let topic = libp2p::floodsub::Topic::new(topic_str);
                    let message_json = serde_json::to_string(&message).expect("can jsonify message");
                    let peers = p2p::get_list_peers(&swarm);
                    let pbft_node_views = pbft::get_total_pbft_view(&swarm);
                    // println!("Number of NODES: {:?}",peers.len());
                    // println!("PBFT Node number of views for consensus {:?}",pbft_node_views);
                    swarm.behaviour_mut().floodsub.publish(topic,message_json.as_bytes())
                }
                p2p::EventType::PublishBlock(title,message)=>{
                    let title_json = serde_json::to_string(&title).expect("can jsonify title");
                    let topic_str = title_json.trim_matches('"');
                    let topic = libp2p::floodsub::Topic::new(topic_str);
                    let message_json = serde_json::to_string(&message).expect("can jsonify message");
                    swarm.behaviour_mut().floodsub.publish(topic,message)
                }
                p2p::EventType::Input(line) => match line.as_str() {
                    "ls p" => p2p::handle_print_peers(&swarm),
                    cmd if cmd.starts_with("ls c") => p2p::handle_print_chain(&swarm),
                    cmd if cmd.starts_with("ls t") => p2p::handle_print_txn(&swarm),
                    cmd if cmd.starts_with("ls rt") => p2p::handle_print_raw_txn(&swarm),
                    cmd if cmd.starts_with("create b") => public_block::handle_create_block(cmd, &mut swarm),
                    cmd if cmd.starts_with("create txn")=> pbft::pbft_pre_message_handler(cmd, &mut swarm),
                    _ => error!("unknown command"),  
                },
            }
        }
    }
}