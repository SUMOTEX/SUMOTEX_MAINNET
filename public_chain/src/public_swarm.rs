use libp2p::{
    core::upgrade,
    noise::{Keypair, NoiseConfig, X25519Spec},
    mplex,
    identity::{self, ed25519},
    tcp::TokioTcpConfig,
    swarm::{Swarm,SwarmBuilder},
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

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
    let behaviour = AppBehaviour::new(app.clone(),Txn::new(),PBFTNode::new(PEER_ID.clone().to_string()),storage, response_sender, init_sender.clone()).await;

    let swarm = SwarmBuilder::new(transp, behaviour, *PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();
    set_global_swarm_public_net(swarm);
    //swarm

}

pub async fn add_listener(addr: String, swarm: &mut Swarm<AppBehaviour>) -> Result<(), Box<dyn Error>> {
    info!("Attempting to add listener on address: {}", addr);

    let the_address = Multiaddr::from_str(&addr)
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;
    
    // Directly use the mutable reference to `Swarm<AppBehaviour>` for adding a listener
    match Swarm::listen_on(swarm, the_address.clone()) {
        Ok(_) => {
            println!("Listening on {:?}", the_address);
            Ok(())
        },
        Err(e) => Err(Box::new(e) as Box<dyn Error>),
    }
}