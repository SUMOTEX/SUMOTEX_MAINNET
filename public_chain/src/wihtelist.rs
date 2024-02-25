use std::sync::{Arc, Mutex};
use libp2p::{Swarm, PeerId, Multiaddr};

// Define a data structure to store whitelisted peers
#[derive(Default)]
struct WhitelistedPeers {
    peers: Arc<Mutex<Vec<Multiaddr>>>,
}

impl WhitelistedPeers {
    // Method to add a new whitelisted peer
    fn add_peer(&self, address: Multiaddr) {
        let mut peers = self.peers.lock().unwrap();
        peers.push(address);
    }

    // Method to get the current list of whitelisted peers
    fn get_peers(&self) -> Vec<Multiaddr> {
        let peers = self.peers.lock().unwrap();
        peers.clone()
    }
}
