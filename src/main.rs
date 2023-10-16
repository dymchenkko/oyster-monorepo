#[macro_use]
extern crate lazy_static;
use actix_web::{rt::spawn, web, App, HttpServer};
use std::error::Error;
use std::fs;

mod config;
mod handlers;
mod types;

use types::handlers::AppState;

// global config
lazy_static! {
    static ref CONFIG: config::Configuration =
        config::Configuration::new().expect("config can be loaded");
}
#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let private_key = fs::read(CONFIG.enclave.privatekeypath.clone())?;
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                private_key: private_key.clone(),
            }))
            .service(handlers::attestationdoc::verify)
    })
    .bind((CONFIG.server.ip.clone(), CONFIG.server.port))?
    .run();
    println!(
        "api server running at {}:{}",
        CONFIG.server.ip, CONFIG.server.port
    );
    server.await?;
    Ok(())
}
