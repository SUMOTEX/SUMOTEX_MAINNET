use libp2p::{
    swarm::{Swarm},
    Transport,
};
use libp2p::PeerId;
use crate::verkle_tree::VerkleTree;
use log::{error, info};
use sha2::{Digest};
use std::time::Duration;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};
use libp2p::Multiaddr;
use std::str::FromStr;
use libp2p::futures::StreamExt;
mod verkle_tree;
mod p2p;
mod public_swarm;
mod publisher;
mod public_block;
mod pbft;
mod public_app;
mod public_txn;
mod bridge;
use bridge::accept_loop;
use publisher::Publisher;
use tokio::net::TcpStream;
use tokio::net::TcpListener;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use std::io::Result;
enum CustomEvent {
    ReceivedRequest(PeerId, Vec<u8>),
    ReceivedResponse(PeerId, Vec<u8>),
    // ... potentially other custom events specific to your application
}


#[tokio::main]
async fn main() ->tokio::io::Result<()>{
    pretty_env_logger::init();
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
        // ... other addresses
        ];
    //info!("Peer Id: {}", p2p::PEER_ID.clone());
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();
    let (init_sender, mut init_rcv) = mpsc::unbounded_channel();


    let (publisher, mut publish_receiver, mut publish_bytes_receiver): (Publisher, mpsc::UnboundedReceiver<(String, String)>, mpsc::UnboundedReceiver<(String, Vec<u8>)>) = Publisher::new();
    Publisher::set(publisher);
    // Create and initialize your swarm here
    let mut swarm_public_net = public_swarm::create_public_swarm().await;
    let mut stdin = BufReader::new(stdin()).lines();
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
    let the_address = Multiaddr::from_str("/ip4/0.0.0.0/tcp/8083").expect("Failed to parse multiaddr");
    loop {
        if let Some(port) = whitelisted_peers.pop() {
            let address_str = format!("{}",port);
            let the_address = Multiaddr::from_str(&address_str).expect("Failed to parse multiaddr");        
            //Loop  to listen
            match Swarm::listen_on(&mut swarm_public_net, the_address.clone()) {
                Ok(_) => {
                    info!("Listening on {:?}", the_address.clone());
                    spawn(async move {
                        sleep(Duration::from_secs(1)).await;
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
            info!("No more ports to pop!");
        }
    }
    let mut init_received = false;  // flag to track if Init event is processed

    if !init_received {
        let recv_result = init_rcv.recv().await;
        match recv_result {
            Some(_) => {
                println!("Initialization event received.");
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
                init_received = true;  // Set flag to true, so this block won't execute again
                // Now you can return Some(p2p::EventType::Init) or do something else
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
                    None
                }
                publish = publish_receiver.recv() => {
                    let (title, message) = publish.clone().expect("Publish exists");
                    info!("Publish Swarm Event: {:?}", title);
                    Some(p2p::EventType::Publish(title, message))
                },
                publish_block = publish_bytes_receiver.recv()=>{
                    let (title, message) = publish_block.clone().expect("Publish Block exists");
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
                        let pbft_node_views = pbft::get_total_pbft_view(&swarm_public_net);
                        // println!("Number of NODES: {:?}",peers.len());
                        // println!("PBFT Node number of views for consensus {:?}",pbft_node_views);
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
                        cmd if cmd.starts_with("ls t") => p2p::handle_print_txn(&swarm_public_net),
                        cmd if cmd.starts_with("ls rt") => p2p::handle_print_raw_txn(&swarm_public_net),
                        cmd if cmd.starts_with("create b") => public_block::handle_create_block(cmd, &mut swarm_public_net),
                        cmd if cmd.starts_with("create txn")=> pbft::pbft_pre_message_handler(cmd, &mut swarm_public_net),
                        _ => error!("unknown command"),  
                    },
                }
            }
        }
}