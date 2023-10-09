use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use crate::p2p::AppBehaviour;
use std::sync::{Arc, Mutex};
use serde_json::json;
use libp2p::swarm::Swarm;
use crate::public_app::App as PubApp;
use crate::public_block::Block;
use crate::p2p;
type MySwarm = Swarm<AppBehaviour>;

#[derive(Debug,Clone)]
pub struct AppBlocks {
    pub blocks: Vec<Block>,
}
// Global shared state
lazy_static::lazy_static! {
    static ref APP_BLOCKS: Arc<Mutex<AppBlocks>> = Arc::new(Mutex::new(
        AppBlocks { 
            blocks: vec![] }));
}

// Endpoint to print peers
// async fn print_peers() -> impl Responder {
//     let swarm_public_net = "" // TODO: fetch or get access to your swarm
//     p2p::handle_print_peers(&swarm_public_net);
//     HttpResponse::Ok().finish()
// }

// // Endpoint to print the chain
// async fn print_chain() -> impl Responder {
//     let swarm_public_net = "" // TODO: fetch or get access to your swarm
//     p2p::handle_print_chain(swarm_public_net);
//     HttpResponse::Ok().finish()
// }

// // Endpoint to create a block
// async fn create_block() -> impl Responder {
//     let swarm_public_net = "" // TODO: fetch or get access to your swarm
//     // Note: You might want to get more details to create a block via request parameters or body.
//     public_block::handle_create_block("", swarm_public_net);
//     HttpResponse::Ok().finish()
// }


pub fn add_api_blocks(app: PubApp) -> impl Responder {
    let new_blocks = app.get_blocks();
    let mut app_blocks = APP_BLOCKS.lock().unwrap();
    app_blocks.blocks = new_blocks.clone();
    let json_response = json!(new_blocks);
    HttpResponse::Ok()
    .header("Access-Control-Allow-Origin", "*")
    .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE")
    .header("Access-Control-Allow-Headers", "Authorization, Content-Type")
    .body(json_response)
}

async fn obtain_blocks() -> impl Responder {
    let local_blocks = APP_BLOCKS.lock().unwrap();
    HttpResponse::Ok().json(&local_blocks.blocks)
}


#[actix_web::main]
pub async fn pub_api() -> std::io::Result<()> {
    // println!("{:?}",PubApp.blocks);
    HttpServer::new(|| {
        App::new()
            .route("/blocks", web::get().to(obtain_blocks))
            // .route("/print-peers", web::post().to(print_peers))
            // .route("/print-chain", web::post().to(print_chain))
            // .route("/create-block", web::post().to(create_block))
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
