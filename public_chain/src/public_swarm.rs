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
use std::sync::{Arc, Mutex};
use log::{ info};
use once_cell::sync::Lazy;
use std::sync::Once;
use once_cell::sync::OnceCell;
use crate::p2p::PEER_ID;
use crate::p2p::AppBehaviour;
use crate::p2p::KEYS;
use crate::public_app::App;
use crate::public_txn::Txn;
use crate::pbft::PBFTNode;

use libp2p::PeerId;
use libp2p::identity::{Keypair as IdentityKeypair};
type MySwarm = Swarm<AppBehaviour>;

static GLOBAL_APP_BEHAVIOUR: OnceCell<Arc<Mutex<AppBehaviour>>> = OnceCell::new();

pub async fn init_global_app_behaviour() -> Arc<Mutex<AppBehaviour>> {
    // Try to get the global instance
    if let Some(global_instance) = GLOBAL_APP_BEHAVIOUR.get() {
        return global_instance.clone();
    }
    // If it doesn't exist, create one
    let (response_sender, _response_rcv) = mpsc::unbounded_channel();
    let (init_sender,  _init_rcv) = mpsc::unbounded_channel();

    let app_behaviour = AppBehaviour::new(
        App::new(),
        Txn::new(),
        PBFTNode::new(PEER_ID.clone().to_string()),
        response_sender,
        init_sender.clone()
    ).await;

    let arc_app_behaviour = Arc::new(Mutex::new(app_behaviour));

    // Set the global instance
    GLOBAL_APP_BEHAVIOUR.set(arc_app_behaviour.clone()).unwrap_or_else(|_| {
        // If set fails, another thread has set it in the meantime.
        // This is fine; we'll just use that.
    });

    arc_app_behaviour
}
pub async fn create_public_swarm(app: App) -> MySwarm {
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
    let behaviour = AppBehaviour::new(app.clone(),Txn::new(),PBFTNode::new(PEER_ID.clone().to_string()), response_sender, init_sender.clone()).await;

    let swarm = SwarmBuilder::new(transp, behaviour, *PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();
    swarm

}

