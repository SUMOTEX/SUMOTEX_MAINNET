use super::{App,Txn,pbft::PBFTNode};
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
use crate::private_p2p::PEER_ID;
use crate::private_p2p::PrivateAppBehaviour;
use crate::private_p2p::KEYS;
use crate::PrivateApp;
type MySwarm = Swarm<PrivateAppBehaviour>;

pub async fn  create_swarm() -> MySwarm {
    // Create and initialize your swarm here
    info!("Private Peer Id: {}", PEER_ID.clone());
    let (response_sender, _response_rcv) = mpsc::unbounded_channel();
    let (init_sender,  _init_rcv) = mpsc::unbounded_channel();
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can create auth keys");
    // Convert to AuthenticKeypair
    
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
    let private_behaviour = PrivateAppBehaviour::new(PrivateApp::new(),Txn::new(),PBFTNode::new(PEER_ID.clone().to_string()),response_sender, init_sender.clone()).await;
    let swarm = SwarmBuilder::new(transp, private_behaviour, *PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();
    swarm

}

