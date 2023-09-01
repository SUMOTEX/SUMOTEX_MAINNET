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
use tokio::net::TcpStream;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use std::io::Result;
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
use std::str::FromStr;
use libp2p::Multiaddr;
use libp2p::futures::StreamExt;
mod verkle_tree;
mod private_p2p;
mod publisher;
mod private_block;
mod pbft;
mod private_pbft;
mod account_root;
mod private_transactions;
mod private_app;
mod private_swarm;
mod bridge;
use bridge::accept_loop;
use tokio::net::TcpListener;
use publisher::Publisher;
use crate::account_root::AccountRoot;




use common::common::SMTXBridge;
enum CustomEvent {
    ReceivedRequest(PeerId, Vec<u8>),
    ReceivedResponse(PeerId, Vec<u8>),
    // ... potentially other custom events specific to your application
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
        // ... other addresses
        ];
    
    let mut whitelisted_listener = vec![
        "127.0.0.1:8088",
        "127.0.0.1:8089",
        "127.0.0.1:8090",
        "127.0.0.1:8090",
        "127.0.0.1:8091",
        "127.0.0.1:8090",
        "127.0.0.1:8092",
        "127.0.0.1:8093",
        "127.0.0.1:8094",
        "127.0.0.1:8095",
        // ... other addresses
        ];
    //PRIVATE
    let (response_private_sender, mut response_private_rcv) = mpsc::unbounded_channel();
    let (init_private_sender, mut init_private_rcv) = mpsc::unbounded_channel();
    let (private_tx, mut _private_rx): (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>) = mpsc::unbounded_channel();    //bridge sender
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


    let mut swarm_private_net = private_swarm::create_private_swarm(private_tx.clone()).await;
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
    let key_bytes = private_p2p::PEER_ID.clone().to_bytes();
    let kad_key = Key::new(&key_bytes);

    // Put a record to DHT
    let record = Record {
        key: kad_key.clone(),
        value: b"some_value".to_vec(),
        publisher: Some(private_p2p::PEER_ID.clone()),
        expires: None,
    };
    // Create a Multiaddress for the bootstrap node. Replace with the actual address.
    let bootstrap_addr: Multiaddr = "/ip4/104.131.131.82/tcp/4001/p2p/QmSoLnSGccFuZQJzRadHn95W2CrSFmZuTdDWP8HXaHca9z".parse().unwrap();
    let bootstrap_peer_id = "QmSoLnSGccFuZQJzRadHn95W2CrSFmZuTdDWP8HXaHca9z".parse().unwrap();
    // Bootstrap by connecting to the known peer.
    Swarm::dial_addr(&mut swarm_private_net, bootstrap_addr.clone()).expect("Failed to dial bootstrap node");
    swarm_private_net.behaviour_mut().kademlia.add_address(&bootstrap_peer_id, bootstrap_addr.clone());

    swarm_private_net.behaviour_mut().kademlia.put_record(record, Quorum::One);

    // Get a record from DHT
    let the_record = swarm_private_net.behaviour_mut().kademlia.get_record(&kad_key, Quorum::One);
    let vec_u8_key=kad_key.to_vec(); // Assuming to_vec_u8() is a method that returns Option<Vec<u8>>
    swarm_private_net.behaviour_mut().kademlia.get_closest_peers(vec_u8_key);
    // Inform the swarm to send a message to the dialed peer.
    loop {
        if let Some(port) = whitelisted_listener.pop() {
            match TcpListener::bind(&port).await {
                Ok(listener) => {
                    // Loop to listen
                    let accept_loop_task = tokio::spawn(async {
                        accept_loop(listener).await;
                    });
                    break;
                }
                Err(e) => {
                    info!("Failed to bind to {}: {}", port, e);
                }
            }
        } else {
            info!("No more ports to pop!");
        }
    }
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
    let mut init_received = false;  // flag to track if Init event is processed

    if !init_received {
        let recv_result = init_private_rcv.recv().await;
        match recv_result {
            Some(_) => {
                println!("Initialization event received.");
                init_received = true;  // Set flag to true, so this block won't execute again
                private_p2p::EventType::Init;
                // Now you can return Some(p2p::EventType::Init) or do something else
            },
            None => {
                // Handle the case where recv_result is None, perhaps breaking the loop or continuing
            },
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
                // _init = init_private_rcv.recv() => {
                //     info!("Private Block Setup");
                //     Some(private_p2p::EventType::Init)  
                // }
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
                        //private_p2p::handle_start_chain(&mut swarm_private_net);
                        if !peers.is_empty() {
                            let req = private_p2p::PrivateLocalChainRequest {
                                from_peer_id: peers
                                    .iter()
                                    .last()
                                    .expect("at least 2 peer")
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
                        println!("KADEMLIA {:?}",resp);
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