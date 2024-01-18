// use actix_web::{get, web, HttpResponse,App, HttpServer, Responder};
// use std::process::Command;

// #[get("/build")]
// async fn build() -> impl Responder {
//     // Execute the build command
//     let output = Command::new("cargo")
//         .arg("build")
//         .arg("--target=wasm32-wasi")
//         .arg("--release")
//         .output();

//     // Check if the build was successful
//     if let Ok(output) = output {
//         if output.status.success() {
//             // The build was successful, return the wasm file
//             HttpResponse::Ok()
//                 .header("Content-Type", "application/wasm")
//                 .body(output.stdout)
//         } else {
//             // The build failed, return an error
//             HttpResponse::InternalServerError().body("Build failed")
//         }
//     } else {
//         // Error executing the build command
//         HttpResponse::InternalServerError().body("Error executing build command")
//     }
// }

// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     HttpServer::new(|| {
//         App::new().service(build)
//     })
//     .bind("127.0.0.1:8080")?
//     .run()
//     .await
// }
