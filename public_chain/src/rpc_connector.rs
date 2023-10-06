use rocket::Request;
use rocket::post;
use rocket::routes;
use rocket::serde::json::Json;
use std::thread;
use serde_json::json;
use rocket_cors::Cors;
use rocket::http::Method;
use rocket::http::Header;
use rocket::{ Response};
use rocket::fairing::{Fairing, Info, Kind};
// Define a structure for incoming RPC requests.
#[derive(serde::Deserialize, Debug)]
struct RpcRequest {
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    params: Option<serde_json::Value>,
}

// Route to handle RPC requests.
#[post("/", data = "<request>")]
fn handle_rpc(request: Json<RpcRequest>) -> Json<serde_json::Value> {
    println!("RPC called");
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
        .configure(rocket::Config {
            address: std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            port: 8545,
            ..rocket::Config::default()
        })
        .mount("/", routes![handle_rpc])
        .launch()
        .await
        .expect("Failed to start Rocket server");
}

