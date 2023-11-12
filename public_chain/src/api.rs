use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use crate::p2p::AppBehaviour;
use std::sync::{Arc, Mutex};
use serde_json::json;
use libp2p::swarm::Swarm;
use log::error;
use crate::public_app::App as PubApp;
use crate::public_block::Block;
use crate::smart_contract;
use crate::public_swarm;
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
pub async fn create_nft_contract(info: web::Json<(String, String)>)-> impl Responder{
    let (call_address, private_key) = info.into_inner();
    let swarm_mutex = public_swarm::get_global_swarm_public_net();
    let mut swarm_public_net_guard = match swarm_mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            error!("Mutex is poisoned. Aborting block creation.");
            return HttpResponse::InternalServerError().finish()
        },
    };
    if let Some(swarm_public_net) = &mut *swarm_public_net_guard {
        // If create_erc721_contract_official returns a Result, handle it accordingly
        match smart_contract::create_erc721_contract_official(&call_address, &private_key, swarm_public_net) {
            Ok(contract_address) => {
                // If the contract creation is successful, return the contract address
                let response_body = json!({"contract_address": contract_address});
                return HttpResponse::Ok().json(response_body)
            },
            Err(e) => {
                // Handle any errors that occur during contract creation
                error!("Error creating contract: {:?}", e);
                return HttpResponse::InternalServerError().finish()
            }
        }
    }else {
        error!("Swarm public net is not initialized.");
        return HttpResponse::InternalServerError().finish()
    }

}

async fn obtain_blocks() -> impl Responder {
    let local_blocks = APP_BLOCKS.lock().unwrap();
    HttpResponse::Ok().json(&local_blocks.blocks)
}


#[actix_web::main]
pub async fn pub_api() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/blocks", web::get().to(obtain_blocks))
            .route("/create-nft-contract", web::post().to(create_nft_contract))
            // .route("/print-peers", web::post().to(print_peers))
            // .route("/print-chain", web::post().to(print_chain))
            // .route("/create-block", web::post().to(create_block))
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
