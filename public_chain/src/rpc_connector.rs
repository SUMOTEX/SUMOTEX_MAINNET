use rocket::Request;
use rocket::post;
use rocket::routes;
use rocket::serde::json::Json;
use serde_json::json;
use rocket::http::Header;
use rocket::{ Response};
use rocket::fairing::{Fairing, Info, Kind};
use log::error;
use crate::smart_contract;
use crate::public_swarm;
use crate::account;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

// Define a structure for incoming RPC requests.
#[derive(serde::Deserialize, Debug)]
struct RpcRequest {
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, Debug)]
pub struct ContractInfo {
    call_address: String,
    private_key: String,
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

// Route to handle RPC requests.
#[post("/", data = "<request>")]
fn handle_rpc(request: Json<RpcRequest>) -> Json<serde_json::Value> {
   // println!("RPC called");
    match request.method.as_str() {
        "eth_chainId" => {
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": "0x184"}))  // 0x58 is 88 in hexadecimal
        },
        "eth_getBalance" => {
            // This is just a mock. In reality, you should extract the address from the params and look up the balance.
            if let Some(params) = &request.params {
                Json(json!({"jsonrpc": "2.0", "id": request.id, "result": "1000"}))
                //let address = params[0].as_str().unwrap_or_default();
                // match account::get_balance(address).await {
                //     Ok(balance) => Json(json!({"jsonrpc": "2.0", "id": request.id, "result": balance})),
                //     Err(_) => Json(json!({"jsonrpc": "2.0", "id": request.id, "error": {"code": -32603, "message": "Internal error"}}))
                // }
            } else {
                Json(json!({"jsonrpc": "2.0", "id": request.id, "error": {"code": -32602, "message": "Invalid params"}}))
            }
        },
        "eth_accounts" => {
            // Mocking a single Ethereum address. You should query your actual accounts here.
            println!("{:?}",request);
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": ["0x742d35Cc6634C0532925a3b844Bc454e4438f44e"]}))
        },
        "eth_sendTransaction" => {
            // Mock a transaction hash for now
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"}))
        },
        "eth_call" => {
            // Mock a return value
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": "0x"}))
        },
        "eth_getTransactionReceipt" => {
            // Mock a transaction receipt
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": {
                "transactionHash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                "status": "0x1",
                // ... other receipt fields
            }}))
        },
        "eth_estimateGas" => {
            // Mock a gas estimation
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": "0x5208"}))
        },
        "net_version" => {
            // Mock a network version
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": "1"}))
        },
        _ => {
            // Unsupported method
            Json(json!({"jsonrpc": "2.0", "id": request.id, "error": {"code": -32601, "message": "Method not found"}}))
        }
    }
}


// Route to handle RPC requests.
#[post("/create-nft-contract", data = "<post_data>")]
fn create_nft_contract(post_data: Json<ContractInfo>)-> Json<serde_json::Value> {
    println!("create_nft_contract");
    let call_address = &post_data.call_address;
    let private_key = &post_data.private_key;
    match smart_contract::create_erc721_contract_official(&call_address, &private_key) {
        Ok(contract_address) => {
            println!("Contract successfully created: {:?}", contract_address);
            let response_body = json!({"contract_address": contract_address});
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
        Ok(token_id) => {
            let response_body = json!({"token_id": token_id.to_string()});
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
            error!("Error creating wallet: {:?}", e);
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

pub struct CORS;
#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }
    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
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
            port: 8545,
            ..rocket::Config::default()
        })
        .mount("/", routes![handle_rpc,create_nft_contract,create_wallet,mint_token_contract])
        .launch()
        .await
        .expect("Failed to start Rocket server");
}

