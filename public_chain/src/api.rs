use actix_web::{web,App, HttpServer,Result, Error};
use serde::{Serialize, Deserialize};
use crate::mutex_public_swarm_net;
#[derive(Deserialize, Serialize)]
struct ResponseMessage {
    message: String,
}

async fn public_blocks() -> Result<web::Json<ResponseMessage>, Error> {
    // let mut swarm_lock = mutex_public_swarm_net.lock().unwrap();
    // let mut swarm_public_net = swarm_lock.as_mut().unwrap();
    // println!("Blocks: {:?}",swarm_public_net.behaviour_mut().app.get_blocks());
    Ok(web::Json(ResponseMessage {
        message: "Hello".to_string(),
    }))
}


#[actix_web::main]
pub async fn pub_api() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(public_blocks))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}