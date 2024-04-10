use libp2p::{
    core::upgrade,
    identify::{Identify, IdentifyConfig},
    noise::{Keypair, NoiseConfig, X25519Spec},
    mplex,
    identity::{self, ed25519},
    tcp::TokioTcpConfig,
    swarm::{Swarm,SwarmBuilder},
};
use std::borrow::Cow;
use std::collections::HashSet;
use libp2p::gossipsub::{Gossipsub, GossipsubConfig, GossipsubEvent, IdentTopic as Topic, MessageAuthenticity};
use once_cell::sync::Lazy;
use libp2p::kad::{Kademlia, KademliaConfig};
use libp2p::kad::store::MemoryStore;
use libp2p::{
    core::multiaddr::{Protocol},
};
use log::warn;
use libp2p::Multiaddr;
use std::str::FromStr;
use tokio::{
    sync::mpsc,
     spawn,
};
use std::{fs, io,path::Path,error::Error};
use libp2p::Transport;
use log::{ info};
use crate::p2p::PEER_ID;
use crate::p2p::AppBehaviour;
use crate::p2p::KEYS;
use crate::public_app::App;
use crate::public_txn::Txn;
use crate::public_block;
use crate::pbft::PBFTNode;
use crate::rock_storage::StoragePath;
use libp2p::PeerId;
use libp2p::identity::{Keypair as IdentityKeypair};
type MySwarm = Swarm<AppBehaviour>;
use log::error;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;


lazy_static! {
    static ref GLOBAL_SWARM_PUBLIC_NET: Arc<Mutex<Option<Swarm<AppBehaviour>>>> = Arc::new(Mutex::new(None));
}

// lazy_static! {
//     static ref GLOBAL_SWARM_PUBLIC_NET: Arc<Mutex<Option<Swarm<AppBehaviour>>>> = Arc::new(Mutex::new(None));
// }

pub fn set_global_swarm_public_net(swarm: Swarm<AppBehaviour>) {
    let mut global_swarm = GLOBAL_SWARM_PUBLIC_NET.lock().unwrap();
    *global_swarm = Some(swarm);
}

pub fn get_global_swarm_public_net() -> Arc<Mutex<Option<Swarm<AppBehaviour>>>> {
    Arc::clone(&GLOBAL_SWARM_PUBLIC_NET)
}

fn generate_and_save_key_if_not_exists(file_path: &str) ->  Result<(), Box<dyn Error>> {
    // Check if the key file already exists
    if Path::new(file_path).exists() {
        // Read the secret key from the file
        // let secret_key_bytes = fs::read(file_path)?;
        // let secret_key = identity::ed25519::SecretKey::from_bytes(&mut secret_key_bytes)
        //     .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Unable to decode the secret key"))?;
        // let public_key = ed25519::PublicKey::from(&secret_key);
        // // Construct an Ed25519 keypair
        // let keypair = ed25519::Keypair { secret: secret_key, public: public_key };
        // Ok(IdentityKeypair::Ed25519(keypair))
    } else {
        // Generate a new Ed25519 keypair
        let key = IdentityKeypair::generate_ed25519();
        if let IdentityKeypair::Ed25519(ed_key) = &key {
            // Extract the secret key bytes
            let secret = ed_key.secret();
            // Now you can take a reference to `secret` without it being dropped
            let secret_key_bytes = secret.as_ref();
            // Write the secret key bytes to the file
            fs::write(file_path, secret_key_bytes)?;
        }
    }
    return Ok(());
}

pub async fn create_public_swarm(app: App,storage:StoragePath) {
    // Create and initialize your swarm hereq
    let (response_sender, _response_rcv) = mpsc::unbounded_channel();
    let (init_sender,  _init_rcv) = mpsc::unbounded_channel();
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can create auth keys");
    // Convert to AuthenticKeypair
    let _keypair = generate_and_save_key_if_not_exists("/Users/leowyennhan/Desktop/sumotex_mainnet/chain/public_chain/key_storage");
    let key_public_net = IdentityKeypair::generate_ed25519();

    // Setting up Kademlia
    let store = MemoryStore::new(*PEER_ID);
    let mut cfg = KademliaConfig::default();
    cfg.set_protocol_name(Cow::Borrowed("/sumotex/kad/1.0.0".as_bytes()));
    // Example: Add custom Kademlia configuration here
    let kademlia = Kademlia::with_config(*PEER_ID, store, cfg);

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
    let behaviour = AppBehaviour::new(app.clone(),
                Txn::new(),
                PBFTNode::new(PEER_ID.clone().to_string()),
                storage,
                kademlia,
                response_sender,
                init_sender.clone(),).await;
    println!("PEER_ID: {:?}",PEER_ID);
    let mut swarm = SwarmBuilder::new(transp, behaviour, *PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();
    set_global_swarm_public_net(swarm);
    //swarm

}

pub async fn setup_node(
    listen_addr: &str,
    peer_addr: Option<String>,
    swarm: &mut Swarm<AppBehaviour>,
) -> Result<(), Box<dyn Error>> {
    info!("Setting up node with listen address: {}", listen_addr);

    let listen_multiaddr = Multiaddr::from_str(listen_addr);
    match listen_multiaddr {
        Ok(addr) => {
            if let Some(addr_str) = peer_addr {
                match PeerId::from_str(&addr_str) {
                    Ok(peer_id) => {
                        info!("Adding known peer {} with address {}", peer_id, addr);
                        swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                        info!("Subscribing to topics.");
                        static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
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
                        for topic in vec![
                            CHAIN_TOPIC.clone(),
                            public_block::BLOCK_TOPIC.clone(),
                            BLOCK_PBFT_PREPREPARED_TOPIC.clone(),
                            BLOCK_PBFT_PREPARED_TOPIC.clone(),
                            BLOCK_PBFT_COMMIT_TOPIC.clone(),
                            TXN_PBFT_PREPARED_TOPIC.clone(),
                            TXN_PBFT_COMMIT_TOPIC.clone(),
                            PRIVATE_BLOCK_GENESIS_CREATION.clone(),
                            HYBRID_BLOCK_CREATION.clone(),
                            ACCOUNT_CREATION_TOPIC.clone()
                            // Add other topics as in your original code
                        ] {
                            match swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                                Ok(subscribed) => {
                                    if subscribed {
                                        info!("Successfully subscribed to topic: {:?}", topic);
                                    } else {
                                       
                                        warn!("Already subscribed to topic: {:?}", topic);
                                    }
                                },
                                Err(e) => {
                                    error!("Failed to subscribe to topic {:?}: {:?}", topic, e);
                                    return Err(e.into());
                                }
                            }
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                        println!("Successfully added explicit peer: {:?}", peer_id);
                        info!("Attempting to dial peer at {}", addr);
                        if !swarm.is_connected(&peer_id) {
                            swarm.dial(peer_id)?;
                        }
                        if let Err(e) = swarm.dial(addr.clone()) {
                            error!("Failed to dial {}: {:?}", addr, e);
                            return Err(e.into());
                        }
                    },
                    Err(e) => {
                        error!("Invalid peer address {}: {:?}", addr_str, e);
                        return Err(e.into());
                    }
                }
            }

            info!("Listening on {}", addr);
            if let Err(e) = Swarm::listen_on(swarm, addr.clone()) {
                error!("Failed to listen on {}: {:?}", addr, e);
                return Err(e.into());
            }
        },
        Err(e) => {
            error!("Error parsing Multiaddr {}: {:?}", listen_addr, e);
            return Err(e.into());
        }
    }

    info!("Bootstrapping Kademlia DHT.");
    if let Err(e) = swarm.behaviour_mut().kademlia.bootstrap() {
        warn!("Kademlia bootstrap failed: {}", e);
    }
    Ok(())
}

pub fn check_connected_peers(swarm: &mut Swarm<AppBehaviour>){


    // Extract Kademlia known peers
    let kademlia = &mut swarm.behaviour_mut().kademlia;
    for bucket in kademlia.kbuckets() {
        for entry in bucket.iter() {
            let peer_id = entry.node.key.preimage();
            println!("Known peer in Kademlia: {:?}", peer_id);
        }
    }

    // Extract Gossipsub connected peers
    let gossipsub = &mut swarm.behaviour_mut().gossipsub;
    for peer_id in gossipsub.all_peers() {
        println!("Connected to peer in Gossipsub: {:?}", peer_id);
    }

}