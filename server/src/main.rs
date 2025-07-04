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

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordResponse {
    pub message: String,
    pub ip_addr: String,
}

#[get("/validate")]
async fn root() -> impl Responder {
   log::info!("Validating server information");
   HttpResponse::Ok().json(
    Response {
        status: true,
        message: "User fetched successfully".to_string(),
        ip_addr: "https://admin-examalpha.netlify.app".to_string(),
        port: 443,
    })
}

#[get("/password")]
async fn get_password() -> impl Responder {
   log::info!("Received a request for password !!");
   HttpResponse::Ok().json(
   PasswordResponse{
    message: "password".into(),
    ip_addr: "192.67.4.1".into()
   })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    log::info!("Starting HTTP server at port 8080");
    HttpServer::new(|| {
        App::new()
            .service(root)
            .service(get_password)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}