use libp2p::{
    core::upgrade,
    noise::{Keypair, NoiseConfig, X25519Spec},
    mplex,
    identity::{self, ed25519},
    tcp::TokioTcpConfig,
    swarm::{Swarm,SwarmBuilder},
};
use libp2p::kad::{Kademlia, KademliaConfig};
use libp2p::kad::store::MemoryStore;
use libp2p::{
    core::multiaddr::{Protocol},
};
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
    let local_peer_id_net1 = PeerId::from(key_public_net.public());

    // Setting up Kademlia
    let store = MemoryStore::new(*PEER_ID);
    let mut cfg = KademliaConfig::default();
    // Example: Add custom Kademlia configuration here
    let kademlia = Kademlia::with_config(*PEER_ID, store, cfg);

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
    let behaviour = AppBehaviour::new(app.clone(),Txn::new(),PBFTNode::new(PEER_ID.clone().to_string()),storage,kademlia, response_sender, init_sender.clone(),).await;
    println!("PEER_ID: {:?}",PEER_ID);
    let swarm = SwarmBuilder::new(transp, behaviour, *PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();
    set_global_swarm_public_net(swarm);
    //swarm

}

pub async fn setup_node(listen_addr: &str, peer_addr: Option<String>, swarm: &mut Swarm<AppBehaviour>) -> Result<(), Box<dyn Error>> {
    println!("Received listen address: {}", listen_addr);
    let listen_multiaddr = Multiaddr::from_str(listen_addr)
        .map_err(|e| {
            eprintln!("Error parsing listen address {}: {}", listen_addr, e);
            Box::new(e) as Box<dyn Error>
        })?;

    match swarm.listen_on(listen_multiaddr.clone()) {
        Ok(_) => println!("Listening on {}", listen_multiaddr),
        Err(e) => return Err(Box::new(e) as Box<dyn Error>),
    }

    if let Some(addr) = peer_addr {
        println!("Received peer address: {}", addr);
        let peer_multiaddr = Multiaddr::from_str(&addr)
            .map_err(|e| {
                eprintln!("Error parsing peer address {}: {}", addr, e);
                Box::new(e) as Box<dyn Error>
            })?;

        let peer_id = peer_multiaddr.iter().find_map(|protocol| match protocol {
            Protocol::P2p(hash) => PeerId::from_multihash(hash).ok(),
            _ => None,
        }).ok_or_else(|| {
            eprintln!("No PeerId found in Multiaddr.");
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No PeerId found in Multiaddr.")) as Box<dyn Error>
        })?;

        if let Err(e) = swarm.dial(peer_multiaddr.clone()) {
            eprintln!("Error dialing peer address {}: {}", peer_multiaddr, e);
        }

        swarm.behaviour_mut().kademlia.add_address(&peer_id, peer_multiaddr.clone());
        println!("Dialing peer {} and added to Kademlia", peer_id);
        // let kademlia = &swarm.behaviour().kademlia;
        // let routing_table = kademlia.routing_table();

        // for bucket in routing_table.buckets() {
        //     for entry in bucket.iter() {
        //         println!("Peer in bucket: {:?}", entry.node.key.preimage());
        //     }
        // }
    }
    Ok(())
}
