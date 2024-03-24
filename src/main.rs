mod handler;

use std::fs;

use actix_web::{web, App, HttpServer};
use anyhow::{Context, Result};
use clap::Parser;

use handler::AppState;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// path to secp256k1 private key file
    #[arg(short, long)]
    secp256k1_secret: String,

    /// path to secp256k1 public key file
    #[arg(short, long)]
    secp256k1_public: String,

    /// server ip
    #[arg(short, long)]
    ip: String,

    /// server port
    #[arg(short, long)]
    port: u16,
}

#[actix_web::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let secp256k1_secret = fs::read(cli.secp256k1_secret.clone()).with_context(|| {
        format!(
            "Failed to read secp256k1_secret from {}",
            cli.secp256k1_secret
        )
    })?;
    let secp256k1_secret = secp256k1::SecretKey::from_slice(&secp256k1_secret)
        .context("unable to decode secp256k1_secret key from slice")?;

    let secp256k1_public = fs::read(cli.secp256k1_public.clone()).with_context(|| {
        format!(
            "Failed to read secp256k1_public from {}",
            cli.secp256k1_public
        )
    })?;
    let secp256k1_public: [u8; 64] = secp256k1_public
        .as_slice()
        .try_into()
        .context("invalid public key length")?;

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                secp256k1_secret,
                secp256k1_public,
            }))
            .service(handler::verify_raw)
    })
    .bind((cli.ip.clone(), cli.port))
    .context("unable to start the server")?
    .run();

    println!("api server running at {}:{}", cli.ip, cli.port);
    server.await.context("error while running server")?;

    Ok(())
}
