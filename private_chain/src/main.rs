use libp2p::{
    swarm::{Swarm}
};
use local_ip_address::local_ip;
use log::{error, info};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep
};
use std::path::Path;
use std::fs;
use libp2p::Multiaddr;
use std::str::FromStr;
use tokio::time::{Duration};
use libp2p::futures::StreamExt;
mod verkle_tree;
mod p2p;
mod swarm;
mod publisher;
mod block;
mod pbft;
mod app;
mod txn;
mod bridge;
mod rock_storage;
mod api;
mod account;
mod smart_contract;
mod rpc_connector;
mod gas_calculator;
mod txn_pool;
mod token;
mod staking;
use crate::staking::NodeInfo;
use bridge::accept_loop;
use crate::app::App;
use std::sync::{RwLock, Arc};
use publisher::Publisher;
use tokio::net::TcpListener;
use crate::p2p::AppBehaviour;
use rocksdb::DBWithThreadMode;
use rocksdb::SingleThreaded;
type MySwarm = Swarm<AppBehaviour>;


pub fn create_pub_storage()->  Result<rock_storage::StoragePath, Box<dyn std::error::Error>>{
    let paths = [
        "./blockchain",
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

    let db_block =open_or_create_storage("./blockchain")?;
    let db_account = open_or_create_storage("./account")?;
    let db_node = open_or_create_storage("./node")?;
    let db_transactions =open_or_create_storage("./transactions")?;
    let db_contract = open_or_create_storage("./contract")?;
    let the_storage = rock_storage::StoragePath {
        blocks: db_block,
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
    let lock_path = "./blockchain/LOCK";
    if let Err(e) = fs::remove_file(lock_path) {
        eprintln!("Error removing lock file: {:?}", e);
    }
    let lock_path_2 = "./account/LOCK";
    if let Err(e) = fs::remove_file(lock_path_2) {
        eprintln!("Error removing lock file: {:?}", e);
    }
    let lock_path_3 = "./contract/LOCK";
    if let Err(e) = fs::remove_file(lock_path_3) {
        eprintln!("Error removing lock file: {:?}", e);
    }
    let lock_path_4 = "./transactions/LOCK";
    if let Err(e) = fs::remove_file(lock_path_4) {
        eprintln!("Error removing lock file: {:?}", e);
    }
    let lock_path_5 = "./node/LOCK";
    if let Err(e) = fs::remove_file(lock_path_5) {
        eprintln!("Error removing lock file: {:?}", e);
    }
}


async fn block_producer() {
    loop {
        // Your periodic function logic goes here
        let _ = block::pbft_pre_message_block_create_scheduler();

        // Sleep for the specified interval
        sleep(Duration::from_secs(20)).await; // Adjust the interval as needed
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
        "/ip4/0.0.0.0/tcp/8091",
        "/ip4/0.0.0.0/tcp/8092",
        "/ip4/0.0.0.0/tcp/8093",
        "/ip4/0.0.0.0/tcp/8094",
        "/ip4/0.0.0.0/tcp/8095",
        "/ip4/0.0.0.0/tcp/8096",
        "/ip4/0.0.0.0/tcp/8097",
        "/ip4/0.0.0.0/tcp/8098",
        // ... other addresses
        ];

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
    whitelisted_peers.push(&binding);

    //create storage
    remove_lock_file();
    let the_storage = create_pub_storage().expect("Failed to create storage");
    //info!("Peer Id: {}", p2p::PEER_ID.clone());
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();
    let (init_sender, mut init_rcv) = mpsc::unbounded_channel();
    let (publisher, mut publish_receiver, mut publish_bytes_receiver): (Publisher, mpsc::UnboundedReceiver<(String, String)>, mpsc::UnboundedReceiver<(String, Vec<u8>)>) = Publisher::new();
    Publisher::set(publisher);
    let app = App::new();
    swarm::create_public_swarm(app.clone(),the_storage).await;
    // Lock the swarm and access it
    println!("Before RPC server");
    let rpc_runner = tokio::spawn( async{
        rpc_connector::start_rpc().await
    });
    tokio::spawn(block_producer());
    // let rpc_runner = tokio::task::spawn_local(async {
    //     rpc_connector::start_rpc().await
    // });    
    println!("After RPC server");
    let swarm_mutex = swarm::get_global_swarm_public_net();

    let mut stdin = BufReader::new(stdin()).lines();
    let mut swarm_public_net_guard = swarm_mutex.lock().unwrap();    
    //WHITE-LABEL PRODUCT: CHANGE OF CHAIN
    let mut gas_token = token::SMTXToken::new("SUMOTEX".to_string(), "SMTX".to_string(), 18, 1000000000000000000);
    if let Some(swarm_public_net) = &mut *swarm_public_net_guard {
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
        //let the_address = Multiaddr::from_str("/ip4/0.0.0.0/tcp/8083").expect("Failed to parse multiaddr");
        loop {
            if let Some(port) = whitelisted_peers.pop() {
                let address_str = format!("{}",port);
                let the_address = Multiaddr::from_str(&address_str).expect("Failed to parse multiaddr");        
                //Loop  to listen
                match Swarm::listen_on( swarm_public_net, the_address.clone()) {
                    Ok(_) => {
                        info!("Listening on {:?}", the_address.clone());
                        spawn(async move {
                            //sleep(Duration::from_secs(1)).await;
                            info!("sending init event");
                            init_sender.send(true).expect("can send init event");
                        });
                        break;
                    },
                    Err(e) => {
                        info!("Failed to listen on {:?}. Reason: {:?}", the_address, e);
                    }
                    }
            } else {
                info!("No more Whitelisted Peers!");
            }
        }
        let mut init_received = false;  // flag to track if Init event is processed
        if !init_received {
            let recv_result = init_rcv.recv().await;
            match recv_result {
                Some(_) => {
                    let peers = p2p::get_list_peers(&swarm_public_net);
                    let _ = swarm_public_net.behaviour_mut().app.initialize_from_storage();
                    // info!("Connected nodes: {}", peers.len());
                    if !peers.is_empty() {
                        let req = p2p::LocalChainRequest {
                            from_peer_id: peers
                                .iter()
                                .last()
                                .expect("at least one peer")
                                .to_string(),
                        };
                        let (pub_key,private_key)=account::create_account().expect("Failed to create account");
                        let n_path = "./node/db";
                        
                        let node_path = match rock_storage::open_db(n_path) {
                            Ok(path) => path,
                            Err(e) => {
                                // Handle the error, maybe log it, and then decide what to do next
                                panic!("Failed to open database: {:?}", e); // or use some default value or error handling logic
                            }
                        };
                        //let whitelisted_peers = WhitelistedPeers::default();
                        let my_local_ip = local_ip().unwrap();
                        // Add initial whitelisted peers (if any)
                        println!("This is my local IP address: {:?}", my_local_ip);
                        let binding = my_local_ip.to_string();
                        let _ = rock_storage::put_to_db(&node_path,"node_id",&pub_key.clone().to_string());
                        let json = serde_json::to_string(&req).expect("can jsonify request");
                        let start = SystemTime::now();
                        let since_the_epoch = start.duration_since(UNIX_EPOCH)?;
                        let node_info = NodeInfo {
                            node_address: pub_key.to_string(),
                            ip_address:my_local_ip.to_string(),
                            last_active: since_the_epoch.as_secs(),
                        };
                        NodeInfo::upsert_node_info(&db, &node_info)?;

                        swarm_public_net
                            .behaviour_mut()
                            .floodsub
                            .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
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
                        None
                    }
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
                            let peers = p2p::get_list_peers(&swarm_public_net);
                            swarm_public_net.behaviour_mut().app.genesis();
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
                                swarm_public_net
                                    .behaviour_mut()
                                    .floodsub
                                    .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                            }
                        }
                        p2p::EventType::LocalChainResponse(resp) => {
                            let json = serde_json::to_string(&resp).expect("can jsonify response");
                            swarm_public_net
                                .behaviour_mut()
                                .floodsub
                                .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                        }
                        p2p::EventType::Publish(title,message)=>{
                            let title_json = serde_json::to_string(&title).expect("can jsonify title");
                            let topic_str = title_json.trim_matches('"');
                            let topic = libp2p::floodsub::Topic::new(topic_str);
                            let message_json = serde_json::to_string(&message).expect("can jsonify message");
                            let peers = p2p::get_list_peers(&swarm_public_net);
                            swarm_public_net.behaviour_mut().floodsub.publish(topic,message_json.as_bytes())
                        }
                        p2p::EventType::PublishBlock(title,message)=>{
                            let title_json = serde_json::to_string(&title).expect("can jsonify title");
                            let topic_str = title_json.trim_matches('"');
                            let topic = libp2p::floodsub::Topic::new(topic_str);
                            let message_json = serde_json::to_string(&message).expect("can jsonify message");
                            swarm_public_net.behaviour_mut().floodsub.publish(topic,message)
                        }
                        p2p::EventType::Input(line) => match line.as_str() {
                            "ls p" => p2p::handle_print_peers(&swarm_public_net),
                            cmd if cmd.starts_with("ls b") => p2p::handle_print_chain(&swarm_public_net),
                            cmd if cmd.starts_with("ls rt") => p2p::handle_print_raw_txn(&swarm_public_net),
                            _ => error!("unknown command"),  
                        },
                    }
                }
            }
        } else {
            panic!("Swarm not initialized");
        }
}