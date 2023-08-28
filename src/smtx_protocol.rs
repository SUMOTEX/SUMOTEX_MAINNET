use tokio::io::{AsyncReadExt, AsyncWriteExt};
use libp2p::{core::upgrade::UpgradeInfo, identity::Keypair, PeerId};
use std::io;
use libp2p::core::{Transport, identity::Keypair, multiaddr::Multiaddr};
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
    pub async fn connected_socket_mut(&mut self, peer_id: &PeerId) -> Option<Negotiated<TcpStream>> {
        if let Some(addr) = peer_id_to_socket_addr(peer_id) {
            match TcpConfig::new().connect(addr) {
                Ok(socket) => {
                    let transport = TcpConfig::new();
                    let negotiated = transport.negotiate(socket).await.ok()?;
                    Some(negotiated)
                }
                Err(_) => None,
            }
        } else {
            None
        }
    }
    pub async fn send_message(&mut self, socket: &mut Negotiated<TcpStream>, message: &str) -> Result<(), io::Error> {
        socket.write_all(message.as_bytes()).await?;
        Ok(())
    }
}

async fn main() {
    // Initialize your SMTXProtocol instance
    let smtx_protocol = SMTXProtocol::new();

    // Obtain the peer_id
    let remote_peer_id = ...;

    if let Some(mut connected_socket) = smtx_protocol.connected_socket_mut(&remote_peer_id).await {
        if let Err(err) = smtx_protocol.send_message(&mut connected_socket, "Hello, remote peer!").await {
            eprintln!("Failed to send message: {:?}", err);
        }
    }
}

fn peer_id_to_socket_addr(peer_id: &PeerId) -> Option<Multiaddr> {
    // Logic to convert a PeerId to a Multiaddr
    // For example: "/ip4/127.0.0.1/tcp/12345".parse().ok()
    unimplemented!()
}
