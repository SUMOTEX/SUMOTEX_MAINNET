use rocket::{post,get,options};
use rocket::routes;
use rocket::launch;
use rocket::serde::json::Json;
use serde_json::json;
use rocket::http::{Header, ContentType, Method,Status};
use rocket::{Request, Response};
use rocket::fairing::AdHoc;
use std::sync::{Arc, Mutex};
use rocket::fairing::{Fairing, Info, Kind};
use log::error;
use crate::smart_contract;
use crate::public_swarm;
use crate::account;
use crate::public_txn;
use crate::public_block;
use crate::public_block::Block;
use crate::public_txn::TransactionType;
use crate::public_app::App as PubApp;
use lazy_static::lazy_static;
use rocket::Route;
use secp256k1::SecretKey;
use rocket_okapi::{openapi, routes_with_openapi, JsonSchema};
use rocket_okapi::swagger_ui::{make_swagger_ui, SwaggerUIConfig};


#[derive(Debug,serde::Serialize, serde::Deserialize)]
struct TransactionInfo {
    caller_address: String,
    to_address: String,
    computed_value: u64,
    transaction_type: String
}
#[derive(Debug,serde::Serialize, serde::Deserialize)]
struct TransactionSignedInfo {
    caller_address: String,
    txn_hash:String,
    computed_value: u64,
    transaction_type: String,
    private_key:String
}
#[derive(serde::Deserialize, Debug)]
pub struct ContractInfo {
    call_address: String,
    private_key: String,
    contract_name:String,
    contract_symbol:String
}
#[derive(serde::Deserialize, Debug)]
pub struct MintTokenInfo {
    contract_address: String,
    caller_address: String,
    caller_private_key:String,
    ipfs_detail:String
}
#[derive(serde::Deserialize, Debug)]
pub struct ReadTokenInfo {  
    contract_address: String,
    token_id: i32,
}

#[derive(serde::Deserialize, Debug)]
pub struct ReadAccountInfo {
    pub_address: String
}

#[derive(serde::Deserialize, Debug)]
pub struct TransferTokenInfo {
    from_address: String,
    from_private_key:String,
    to_address:String,
    amount:f64
}
#[derive(serde::Deserialize, Debug)]
pub struct TxnIdInfo {
    txn_id: String,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SignedTransaction {
    transaction_hash: String,
    signature: String,
    // ... other fields as needed ...
}

#[derive( serde::Serialize, serde::Deserialize,Debug,Clone)]
pub struct AppBlocks {
    pub blocks: Vec<Block>,
}
// Global shared state
lazy_static::lazy_static! {
    static ref APP_BLOCKS: Arc<Mutex<AppBlocks>> = Arc::new(Mutex::new(
        AppBlocks { 
            blocks: vec![] }));
}

pub fn add_api_blocks(app: PubApp) ->  Json<serde_json::Value>  {
    let new_blocks = app.get_blocks();
    let mut app_blocks = APP_BLOCKS.lock().unwrap();
    app_blocks.blocks = new_blocks.clone();
    let json_response = json!(new_blocks);
    Json(json!({"jsonrpc": "1.0", "result": json_response}))
}

// Route to handle RPC requests for transaction creation
#[post("/create-transaction", data = "<transaction_data>")]
fn create_transaction(transaction_data: Json<TransactionInfo>) -> Json<serde_json::Value> {
    println!("Creating transaction");

    // Extracting data from request
    let caller_address = &transaction_data.caller_address;
    let to_address = &transaction_data.to_address;
    let computed_value = transaction_data.computed_value;
    let transaction_type = match transaction_data.transaction_type.as_str() {
        "SimpleTransfer" => TransactionType::SimpleTransfer,
        "ContractCreation" => TransactionType::ContractCreation,
        "ContractInteraction" => TransactionType::ContractInteraction,
        _ => return Json(json!({"jsonrpc": "2.0", "error": "Invalid transaction type"}))
    };
    match public_txn::Txn::create_and_prepare_transaction(
        transaction_type,
        caller_address.to_string(),
        to_address.to_string(),
        computed_value
    ) {
        Ok((txn_hash_hex,gas_cost, _)) => {
            println!("Transaction successfully prepared: {:?}", txn_hash_hex);
            Json(json!({
                "jsonrpc": "2.0", 
                "result": {
                    "transaction_hash": txn_hash_hex,
                    "gas_cost": gas_cost
                }
            }))
        },
        Err(e) => {
            println!("Error creating transaction: {:?}", e);
            Json(json!({
                "jsonrpc": "2.0", 
                "error": "Transaction creation failed"
            }))
        }
    }
}

#[post("/sign-transaction", data = "<transaction_signed_data>")]
fn sign_transaction(transaction_signed_data: Json<TransactionSignedInfo>) -> Json<serde_json::Value> {
    
    println!("Signing transaction");
    let caller_address = &transaction_signed_data.caller_address;
    // Attempt to decode the provided private key
    let private_key_bytes = match hex::decode(&transaction_signed_data.private_key) {
        Ok(bytes) => bytes,
        Err(_) => return Json(json!({"jsonrpc": "2.0", "error": "Invalid private key format"})),
    };

    // Attempt to create a SecretKey from the decoded bytes
    let private_key = match SecretKey::from_slice(&private_key_bytes) {
        Ok(key) => key,
        Err(_) => return Json(json!({"jsonrpc": "2.0", "error": "Invalid private key"})),
    };
    match public_txn::Txn::sign_and_submit_transaction(caller_address,transaction_signed_data.txn_hash.clone(),&private_key){
        Ok(()) => {
            Json(json!({
                "jsonrpc": "2.0", 
                "result": {
                }
            }))
        },
        Err(e) => {
            println!("Error signing transaction: {:?}", e);
            Json(json!({
                "jsonrpc": "2.0", 
                "error": "Transaction signed failed"
            }))
        }
    }
}
// Route to handle RPC requests.
#[post("/create-nft-contract", data = "<post_data>")]
fn create_nft_contract(post_data: Json<ContractInfo>)-> Json<serde_json::Value> {
    println!("create_nft_contract");
    let call_address = &post_data.call_address;
    let private_key = &post_data.private_key;
    let contract_name = &post_data.contract_name;
    let contract_symbol = &post_data.contract_symbol;
    match smart_contract::create_erc721_contract_official(&call_address, &private_key,contract_name,contract_symbol) {
        Ok((contract_address,txn_hash,gas_cost,body)) => {
            let response_body = json!({"contract_address": contract_address,
                                        "txn_hash":txn_hash,
                                        "gas_cost":gas_cost,
                                        });
            Json(json!({"jsonrpc": "1.0", "result": response_body}))
        },
        Err(e) => {
            error!("Error creating contract: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }
}

// Route to handle RPC requests.
#[post("/mint-token", data = "<post_data>")]
fn mint_token_contract(post_data: Json<MintTokenInfo>)-> Json<serde_json::Value> {
    println!("mint_token");
    let contract_address = &post_data.contract_address;
    let owner_address = &post_data.caller_address;
    let owner_private_key = &post_data.caller_private_key;
    let ipfs= &post_data.ipfs_detail;
    match smart_contract::mint_token_official(&contract_address, &owner_address,&owner_private_key,&ipfs) {
        Ok((token_id,txn_hash,gas_cost)) => {
            let response_body = json!({"token_id": token_id.to_string(),
                                        "txn_hash":txn_hash,
                                        "gas_cost":gas_cost});
            Json(json!({"jsonrpc": "1.0", "result": response_body}))
        },
        Err(e) => {
            error!("Error minting token: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "Error Minting Token"}))
        }
    }
}
// // Route to handle RPC requests.
#[post("/read-token-by-id", data = "<post_data>")]
fn read_token_contract(post_data: Json<ReadTokenInfo>)-> Json<serde_json::Value> {
    println!("Read Token By ID");
    let contract_address = &post_data.contract_address;
    let token_id = &post_data.token_id;
    match smart_contract::read_token_by_id(&contract_address, token_id) {
        Ok(token_detail) => {
            println!("Read Token Details: {:?}", token_detail);
            let response_body = json!({"token_detail": token_detail});
            Json(json!({"jsonrpc": "1.0",  "result": response_body}))
        },
        Err(e) => {
            error!("Error creating contract: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }
}

// Route to handle RPC requests.
#[post("/create-wallet")]
fn create_wallet()-> Json<serde_json::Value> {
    match account::create_account() {
        Ok((wallet_address, private_key)) => {
            let response_body = json!({
                "wallet_address": wallet_address,
                "private_key": private_key.to_string(), // Be cautious with private key handling
            });
            Json(json!({"jsonrpc": "1.0", "result": response_body}))
        },
        Err(e) => {
            error!("Error creating wallet: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }
}
//All wallet related
#[post("/check-account",data="<post_data>")]
fn get_account(post_data: Json<ReadAccountInfo>)->Json<serde_json::Value>{
    let pub_add = &post_data.pub_address;
    match account::account_exists(pub_add) {
        Ok(true) =>{
            Json(json!({"jsonrpc": "1.0", "result": true}))
        },
        Ok(false) => {
            Json(json!({"jsonrpc": "1.0", "result": false}))
        },
        Err(e) => {
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }

}
// Get balance
#[post("/get-wallet-balance",data="<post_data>")]
fn get_balance(post_data: Json<ReadAccountInfo>)-> Json<serde_json::Value> {
    let pub_add = &post_data.pub_address;
    match account::get_balance(pub_add) {
        Ok(acc_balance) => {
            let response_body = json!({
                "balance": acc_balance.to_string(), // Be cautious with private key handling
            });
            Json(json!({"jsonrpc": "1.0", "result": response_body}))
        },
        Err(e) => {
            error!("Error getting wallet: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }
}

#[post("/get-wallet-transactions",data="<post_data>")]
fn get_wallet_transactions(post_data: Json<ReadAccountInfo>)-> Json<serde_json::Value> {
    let pub_add = &post_data.pub_address;
    match public_txn::Txn::get_transactions_by_caller(pub_add) {
        Ok(txns) => {
            let response_body = json!({
                "transactions": txns, // Be cautious with private key handling
            });
            Json(json!({"jsonrpc": "1.0", "result": response_body}))
        },
        Err(e) => {
            error!("Error getting wallet: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }
}
#[post("/get-receiver-transactions",data="<post_data>")]
fn get_receiver_transactions(post_data: Json<ReadAccountInfo>)-> Json<serde_json::Value> {
    let pub_add = &post_data.pub_address;
    match public_txn::Txn::get_transactions_by_sender(pub_add) {
        Ok(txns) => {
            let response_body = json!({
                "transactions": txns, // Be cautious with private key handling
            });
            Json(json!({"jsonrpc": "1.0", "result": response_body}))
        },
        Err(e) => {
            error!("Error getting wallet: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }
}
#[post("/transfer-token",data="<post_data>")]
fn transfer_token(post_data: Json<TransferTokenInfo>)-> Json<serde_json::Value> {
    let from_address = &post_data.from_address;
    let from_priv_address = &post_data.from_private_key;
    let to_address = &post_data.to_address;
    let amount = &post_data.amount;
    match account::Account::transfer(from_address,to_address,*amount) {
        Ok(()) => {
            let response_body = json!({});
            Json(json!({"jsonrpc": "1.0", "result": response_body}))
        },
        Err(e) => {
            error!("Error: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }
}
#[post("/transfer-nft",data="<post_data>")]
fn transfer_nft(post_data: Json<TransferTokenInfo>)-> Json<serde_json::Value> {
    let from_address = &post_data.from_address;
    let from_priv_address = &post_data.from_private_key;
    let to_address = &post_data.to_address;
    let amount = &post_data.amount;
    match account::Account::transfer(from_address,to_address,*amount) {
        Ok(()) => {
            let response_body = json!({});
            Json(json!({"jsonrpc": "1.0", "result": response_body}))
        },
        Err(e) => {
            error!("Error: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }
}
#[post("/complete-transaction", data = "<transaction_info>")]
fn complete_transaction(transaction_info: Json<TransactionSignedInfo>) -> Json<serde_json::Value> {
    // Extract transaction information
    let txn_hash = &transaction_info.txn_hash;

    // Implement logic to complete the transaction
    // For example, updating its status in your database or any other required actions
    match public_txn::Txn::update_transaction_status(txn_hash,2) {
        Ok(_) => {
            Json(json!({
                "jsonrpc": "2.0",
                "result": "Transaction completed successfully"
            }))
        },
        Err(e) => {
            Json(json!({
                "jsonrpc": "2.0",
                "error": "Transaction completion failed"
            }))
        }
    }
}

#[get("/create-block")]
fn create_block() -> Json<serde_json::Value> {
    //Create blocks
    let response_body = json!({});
    public_block::pbft_pre_message_block_create_scheduler();
    Json(json!({"jsonrpc": "1.0", "result": response_body}))
}

#[get("/get-block")]
fn get_block() -> Json<serde_json::Value>{
    let local_blocks = APP_BLOCKS.lock().unwrap();
    let data = (*local_blocks).clone();
    let json_value = serde_json::to_value(data).unwrap(); 
    Json(json!({"jsonrpc": "1.0", "result": json_value}))
}
#[post("/read-transaction", data = "<txn_id_info>")]
fn read_transaction(txn_id_info: Json<TxnIdInfo>) -> Json<serde_json::Value> {
    let txn_id = &txn_id_info.txn_id;

    // Assuming a function `get_transaction_by_id` that fetches the transaction from storage
    match public_txn::Txn::get_transaction_by_id(txn_id) {
        Ok(transaction) => {
            // Assuming `transaction` is serializable with `serde`
            Json(json!({
                "jsonrpc": "2.0",
                "result": transaction
            }))
        },
        Err(e) => {
            println!("Error reading transaction: {:?}", e);
            Json(json!({
                "jsonrpc": "2.0", 
                "error": "Transaction read failed"
            }))
        }
    }
}


#[get("/healthcheck")]
fn healthcheck() -> Json<serde_json::Value> {
    // Perform any necessary health checks here. For simplicity, this example
    // will just return a success message.

    let response_body = json!({
        "status": "OK",
        "message": "Service is up and running"
    });
    Json(json!({"status":"Okay"}))
}


pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }
    
    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if request.method() == Method::Options {
            response.set_status(Status::NoContent);
            response.set_header(Header::new(
                "Access-Control-Allow-Methods",
                "POST, PATCH, GET, DELETE",
            ));
            response.set_header(Header::new(
                "Access-Control-Allow-Headers",
                "content-type, authorization",
            ));
        }
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
        response.set_header(Header::new("Vary", "Origin"));
    }
}

// Launch the Rocket HTTP server.
pub async fn start_rpc() {
    println!("Starting RPC server...");
    rocket::build()
        .attach(CORS)
        //.manage(swarm) // Add the swarm to the application state
        .configure(rocket::Config {
            address: std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
           //prod
            port:8000,
            //dev
            //port: 8545,
            ..rocket::Config::default()
        })
        .mount("/", routes![create_nft_contract,
                            create_transaction,
                            sign_transaction,
                            create_wallet,
                            mint_token_contract,
                            read_token_contract,
                            transfer_nft,
                            transfer_token,
                            get_balance,
                            get_wallet_transactions,
                            get_account,
                            complete_transaction,
                            read_transaction,
                            get_receiver_transactions,
                            create_block,
                            get_block,
                            healthcheck])
        .launch()
        .await
        .expect("Failed to start Rocket server");
}

