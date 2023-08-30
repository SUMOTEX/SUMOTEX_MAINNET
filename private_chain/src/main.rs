use chrono::prelude::*;
use libp2p::{
    core::{
        upgrade::{self},
    },
    mplex,
    identity, noise,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm,SwarmBuilder},
    tcp::TokioTcpConfig,
    Transport,
    PeerId,
};
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{
    record::Key, AddProviderOk, GetProvidersOk, GetRecordOk, Kademlia, KademliaEvent, PeerRecord,
    PutRecordOk, QueryResult, Record,Quorum
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
use std::str::FromStr;
use libp2p::Multiaddr;
use libp2p::futures::StreamExt;
mod verkle_tree;
mod private_p2p;
mod private_swarm;
mod publisher;
mod private_block;
mod pbft;
mod private_pbft;
mod account_root;
use publisher::Publisher;
use crate::account_root::AccountRoot;

enum CustomEvent {
    ReceivedRequest(PeerId, Vec<u8>),
    ReceivedResponse(PeerId, Vec<u8>),
    // ... potentially other custom events specific to your application
}
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
pub struct PrivateApp {
    pub blocks: Vec<private_block::PrivateBlock>,
}



#[allow(clippy::large_enum_variant)]
enum MyBehaviourEvent {
    Kademlia(KademliaEvent),
}

impl From<KademliaEvent> for MyBehaviourEvent {
    fn from(event: KademliaEvent) -> Self {
        MyBehaviourEvent::Kademlia(event)
    }
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
impl PrivateApp {
    fn new() -> Self {
        Self { blocks: vec![]}
    }
    
    pub fn genesis(&mut self)->private_block::PrivateBlock {
        let genesis_block = private_block::PrivateBlock {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("00Genesis"),
            private_hash:Some(String::from("00")),
            transactions:(vec!["".to_string()].into()),
            nonce: 1,
            public_hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.blocks.push(genesis_block.clone());
        return genesis_block.clone();
    }
    pub fn try_add_genesis(&mut self) {
        let genesis_block = private_block::PrivateBlock {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("00Genesis"),
            private_hash:Some(String::from("00")),
            transactions:(vec!["".to_string()].into()),
            nonce: 1,
            public_hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.blocks.push(genesis_block.clone());
    }
    pub fn try_add_block(&mut self, block: private_block::PrivateBlock) {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if private_block::PrivateBlock::is_block_valid(&block, latest_block) {
            self.blocks.push(block);
        } else {
            error!("could not add block - invalid");
        }
    }
    pub fn is_chain_valid(&self, chain: &[private_block::PrivateBlock]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            //let block_instance = public_block::Block::new();
            if !private_block::PrivateBlock::is_block_valid(second, first) {
                return false;
            }
        }
        true
    }
    // We always choose the longest valid chain
    fn choose_chain(&mut self, local: Vec<private_block::PrivateBlock>, remote: Vec<private_block::PrivateBlock>) -> Vec<private_block::PrivateBlock> {
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
    info!("Peer Id: {}", private_p2p::PEER_ID.clone());
    let mut whitelisted_peers = vec![
        "/ip4/0.0.0.0/tcp/8081",
        "/ip4/0.0.0.0/tcp/8082",
        "/ip4/0.0.0.0/tcp/8083",
        "/ip4/0.0.0.0/tcp/8084",
        "/ip4/0.0.0.0/tcp/8085",
        "/ip4/0.0.0.0/tcp/8086",
        "/ip4/0.0.0.0/tcp/8087",
        "/ip4/0.0.0.0/tcp/8089",
        "/ip4/0.0.0.0/tcp/8090",
        // ... other addresses
        ];
    

    //PRIVATE
    let (response_private_sender, mut response_private_rcv) = mpsc::unbounded_channel();
    let (init_private_sender, mut init_private_rcv) = mpsc::unbounded_channel();

    let (publisher, mut publish_receiver, mut publish_bytes_receiver): (Publisher, mpsc::UnboundedReceiver<(String, String)>, mpsc::UnboundedReceiver<(String, Vec<u8>)>) = Publisher::new();
    Publisher::set(publisher);
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&private_p2p::KEYS)
        .expect("can create auth keys");

    // Create and initialize your swarm here
    const SMTX_TESTNET_PROTOCOL: &str = "smtxtestnet/1.0.0";
    const SMTX_TESTNET_PROTOCOL_BYTES: &[u8] = b"/smtxtestnet/1.0.0";
    info!("Private Network Peer Id: {}", private_p2p::PEER_ID.clone());

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&private_p2p::KEYS)
        .expect("can create auth keys");
    // Convert to AuthenticKeypair
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
    // Create a swarm to manage peers and events.

    
    // Create a Kademlia behaviour.
    let store = MemoryStore::new(private_p2p::PEER_ID.clone());
    let kademlia = Kademlia::new(private_p2p::PEER_ID.clone(), store);
    let private_behaviour = private_p2p::PrivateAppBehaviour::new(
        PrivateApp::new(),
        Txn::new(),
        pbft::PBFTNode::new(private_p2p::PEER_ID.clone().to_string()),
        account_root::AccountRoot::new(),
        kademlia,
        response_private_sender, 
        init_private_sender.clone()).await;

    let mut swarm_private_net = SwarmBuilder::new(transp, private_behaviour, private_p2p::PEER_ID.clone())
    .executor(Box::new(|fut| {
        spawn(fut);
    }))
    .build(); 
    //swarm_private_net.behaviour_mut().kademlia.set_mode(Some(Mode::Server));


    // Using libp2p's Kademlia as an example

    //let mut swarm_private_net = private_swarm::create_swarm().await;
    let mut stdin = BufReader::new(stdin()).lines();
    //TODO: Make the publicnet validators dynamics connection randomised
    let public_chain_peer_id = "12D3KooWSHD7vtVa4zCiTNEjUt3o1zL4FMVZkPzjFUEVipkDQoPi".to_string();
    let public_net_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/8088/p2p/{}",public_chain_peer_id).parse().unwrap();
    // swarm_private_net.dial_addr(public_net_addr).expect("Failed to dial Public SMTX");
    // let remote_peer_id = PeerId::from_str(&public_chain_peer_id).expect("Failed to pass PeerId");
    let target_peer_id:PeerId = "QmSoLnSGccFuZQJzRadHn95W2CrSFmZuTdDWP8HXaHca9z".parse().unwrap();
    let target_peer_addr: Multiaddr = format!("/ip4/104.131.131.82/tcp/4001/p2p/{}", target_peer_id).parse().unwrap();

    // Connect to the target
    Swarm::dial_addr(&mut swarm_private_net, target_peer_addr).expect("Failed to dial");
    let peer_id = "QmSoLnSGccFuZQJzRadHn95W2CrSFmZuTdDWP8HXaHca9z";
    // The key we want to put/get in the DHT
    let key = Key::new(b"QmSoLnSGccFuZQJzRadHn95W2CrSFmZuTdDWP8HXaHca9z");

    // Put a record to DHT
    let record = Record {
        key: key.clone(),
        value: b"some_value".to_vec(),
        publisher: Some(private_p2p::PEER_ID.clone()),
        expires: None,
    };

    swarm_private_net.behaviour_mut().kademlia.put_record(record, Quorum::One);

    // Get a record from DHT
    let the_record = swarm_private_net.behaviour_mut().kademlia.get_record(&key, Quorum::One);
    println!("The records {:?}",the_record);


    // Inform the swarm to send a message to the dialed peer.

    loop {
        if let Some(port) = whitelisted_peers.pop() {
            let address_str = format!("{}",port);
            let the_address = Multiaddr::from_str(&address_str).expect("Failed to parse multiaddr");        
            info!("{:?}", the_address.clone());
            match Swarm::listen_on(&mut swarm_private_net, the_address.clone()) {
                Ok(_) => {
                    info!("Listening on {:?}", the_address.clone());
                    spawn(async move {
                        sleep(Duration::from_secs(1)).await;
                        info!("sending init event");
                        init_private_sender.send(true).expect("can send init event");
                    });
                    break;
                },
                Err(e) => {
                    info!("Failed to listen on {:?}. Reason: {:?}", the_address, e);
                }
                }
            
        } else {
            info!("No more ports to pop!");
        }
    }

    loop {
        
        let private_evt = 
            select! {
                line = stdin.next_line() => 
                    Some(private_p2p::EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                response = response_private_rcv.recv() => {
                    Some(private_p2p::EventType::LocalChainResponse(response.expect("response exists")))
                },
                _init = init_private_rcv.recv() => {
                    info!("Private Block Setup");
                    Some(private_p2p::EventType::Init)
                }
                event = swarm_private_net.select_next_some() => {
                    None
                },
                publish = publish_receiver.recv() => {
                    let (title, message) = publish.clone().expect("Publish exists");
                    info!("Publish Swarm Event: {:?}", title);
                    Some(private_p2p::EventType::Publish(title, message))
                },
            };
            if let Some(event) = private_evt {
                match event {
                    private_p2p::EventType::Init => {
                        let peers = private_p2p::get_list_peers(&swarm_private_net);
                        //swarm_private_net.behaviour_mut().app.genesis();
                        info!("Connected nodes: {}", peers.len());
                        if !peers.is_empty() {
                            let req = private_p2p::PrivateLocalChainRequest {
                                from_peer_id: peers
                                    .iter()
                                    .last()
                                    .expect("at least 4 peer")
                                    .to_string(),
                            };
    
                            let json = serde_json::to_string(&req).expect("can jsonify request");
                            swarm_private_net
                                .behaviour_mut()
                                .floodsub
                                .publish(private_p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                        }
                    }
                    private_p2p::EventType::Kademlia(resp)=>{
                        //let json = KademliaEvent::from_slice(resp).expect("can jsonify response");
                        println!("{:?}",resp);
                    }
                    private_p2p::EventType::LocalChainResponse(resp) => {
                        let json = serde_json::to_string(&resp).expect("can jsonify response");
                        swarm_private_net
                            .behaviour_mut()
                            .floodsub
                            .publish(private_p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
                    private_p2p::EventType::Publish(title,message)=>{
                        let title_json = serde_json::to_string(&title).expect("can jsonify title");
                        let topic_str = title_json.trim_matches('"');
                        let topic = libp2p::floodsub::Topic::new(topic_str);
                        let message_json = serde_json::to_string(&message).expect("can jsonify message");
                        let peers = private_p2p::get_list_peers(&swarm_private_net);
                        // println!("Number of NODES: {:?}",peers.len());
                        // println!("PBFT Node number of views for consensus {:?}",pbft_node_views);
                        swarm_private_net.behaviour_mut().floodsub.publish(topic,message_json.as_bytes())
                    }
                    private_p2p::EventType::PublishBlock(title,message)=>{
                        let title_json = serde_json::to_string(&title).expect("can jsonify title");
                    }
                    private_p2p::EventType::Input(line) => match line.as_str() {
                        "ls p" => private_p2p::handle_print_peers(&swarm_private_net),
                        "start"=>private_p2p::handle_start_chain(&mut swarm_private_net),
                        cmd if cmd.starts_with("ls b") => private_p2p::handle_print_chain(&swarm_private_net),
                        cmd if cmd.starts_with("create txn")=> private_pbft::pbft_pre_message_handler(cmd, &mut swarm_private_net),
                        _ => error!("unknown command"),  
                    },
                }
        }
        }
}