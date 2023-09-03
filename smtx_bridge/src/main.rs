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
use tokio::net::TcpStream;
use tokio::net::TcpListener;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use std::io::Result;


#[tokio::main]
async fn main() ->tokio::io::Result<()> {
    // Create channels for custom events
    let (public_tx, mut _public_rx): (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>) = mpsc::unbounded_channel();
    let (private_tx, mut _private_rx): (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>) = mpsc::unbounded_channel();
    let mut public_swarm = create_public_swarm().await;
    let mut private_swarm = create_private_swarm(private_tx.clone()).await;
    let listener = TcpListener::bind("127.0.0.1:8090").await?;
    println!("Listening on 127.0.0.1:8090...");
    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            // In a loop, read data from the socket and write the data back.
            loop {
                match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => {
                        println!("Connection closed.");
                        return;
                    },
                    Ok(n) => {
                        println!("Received {} bytes: {:?}", n, &buf[0..n]);
                        let received_string = String::from_utf8_lossy(&buf[0..n]);
                        println!("Received string: {}", received_string);
                        if socket.write_all(&buf[0..n]).await.is_err() {
                            eprintln!("Failed to write data back to socket");
                            return;
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to read from socket: {}", e);
                        return;
                    }
                }
            }
        });
    }
    loop {
        select! {
            // Handle private_swarm events
            event = private_swarm.select_next_some() => {
                println!("Received private next event: {:?}", event);  // Fixed here: changed _event to event
            },
            // Handle public_swarm events
            event = public_swarm.select_next_some() => {
                //println!("Received public next event: {:?}", event);
            },
            // Handle events from public_rx
            Some(event) = _public_rx.recv() => {  // Fixed here: changed .rev() to .recv()
                //println!("Received public event: {:?}", event);
            },
            // Handle events from private_rx
            Some(event) = _private_rx.recv() => {
                println!("Received private event: {:?}", event);
            },
            else => {
                // Do something if none of the above are ready.
            }
        }
    }
}

