use rocket::Request;
use rocket::post;
use rocket::routes;
use rocket::serde::json::Json;
use std::thread;
use serde_json::json;

// Define a structure for incoming RPC requests.
#[derive(serde::Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Option<serde_json::Value>,
}

// Route to handle RPC requests.
#[post("/", data = "<request>")]
fn handle_rpc(request: Json<RpcRequest>) -> Json<serde_json::Value> {
    println!("RPC called");
    match request.method.as_str() {
        "eth_chainId" => {
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": "0x1"}))  // Hardcoded to chain ID 1 for now
        },
        "eth_getBalance" => {
            // This is just a mock. In reality, you should extract the address from the params and look up the balance.
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": "0x8AC7230489E80000"})) // 10 Ether in Wei
        },
        "eth_accounts" => {
            // Mocking a single Ethereum address. You should query your actual accounts here.
            Json(json!({"jsonrpc": "2.0", "id": request.id, "result": ["0x742d35Cc6634C0532925a3b844Bc454e4438f44e"]}))
        },
        _ => {
            // Unsupported method
            Json(json!({"jsonrpc": "2.0", "id": request.id, "error": {"code": -32601, "message": "Method not found"}}))
        }
    }
}

// Launch the Rocket HTTP server.
pub async fn start_rpc() {
    println!("Starting RPC server...");
    rocket::build()
        .configure(rocket::Config {
            address: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            port: 8080,
            ..rocket::Config::default()
        })
        .mount("/", routes![handle_rpc])
        .launch()
        .await
        .expect("Failed to start Rocket server");
}

