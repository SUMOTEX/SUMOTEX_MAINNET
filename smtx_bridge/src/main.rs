// main.rs

use libp2p::{
    tcp::TokioTcpConfig,
    swarm::{SwarmBuilder},

};
use tokio::{
    select
};
use libp2p::futures::StreamExt;
extern crate public_chain;  // Assume the crate exposes a "public_swarm()" function
extern crate private_chain; // Assume the crate exposes a "private_swarm()" function
use crate::public_chain::public_swarm::create_public_swarm;
use crate::private_chain::private_swarm::create_private_swarm;
use tokio::sync::mpsc;

fn is_event_type_y(event: &str) -> bool {
    // Here, I'm assuming that the event type can be determined by the event string.
    // Replace this with your actual logic for determining the event type.
    event == "Private Node"
}

#[tokio::main]
async fn main()-> Result<(), Box<dyn std::error::Error>> {
    let mut public_swarm = create_public_swarm().await;
    let mut private_swarm = create_private_swarm().await;
    
    // Create channels for custom events
    let (public_tx, mut public_rx): (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>) = mpsc::unbounded_channel();
    let (private_tx, mut private_rx): (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>) = mpsc::unbounded_channel();

    loop {
        select! {
            // Handle private_swarm events
            _event = private_swarm.select_next_some() => {
                //println!("HERE");
                // if is_event_type_y("Private Node") {
                //     println!("Event with another keyword received in private rx");
                //     // do something...
                // }
            },
            // Handle public_swarm events
            _event = public_swarm.select_next_some() => {
                // if is_event_type_y("Private Node") {
                //     println!("Event with another keyword received in private rx");
                //     // do something...
                // }
                //public_tx.send("Event for public rx".to_string()).unwrap();
                // Handle the event here.
                // For example, you can send a message to the private_swarm
                // if some condition is met in the public_swarm
                // private_tx.send(your_message);
            },
            // Handle events from public_rx
            Some(event_string) = public_rx.recv() => {
                // Handle the event here.
            },
            // Handle events from private_rx
            Some(event_string) = private_rx.recv() => {
                if event_string.contains("add_private_block") {
                    println!("Event with keyword received in private rx");
                    // do something...
                }
                // Handle the event here.
            },
            else => {
                // Do something if none of the above are ready.
            }
        }
    }
}
