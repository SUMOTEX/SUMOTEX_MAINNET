use libp2p::identity::Keypair;
use std::fs::File;
use std::io::prelude::*;
use libp2p::{identity, PeerId, SwarmBuilder, Transport};

fn generate_keypair() -> Keypair {
    Keypair::generate_ed25519()
}
// Call this function and store the generated keypair.


fn save_key_to_file(key: &Keypair, path: &str) -> std::io::Result<()> {
    let key_bytes = key.encode();
    let mut file = File::create(path)?;
    file.write_all(&key_bytes)
}

fn load_key_from_file(path: &str) -> std::io::Result<Keypair> {
    let mut file = File::open(path)?;
    let mut key_bytes = Vec::new();
    file.read_to_end(&mut key_bytes)?;
    Keypair::decode(&key_bytes).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to decode key"))
}

fn start_node_with_key(key: Keypair) {
    let local_peer_id = PeerId::from(key.public());

    // Configure transport, swarm, etc. for the node using the loaded key
    let transport = libp2p::development_transport(local_peer_id.clone()).unwrap();
    
    // ... (Any other configurations like protocols, behaviours, etc.)

    let swarm = SwarmBuilder::new(transport, local_peer_id)
        .executor(Box::new(|fut| {
            async_std::task::spawn(fut);
        }))
        .build();

    // ... (Start async runtime and poll swarm, etc.)
}
