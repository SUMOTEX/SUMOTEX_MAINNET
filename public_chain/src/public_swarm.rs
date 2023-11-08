use libp2p::{
    core::upgrade,
    noise::{Keypair, NoiseConfig, X25519Spec},
    mplex,
    tcp::TokioTcpConfig,
    swarm::{Swarm,SwarmBuilder},
};
use tokio::{
    sync::mpsc,
     spawn,
};

use libp2p::Transport;
use log::{ info};
use std::sync::Once;
use once_cell::sync::OnceCell;
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

use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

lazy_static! {
    static ref GLOBAL_SWARM_PUBLIC_NET: Arc<Mutex<Option<Swarm<AppBehaviour>>>> = Arc::new(Mutex::new(None));
}

pub fn set_global_swarm_public_net(swarm: Swarm<AppBehaviour>) {
    let mut global_swarm = GLOBAL_SWARM_PUBLIC_NET.lock().unwrap();
    *global_swarm = Some(swarm);
}

pub fn get_global_swarm_public_net() -> Arc<Mutex<Option<Swarm<AppBehaviour>>>> {
    Arc::clone(&GLOBAL_SWARM_PUBLIC_NET)
}


pub async fn create_public_swarm(app: App,storage:StoragePath) {
    // Create and initialize your swarm here
    info!("Peer Id: {}", PEER_ID.clone());
    let (response_sender, _response_rcv) = mpsc::unbounded_channel();
    let (init_sender,  _init_rcv) = mpsc::unbounded_channel();
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can create auth keys");
    // Convert to AuthenticKeypair
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

