use actix_web::{App, HttpServer, get};
use serde::{Serialize, Deserialize};
use actix_web::{Responder, HttpResponse};

/// A valid response from the validator server
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub status: bool,
    pub message: String,
    pub ip_addr: String,
    pub port: u16,
}

#[get("/validate")]
async fn root() -> impl Responder {
   println!("Validating server information");
   HttpResponse::Ok().json(
    Response {
        status: true,
        message: "User fetched successfully".to_string(),
        ip_addr: "https://admin-examalpha.netlify.app".to_string(),
        port: 443,
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting HTTP server at port 8080");
    HttpServer::new(|| {
        App::new()
            .service(root)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}