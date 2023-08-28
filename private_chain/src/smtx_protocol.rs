use tokio::io::{AsyncReadExt, AsyncWriteExt};
use libp2p::{core::upgrade::UpgradeInfo, identity::Keypair, PeerId};
use std::io;
use libp2p::core::{Transport,multiaddr::Multiaddr};
use libp2p::tcp::TcpConfig;
use libp2p::swarm::{Swarm, SwarmBuilder};
use futures::StreamExt;

#[derive(Clone)]
pub struct SMTXProtocol;

#[derive(Debug)]
pub enum SMTXProtocolEvent {
    MessageReceived(PeerId, String),
    // Other possible events...
}

impl UpgradeInfo for SMTXProtocol {
    type Info = ();
    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        std::iter::once(())
    }
}

impl SMTXProtocol {
    pub fn new() -> Self {
        SMTXProtocol
    }
}

impl SMTXProtocol {
    pub async fn send_message(&mut self, socket: &mut Negotiated<TcpStream>, message: &str) -> Result<(), io::Error> {
        socket.write_all(message.as_bytes()).await?;
        Ok(())
    }
}


fn peer_id_to_socket_addr(peer_id: &PeerId) -> Option<Multiaddr> {
    // Logic to convert a PeerId to a Multiaddr
    // For example: "/ip4/127.0.0.1/tcp/12345".parse().ok()
    unimplemented!()
}
