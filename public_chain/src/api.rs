use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use crate::p2p::AppBehaviour;
use std::sync::{Arc, Mutex};
use libp2p::swarm::Swarm;
use crate::public_app::App as PubApp;
use crate::public_block::Block;
use actix_cors::Cors;
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
    HttpServer::new(|| {
        let cors = Cors::default()
        .allowed_origin("*")
        .allowed_methods(vec!["GET", "POST"])
        .allowed_headers(vec![actix_web::http::header::AUTHORIZATION, actix_web::http::header::ACCEPT])
        .allowed_header(actix_web::http::header::CONTENT_TYPE)
        .max_age(3600);
        
        App::new()
            .wrap(cors)
            .route("/blocks", web::get().to(obtain_blocks))
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
