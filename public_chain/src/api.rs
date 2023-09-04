use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use crate::p2p::AppBehaviour;
use std::sync::{Arc, Mutex};
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
    static ref APP_BLOCKS: Arc<Mutex<AppBlocks>> = Arc::new(Mutex::new(AppBlocks { blocks: vec![] }));
}

pub fn add_api_blocks(app: PubApp) -> impl Responder {
    let new_blocks = app.get_blocks();
    let mut app_blocks = APP_BLOCKS.lock().unwrap();
    app_blocks.blocks = new_blocks.clone();

    HttpResponse::Ok().json(new_blocks)
}

async fn obtain_blocks() -> impl Responder {
    let local_blocks = APP_BLOCKS.lock().unwrap();
    HttpResponse::Ok().json(&local_blocks.blocks)
}


#[actix_web::main]
pub async fn pub_api() -> std::io::Result<()> {
    // println!("{:?}",PubApp.blocks);
    HttpServer::new(move || {
        App::new()
            .route("/blocks", web::get().to(obtain_blocks))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
