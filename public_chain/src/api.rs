use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use crate::p2p::AppBehaviour;
use std::sync::{Arc, Mutex};
use serde_json::json;
use libp2p::swarm::Swarm;
use crate::public_app::App as PubApp;
use crate::public_block::Block;
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
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
