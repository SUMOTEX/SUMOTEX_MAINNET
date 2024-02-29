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
use crate::swarm;
use crate::account;
use crate::txn;
use crate::block;
use crate::block::Block;
use crate::txn::TransactionType;
use crate::app::App as PubApp;
use lazy_static::lazy_static;
use rocket::Route;
use std::collections::HashMap;
use secp256k1::SecretKey;
use rocket::data::Limits;
use rocket::data::ByteUnit; 
use rocket::config::{Config};
use rocket_okapi::{openapi, routes_with_openapi, JsonSchema};
use rocket_okapi::swagger_ui::{make_swagger_ui, SwaggerUIConfig};
use rocket_okapi::swagger_ui::DocExpansion;
use rocket_okapi::swagger_ui::DefaultModelRendering;


#[derive(Debug,serde::Serialize, serde::Deserialize)]
struct TransactionInfo {
    caller_address: String,
    to_address: String,
    computed_value: u128,
    transaction_type: String
}
#[derive(Debug,serde::Serialize, serde::Deserialize)]
struct TransactionSignedInfo {
    caller_address: String,
    txn_hash:String,
    computed_value: u128,
    transaction_type: String,
    private_key:String
}
#[derive(serde::Deserialize, Debug)]
pub struct Parameter {
    name: String,
    #[serde(rename = "type")]
    p_type: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct GenericContractInfo {
    contract_address: String,
    caller_address:String,
    private_key: String,
    function_name:String,
    args_input_values:Vec<serde_json::Value>,
}
#[derive(serde::Deserialize, Debug)]
pub struct ReadContractInfo {
    contract_address: String
}
#[derive(serde::Deserialize, Debug)]
pub struct ContractInfo {
    call_address: String,
    private_key: String,
    contract_name:String,
    contract_symbol:String
}
#[derive(serde::Deserialize, Debug)]
pub struct GenericContractCreationInfo {
    call_address: String,
    private_key: String,
    contract_name:String,
    contract_symbol:String,
    wasm_file:String
}
#[derive(serde::Deserialize, Debug)]
pub struct MintTokenInfo {
    contract_address: String,
    caller_address: String,
    caller_private_key:String,
    ipfs_detail:String,
    owner_email:String,
    owner_name:String,
    owner_creds:String
}
#[derive(serde::Deserialize, Debug)]
pub struct ReadTokenInfo {  
    contract_address: String,
    token_id: i32,
}

#[derive(serde::Deserialize, Debug)]
pub struct ReadTotalTokenInfo {  
    contract_address: String
}

#[derive(serde::Deserialize, Debug)]
pub struct ReadAccountInfo {
    pub_address: String
}
#[derive(serde::Deserialize, Debug)]
pub struct BlockInfo {
    pub_address: String
}

#[derive(serde::Deserialize, Debug)]
pub struct TransferTokenInfo {
    from_address: String,
    from_private_key:String,
    to_address:String,
    amount:u128
}
#[derive(serde::Deserialize, Debug)]
pub struct TxnIdInfo {
    txn_hash: String
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

pub fn add_api_blocks(app: PubApp) {
    let new_blocks = app.get_blocks();
    let mut app_blocks = APP_BLOCKS.lock().unwrap();
    app_blocks.blocks = new_blocks.clone();
    let _json_response = json!(new_blocks);
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
        _ => return Json(json!({"jsonrpc": "1.0", "error": "Invalid transaction type"}))
    };
    match txn::Txn::create_and_prepare_transaction(
        transaction_type,
        caller_address.to_string(),
        to_address.to_string(),
        computed_value
    ) {
        Ok((txn_hash_hex,gas_cost, _)) => {
            println!("Transaction successfully prepared: {:?}", txn_hash_hex);
            Json(json!({
                "jsonrpc": "1.0", 
                "result": {
                    "transaction_hash": txn_hash_hex,
                    "gas_cost": gas_cost
                }
            }))
        },
        Err(e) => {
            println!("Error creating transaction: {:?}", e);
            Json(json!({
                "jsonrpc": "1.0", 
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
    match txn::Txn::sign_and_submit_transaction(caller_address,transaction_signed_data.txn_hash.clone(),&private_key){
        Ok(()) => {
            Json(json!({
                "jsonrpc": "1.0", 
                "result": {
                }
            }))
        },
        Err(e) => {
            println!("Error signing transaction: {:?}", e);
            Json(json!({
                "jsonrpc": "1.0", 
                "error": "Transaction signed failed"
            }))
        }
    }
}

#[post("/create-contract", data = "<post_data>")]
fn create_contract(post_data: Json<GenericContractCreationInfo>)-> Json<serde_json::Value> {
    println!("Create Contract");
    let call_address = &post_data.call_address;
    let private_key = &post_data.private_key;
    let contract_name = &post_data.contract_name;
    let contract_symbol = &post_data.contract_symbol;
    let wasm_file = &post_data.wasm_file;
    match smart_contract::create_contract_official(&call_address, &private_key,contract_name,contract_symbol,wasm_file) {
        Ok((contract_address,txn_hash,gas_cost)) => {
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
    let owner_email = &post_data.owner_email;
    let owner_name = &post_data.owner_name;
    let owner_creds = &post_data.owner_creds;
    match smart_contract::mint_token_official(&contract_address, &owner_address,&owner_private_key,&owner_creds,&owner_name,&owner_email,&ipfs) {
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
    match smart_contract::read_id(&contract_address, token_id) {
        Ok((token_owner,ipfs_data,owner_name,owner_creds,owned_email)) => {
            let response_body = json!({"owner_address": token_owner,"ipfs":ipfs_data,"name":owner_name,"credential":owner_creds,"owner_email":owned_email});
            Json(json!({"jsonrpc": "1.0",  "result": response_body}))
        },
        Err(e) => {
            error!("Error creating contract: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }   
}
#[post("/read-contract", data = "<post_data>")]
fn read_contract(post_data: Json<ReadContractInfo>)-> Json<serde_json::Value> {
    let contract_address = &post_data.contract_address;
    match smart_contract::read_contract(&contract_address){
        Ok(contract_detail) => {
            let response_body = json!({"contract_detail": contract_detail});
            Json(json!({"jsonrpc": "1.0",  "result": response_body}))
        },
        Err(e) => {
            error!("Error creating contract: {:?}", e);
            Json(json!({"jsonrpc": "1.0", "result": "error"}))
        }
    }
}
// // Route to handle RPC requests.
#[post("/read-total-minted", data = "<post_data>")]
fn read_total_minted_token(post_data: Json<ReadTotalTokenInfo>)-> Json<serde_json::Value> {
    println!("Read Minted Token");
    let contract_address = &post_data.contract_address;
    match smart_contract::read_total_token_erc721(&contract_address) {
        Ok(minted_token) => {
            let response_body = json!({"total_supply": minted_token});
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
        Err(_) => {
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

#[post("/get-caller-transactions",data="<post_data>")]
fn get_wallet_transactions(post_data: Json<ReadAccountInfo>)-> Json<serde_json::Value> {
    let pub_add = &post_data.pub_address;
    match txn::Txn::get_transactions_by_caller(pub_add) {
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
    match txn::Txn::get_transactions_by_sender(pub_add) {
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
    match txn::Txn::update_transaction_status(txn_hash,2) {
        Ok(_) => {
            Json(json!({
                "jsonrpc": "1.0",
                "result": "Transaction completed successfully"
            }))
        },
        Err(e) => {
            Json(json!({
                "jsonrpc": "1.0",
                "error": "Transaction completion failed"
            }))
        }
    }
}

#[post("/call-contract", data = "<post_data>")]
fn generic_smart_contract_function_call(post_data: Json<GenericContractInfo>)->  Json<serde_json::Value>{
        // Extract transaction information
        let contract_address = &post_data.contract_address;
        let call_address = &post_data.caller_address;
        let private_key = &post_data.private_key;
        let function_name = &post_data.function_name;
        let args_input_values = &post_data.args_input_values;
        match smart_contract::call_contract_function(&contract_address,&call_address, &private_key,&function_name,&args_input_values) {
            Ok(result_map) => {
                let response_body = json!({
                    "contract_address": contract_address,
                    "result": result_map,
                });
                Json(json!({"jsonrpc": "1.0", "result": response_body}))
            },
            Err(e) => {
                error!("Error calling function: {:?}", e);
                let error_details = format!("{:?}", e);
                Json(json!({"jsonrpc": "2.0", "error": {"code": 32000, "message": error_details}}))
            }
        }
}

#[get("/create-block")]
fn create_block() -> Json<serde_json::Value> {
    //Create blocks
    let response_body = json!({});
    let _ = block::pbft_pre_message_block_create_scheduler();
    Json(json!({"jsonrpc": "1.0", "result": response_body}))
}

#[post("/get-blocks", data = "<block_info>")]
fn get_block(block_info:Json<BlockInfo>) -> Json<serde_json::Value>{
    let local_blocks = APP_BLOCKS.lock().unwrap();
    //let data = (*local_blocks.blocks);
    let json_value = serde_json::to_value(&local_blocks.blocks).unwrap(); 
    Json(json!({"jsonrpc": "1.0", "result": json_value}))
}

#[get("/latest-block")]
fn get_latest_block() -> Json<serde_json::Value>{
    let data = block::get_latest_block_hash();
    match data {
        Ok(result) => Json(json!({"jsonrpc": "1.0", "result": result})),
        Err(err) => {
            let serialized_error = serde_json::to_string(&(err.to_string()))
                .expect("Serialization failed");
            Json(json!({
                "jsonrpc": "1.0",
                "error": {
                    "code": 500,
                    "message": "Internal Server Error",
                    "data": serialized_error
                }
            }))
        }
    }
}

#[post("/read-transaction", data = "<txn_id_info>")]
fn read_transaction(txn_id_info: Json<TxnIdInfo>) -> Json<serde_json::Value> {
    let txn_id = &txn_id_info.txn_hash;
    // Assuming a function `get_transaction_by_id` that fetches the transaction from storage
    match txn::Txn::get_transaction_by_id(txn_id) {
        Ok(transaction) => {
            // Assuming `transaction` is serializable with `serde`
            Json(json!({
                "jsonrpc": "1.0",
                "result": transaction
            }))
        },
        Err(e) => {
            println!("Error reading transaction: {:?}", e);
            Json(json!({
                "jsonrpc": "1.0", 
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
    let rocket_config = Config {
        address: std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
        port: 8000, // or 8545 for dev
        limits: Limits::new()
            .limit("json",ByteUnit::Megabyte(32)), // 32 MiB in bytes
        ..Config::default()
    };
    rocket::build()
    .attach(CORS)
    //.manage(swarm) // Add the swarm to the application state
    .configure(rocket_config)
    .mount("/", routes![
                        create_transaction,
                        sign_transaction,
                        create_wallet,
                        mint_token_contract,
                        read_token_contract,
                        read_contract,
                        transfer_nft,
                        transfer_token,
                        get_balance,
                        get_wallet_transactions,
                        get_account,
                        complete_transaction,
                        read_transaction,
                        read_total_minted_token,
                        get_receiver_transactions,
                        generic_smart_contract_function_call,
                        create_block,
                        create_contract,
                        get_block,
                        get_latest_block,
                        healthcheck])
    .launch()
    .await
    .expect("Failed to start Rocket server");
}

