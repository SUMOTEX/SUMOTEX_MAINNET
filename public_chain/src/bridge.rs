
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self,AsyncReadExt, AsyncWriteExt};
use crate::publisher::Publisher;
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub title: String,
    pub hash: String,
    pub root_account:Option<String>
}
pub async fn tcp_client(message:Message) -> Result<(),io::Error> {
    let stream = TcpStream::connect("127.0.0.1:8090").await?;
    let (mut reader, mut writer) = io::split(stream);

    // Serialize the Message struct to a JSON string
    let message_string = serde_json::to_string(&message)?;

    // Use the writer for writing
    writer.write_all(message_string.as_bytes()).await?;

    // Use the reader for reading
    let mut buf = vec![0u8; 1024];  // Change buffer size as needed
    let bytes_read = reader.read(&mut buf).await?;

    // Optionally convert the received bytes back to a string
    let _ = String::from_utf8_lossy(&buf[0..bytes_read]);
    // if message.title=="GENESIS"{
    //     println!("Received: {:?}", message);
    //     if let Some(publisher) = Publisher::get(){
    //         publisher.publish("private_blocks_genesis_creation".to_string(), message.hash);
            
    //     }
    // }
    return Ok(());

}

pub async fn handle_client(mut socket: TcpStream) {
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
                obtain_message(received_string.to_string());
                println!("Received private chain request: {}", received_string);
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
}

pub async fn accept_loop(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((socket, _addr)) => {
                tokio::spawn(handle_client(socket));
            }
            Err(e) => eprintln!("Failed to accept socket: {}", e),
        }
    }
}

pub fn obtain_message(r_message:String){
    let deserialized_message = match serde_json::from_str::<Message>(&r_message) {
        Ok(mut message) => {
            println!("Message: {:?}", message);
            if message.title=="GENESIS"{
                println!("Received GENESIS: {:?}", message);
                if let Some(publisher) = Publisher::get(){
                    publisher.publish("private_blocks_genesis_creation".to_string(), message.hash.clone());
                    
                }
            }else if message.title=="TXN_BLOCK" {
                println!("Received Transactions Block: {:?}", message);
                if let Some(publisher) = Publisher::get(){
                    publisher.publish("hybrid_block_creation".to_string(), message.hash.clone());
                    
                }
            }
            message
        },
        Err(e) => {
            eprintln!("Failed to parse JSON: {}", e);
            return; // Or handle the error in another way
        }
    };
}