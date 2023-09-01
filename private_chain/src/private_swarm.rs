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
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{
    Kademlia
};
use libp2p::Transport;
use log::{ info};
use crate::private_p2p::PEER_ID;
use crate::private_p2p::PrivateAppBehaviour;
use crate::private_p2p::KEYS;
use crate::account_root::AccountRoot;
use crate::private_transactions::Txn;
use crate::private_pbft::PrivatePBFTNode as PBFTNode;
use crate::private_app::PrivateApp;
type MySwarm = Swarm<PrivateAppBehaviour>;

pub async fn create_private_swarm(private_tx: mpsc::UnboundedSender<String>) ->  MySwarm {
    // Create and initialize your swarm here
    info!("Private Peer Id: {}", PEER_ID.clone());
    let (response_sender, _response_rcv) = mpsc::unbounded_channel();
    let (init_sender,  _init_rcv) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can create auth keys");
    // Convert to AuthenticKeypair
    let store = MemoryStore::new(PEER_ID.clone());
    let kademlia = Kademlia::new(PEER_ID.clone(), store);
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
    let private_behaviour = PrivateAppBehaviour::new(
            PrivateApp::new(),
            Txn::new(),
            PBFTNode::new(PEER_ID.clone().to_string()),
            AccountRoot::new(),
            kademlia,
            response_sender, 
            init_sender.clone(),
            private_tx).await;
    let swarm = SwarmBuilder::new(transp, private_behaviour, *PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();   
    swarm

}

