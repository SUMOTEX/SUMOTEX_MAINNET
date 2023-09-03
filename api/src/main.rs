use actix_web::{web,App, HttpServer,Result, Error};
use serde::{Serialize, Deserialize};
use public_chain::public_app::App as PubApp;

#[derive(Deserialize, Serialize)]
struct ResponseMessage {
    message: String,
}

async fn hello() -> Result<web::Json<ResponseMessage>, Error> {
    println!("{:?}",PubApp::get_blocks());
    Ok(web::Json(ResponseMessage {
        message: "Hello".to_string(),
    }))
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(hello))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}