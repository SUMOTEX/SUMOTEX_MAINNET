use actix_web::{web, App, HttpResponse, HttpServer, Responder};

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

async fn greet(name: web::Path<String>) -> impl Responder {
    HttpResponse::Ok().body(format!("Hello, {}!", name))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(hello))
            .route("/{name}", web::get().to(greet))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
