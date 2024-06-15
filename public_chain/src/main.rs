use libp2p::{
    gossipsub::{IdentTopic as Topic},
    kad::KademliaEvent,
    swarm::{Swarm, SwarmEvent},
    Multiaddr,
};
use local_ip_address::local_ip;
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    net::TcpListener,
    select, spawn,
    sync::{mpsc, Notify},
    time::sleep,
};
use tokio::signal;

mod account;
mod api;
mod bridge;
mod gas_calculator;
mod p2p;
mod pbft;
mod public_app;
mod public_block;
mod public_swarm;
mod public_txn;
mod publisher;
mod rock_storage;
mod rpc_connector;
mod smart_contract;
mod staking;
mod token;
mod txn_pool;
mod verkle_tree;

use p2p::AppEvent;
use bridge::accept_loop;
use public_app::App;
use publisher::Publisher;
use public_swarm::AppBehaviour;
use rock_storage::StoragePath;
use rocksdb::{DBWithThreadMode, SingleThreaded};
type MySwarm = Swarm<AppBehaviour>;


pub fn create_pub_storage()->  Result<rock_storage::StoragePath, Box<dyn std::error::Error>>{
    let paths = [
        "./public_blockchain",
        "./account",
        "./transactions",
        "./contract",
        "./node"
    ];

    for path in &paths {
        if !Path::new(path).exists() {
            fs::create_dir_all(path)?;
            println!("Directory {:?} created.", path);
        } else {
            eprintln!("Directory {:?} already exists.", path);
        }
    }
    for path in &paths {
        if !Path::new(path).exists() {
            rock_storage::create_storage(path)?;
        } else {
            eprintln!("Directory {:?} already exists.", path);
        }
    }

    let db_public_block =open_or_create_storage("./public_blockchain")?;
    let db_account = open_or_create_storage("./account")?;
    let db_node = open_or_create_storage("./node")?;
    let db_transactions =open_or_create_storage("./transactions")?;
    let db_contract = open_or_create_storage("./contract")?;
    let the_storage = rock_storage::StoragePath {
        blocks: db_public_block,
        account: db_account,
        transactions: db_transactions,
        contract: db_contract,
        node:db_node
    };

    println!("Storage initialized for blocks, accounts, contracts, node and transactions");
    Ok(the_storage)

}

fn open_or_create_storage(path: &str) -> Result<DBWithThreadMode<SingleThreaded>, Box<dyn std::error::Error>> {
    rock_storage::create_storage(path)?;
    if !Path::new(path).exists() {
        rock_storage::create_storage(path)?;
        println!("Database at path {:?} created.", path);
    } else {
        eprintln!("Database at path {:?} already exists.", path);
    }
    Ok(rock_storage::open_storage(path)?)
}
fn db_extract(db: Arc<RwLock<DBWithThreadMode<SingleThreaded>>>) -> DBWithThreadMode<SingleThreaded> {
    Arc::try_unwrap(db).unwrap().into_inner().unwrap()
}

pub fn remove_lock_file() {
    let lock_paths = [
        "./public_blockchain/LOCK",
        "./account/LOCK",
        "./contract/LOCK",
        "./transactions/LOCK",
        "./node/LOCK",
    ];

    for lock_path in &lock_paths {
        if let Err(e) = fs::remove_file(lock_path) {
            eprintln!("Error removing lock file {:?}: {:?}", lock_path, e);
        }
    }
}


async fn block_producer() {
    loop {
        // Your periodic function logic goes here
        let _ = public_block::pbft_pre_message_block_create_scheduler();

        // Sleep for the specified interval
        sleep(Duration::from_secs(5)).await; // Adjust the interval as needed
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    //     let mut whitelisted_peers = vec![
    //     "/ip4/46.137.235.97/tcp/8081",
    //     "/ip4/13.228.172.186/tcp/8082",
    //     // Add more whitelisted peers as needed
    // ];

    let mut whitelisted_listener = vec![
        "127.0.0.1:8089",
        "127.0.0.1:8090",
        "127.0.0.1:8091",
        "127.0.0.1:8092",
        "127.0.0.1:8093",
        "127.0.0.1:8094",
        "127.0.0.1:8095",
        "127.0.0.1:8096",
        "127.0.0.1:8097",
        "127.0.0.1:8098",
        "127.0.0.1:8099",
        "127.0.0.1:8100",
        "127.0.0.1:8101",
        "127.0.0.1:8102",
        ];
    //let whitelisted_peers = WhitelistedPeers::default();
    let my_local_ip = local_ip().unwrap();
    // Add initial whitelisted peers (if any)
    println!("This is my local IP address: {:?}", my_local_ip);
    let binding = my_local_ip.to_string();

    //create storage
    remove_lock_file();
    let the_storage = create_pub_storage().expect("Failed to create storage");
    //info!("Peer Id: {}", p2p::PEER_ID.clone());
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();
    let (init_sender, mut init_rcv) = mpsc::unbounded_channel();
    let (publisher, mut publish_receiver, mut publish_bytes_receiver): (Publisher, mpsc::UnboundedReceiver<(String, String)>, mpsc::UnboundedReceiver<(String, Vec<u8>)>) = Publisher::new();
    Publisher::set(publisher);
    let app = App::new();
    public_swarm::create_public_swarm(app.clone(),the_storage).await;
    // Lock the swarm and access it
    let rpc_runner = tokio::spawn( async{
        rpc_connector::start_rpc().await
    });
    tokio::spawn(block_producer());
    let swarm_mutex = public_swarm::get_global_swarm_public_net();
    let mut swarm_public_net_guard = swarm_mutex.lock().unwrap(); 
    let mut stdin = BufReader::new(stdin()).lines();
    //WHITE-LABEL PRODUCT: CHANGE OF CHAIN
    let mut gas_token = token::SMTXToken::new("SUMOTEX".to_string(), "SMTX".to_string(), 18, 1000000000000000000);
    let (pub_key,private_key)=account::create_account().expect("Failed to create a node account");
    let my_local_ip = local_ip().unwrap();

    println!("Pub node address: {:?}", pub_key);
    println!("Private node address: {:?}", private_key);
    //let binding = my_local_ip.to_string();
    if let Some(swarm_public_net) = &mut *swarm_public_net_guard {
        //rpc_connector::set_global_swarm_public_net(swarm_public_net);
        swarm_public_net.behaviour_mut().app.genesis();
        loop {
            if let Some(port) = whitelisted_listener.pop() {
                match TcpListener::bind(&port).await {
                    Ok(listener) => {
                        // Loop to listen
                        let accept_loop_task = tokio::spawn(async {
                            accept_loop(listener).await;
                        });
                        println!("TCP Port: {:?}",port);
                        break;
                    }
                    Err(e) => {
                        info!("Failed to bind to {}: {}", port, e);
                    }
                }
            } else {
                info!("No more TCP Ports!");
            }
        }
        loop {
            // if let Some(port) = whitelisted_peers.pop() {
                let address_str = format!("/ip4/{}/tcp/8100",(my_local_ip.to_string()));
                let the_address = Multiaddr::from_str(&address_str).expect("Failed to parse multiaddr");  
                println!("{}",the_address);      
                //Loop  to listen
                match Swarm::listen_on( swarm_public_net, the_address.clone()) {
                    Ok(_) => {
                        spawn(async move {
                            info!("sending init event");
                            init_sender.send(true).expect("can send init event");
                        });
                        break;
                    },
                    Err(e) => {
                        info!("Failed to listen on {:?}. Reason: {:?}", the_address, e);
                    }
                }
            // } else {
            //     info!("No more whitelisted Peers!");
            // }
        }
        let mut init_received = false;  // flag to track if Init event is processed
        if !init_received {
            let recv_result = init_rcv.recv().await;
            match recv_result {
                Some(_) => {
                    init_received = true;
                },
                None => {
                    // Handle the case where recv_result is None, perhaps breaking the loop or continuing
                },
            }
        }
        loop {
            let public_evt = 
                select! {
                    line = stdin.next_line() => Some(p2p::EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                    response = response_rcv.recv() => {
                        Some(p2p::EventType::LocalChainResponse(response.expect("response exists")))
                    },
                    event = swarm_public_net.select_next_some() => {
                        let api_app =swarm_public_net.behaviour_mut().app.clone();
                        rpc_connector::add_api_blocks(api_app.clone());
                        match event {
                            SwarmEvent::Behaviour(app_event) => {
                              
                                println!("Received network behaviour event: {:?}", app_event);
                                match app_event {
                                    AppEvent::AccountCreation { propagation_source, message_id, data } => {
                                        println!("Received account creation message from {}", propagation_source);
                                        swarm_public_net.behaviour_mut().process_account_creation_event(&data);
                                    },
                                    AppEvent::CreateBlocks { propagation_source, message_id, data } => {
                                        println!("Received create blocks message from {}", propagation_source);
                                        swarm_public_net.behaviour_mut().process_create_blocks(&data);
                                    },
                                    AppEvent::TxnPbftPrepared { propagation_source, message_id, data } => {
                                        println!("Received txn PBFT prepared message from {}", propagation_source);
                                        swarm_public_net.behaviour_mut().process_txn_pbft_prepared(&data);
                                    },
                                    AppEvent::TxnPbftCommit { propagation_source, message_id, data } => {
                                        println!("Received txn PBFT commit message from {}", propagation_source);
                                        swarm_public_net.behaviour_mut().process_txn_pbft_commit(&data);
                                    },
                                    AppEvent::BlockPbftPrePrepared { propagation_source, message_id, data } => {
                                        println!("Received block PBFT pre-prepared message from {}", propagation_source);
                                        swarm_public_net.behaviour_mut().process_block_pbft_pre_prepared(&data);
                                    },
                                    AppEvent::BlockPbftCommit { propagation_source, message_id, data } => {
                                        println!("Received block PBFT commit message from {}", propagation_source);
                                        swarm_public_net.behaviour_mut().process_block_pbft_commit(&data);
                                    },
                                    AppEvent::PrivateBlocksGenesisCreation { propagation_source, message_id, data } => {
                                        println!("Received private blocks genesis creation message from {}", propagation_source);
                                        swarm_public_net.behaviour_mut().process_private_blocks_genesis_creation(&data);
                                    },
                                    AppEvent::HybridBlockCreation { propagation_source, message_id, data } => {
                                        println!("Received hybrid block creation message from {}", propagation_source);
                                        swarm_public_net.behaviour_mut().process_hybrid_block_creation(&data);
                                    },
                                    AppEvent::Gossipsub(_) => {
                                        println!("Received a Gossipsub event.");
                                    },
                                    _ => {
                                        println!("Received another type of AppEvent.");
                                    }
                                }
                                None
                            }
                            _ =>{
                            println!("Received other swarm event: {:?}", event);
                            None
                        }
                        }
                      
                    },
                    publish = publish_receiver.recv() => {
                        let (title, message) = publish.clone().expect("Publish exists");
                        let api_app =swarm_public_net.behaviour_mut().app.clone();
                        rpc_connector::add_api_blocks(api_app.clone());
                        info!("Publish Swarm Event: {:?}", title);
                        Some(p2p::EventType::Publish(title, message))
                    },
                    publish_block = publish_bytes_receiver.recv()=>{
                        let (title, message) = publish_block.clone().expect("Publish Block exists");
                        let api_app =swarm_public_net.behaviour_mut().app.clone();
                        rpc_connector::add_api_blocks(api_app.clone());
                        Some(p2p::EventType::PublishBlock(title, message.into()))
                    }
                };
                if let Some(event) = public_evt {
                    match event {
                        p2p::EventType::Init => {
                            // let peers = p2p::get_list_peers(&swarm_public_net);
                            
                            // info!("Connected nodes: {}", peers.len());
                            // if !peers.is_empty() {
                            //     let req = p2p::LocalChainRequest {
                            //         from_peer_id: peers
                            //             .iter()
                            //             .last()
                            //             .expect("at least one peer")
                            //             .to_string(),
                            //     };
                            //     let json = serde_json::to_string(&req).expect("can jsonify request");

                            //     swarm_public_net
                            //         .behaviour_mut()
                            //         .floodsub
                            //         .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                            // }
                        }
                        p2p::EventType::LocalChainResponse(resp) => {
                            let json = serde_json::to_string(&resp).expect("can jsonify response");
                            swarm_public_net
                                .behaviour_mut()
                                .gossipsub
                                .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                        }
                        p2p::EventType::Publish(title,message)=>{
                            let title_json = serde_json::to_string(&title).expect("can jsonify title");
                            let topic_str = title_json.trim_matches('"');
                            let topic = Topic::new(topic_str);
                            let message_json = serde_json::to_string(&message).expect("can jsonify message");
                            swarm_public_net.behaviour_mut().gossipsub.publish(topic,message_json.as_bytes());
                        }
                        p2p::EventType::PublishBlock(title,message)=>{
                            println!("Topic: {:?}",title);
                            println!("Message: {:?}",message);
                            let title_json = serde_json::to_string(&title).expect("can jsonify title");
                            let topic_str = title_json.trim_matches('"');
                            let topic = Topic::new(topic_str);
                            let message_json = serde_json::to_string(&message).expect("can jsonify message");
                            match swarm_public_net.behaviour_mut().gossipsub.publish(topic, message) {
                                Ok(_) => println!("Message published to topic {}", title),
                                Err(e) => println!("Failed to publish message to topic {}: {:?}", title, e),
                            }
                        }
                        p2p::EventType::Input(line) => {
                            let command = line.trim();
                            if command == "exit" {
                                // Exit logic here. If it's within a loop, you might need a way to break out of or return from the loop/function.
                                println!("Exiting...");
                            }else if command.starts_with("ls node") {
                                // Split the command to separate the listener address and optional peer address
                                let parts: Vec<&str> = command.strip_prefix("ls node").unwrap().trim().split_whitespace().collect();
                                match parts.len() {
                                    1 => {
                                        // Only a listener address is provided
                                        match public_swarm::setup_node(parts[0], None, swarm_public_net).await {
                                            Ok(_) => println!("Listener added on {}", parts[0]),
                                            Err(e) => eprintln!("Error setting up node: {}", e),
                                        }
                                    },
                                    2 => {
                                        // Both a listener address and a peer address are provided
                                        match public_swarm::setup_node(parts[0], Some(parts[1].to_string()), swarm_public_net).await {
                                            Ok(_) => println!("Listener added on {} and dialed peer {}", parts[0], parts[1]),
                                            Err(e) => eprintln!("Error setting up node: {}", e),
                                        }
                                    },
                                    _ => println!("Invalid 'ls node' command format. Expected 'ls node [listener address] [optional peer address]'"),
                                }
                            }else if command.starts_with("ls peer"){
                                public_swarm::check_connected_peers(swarm_public_net);
                            }else {
                                println!("Unknown command: {}", command);
                            }
                        },
                    }
                }
            }
        } else {
            panic!("Swarm not initialized");
        }
}