use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use crate::private_p2p::PrivateAppBehaviour;
use std::sync::{Arc, Mutex};
use serde_json::json;
use libp2p::swarm::Swarm;
use crate::private_app::PrivateApp;
use crate::private_block;
type MySwarm = Swarm<PrivateAppBehaviour>;

#[derive(Debug,Clone)]
pub struct PrivateAccBlock {
    pub root_account:String,
    pub blocks: Vec<private_block::PrivateBlock>,
}
// Global states
lazy_static::lazy_static! {
    static ref APP_PRIVATE_BLOCKS: Arc<Mutex<PrivateAccBlock>> = Arc::new(Mutex::new(
        PrivateAccBlock {
            root_account:"".to_string(),
            blocks: vec![] }));
}
pub fn add_genesis_blocks(account:String,app:PrivateApp)-> impl Responder {
    let new_blocks = app.get_blocks();
    let mut app_blocks = APP_PRIVATE_BLOCKS.lock().unwrap();
    app_blocks.blocks = new_blocks.clone();
    app_blocks.root_account=account;
    HttpResponse::Ok()
    .header("Access-Control-Allow-Origin", "*")
    .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE")
    .header("Access-Control-Allow-Headers", "Authorization, Content-Type")
    .body("Ok")
}
pub fn add_api_blocks(app: PrivateApp) -> impl Responder {
    let new_blocks = app.get_blocks();
    let mut app_blocks = APP_PRIVATE_BLOCKS.lock().unwrap();
    app_blocks.blocks = new_blocks.clone();
    let json_response = json!(new_blocks);
    HttpResponse::Ok()
    .header("Access-Control-Allow-Origin", "*")
    .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE")
    .header("Access-Control-Allow-Headers", "Authorization, Content-Type")
    .body(json_response)
}

async fn obtain_blocks() -> impl Responder {
    let local_blocks = (APP_PRIVATE_BLOCKS).lock().unwrap();
    HttpResponse::Ok().json(&local_blocks.blocks)
}


#[actix_web::main]
pub async fn private_api() -> std::io::Result<()> {
    // println!("{:?}",PubApp.blocks);
    HttpServer::new(|| {
        
        App::new()
            .route("/blocks/private", web::get().to(obtain_blocks))
    })
    .bind("0.0.0.0:8001")?
    .run()
    .await
}
