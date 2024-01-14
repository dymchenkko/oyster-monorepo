use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use http::{Request, Response, StatusCode};
use hyper::{server::conn::Http, service::service_fn, Body};
use libsodium_sys::{
    crypto_sign_ed25519_pk_to_curve25519, crypto_sign_ed25519_sk_to_curve25519, crypto_sign_keypair,
};
use std::convert::Infallible;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tower::ServiceExt;

pub use oyster::scallop::*;

#[derive(Default)]
struct AuthStore {
    store: HashMap<[u8; 32], ([u8; 48], [u8; 48], [u8; 48])>,
}

impl ScallopAuthStore for AuthStore {
    fn contains(&self, key: &[u8; 32]) -> bool {
        self.store.contains_key(key)
    }

    fn get(&self, key: &[u8; 32]) -> Option<&([u8; 48], [u8; 48], [u8; 48])> {
        self.store.get(key)
    }

    fn set(&mut self, key: [u8; 32], pcrs: ([u8; 48], [u8; 48], [u8; 48])) {
        self.store.insert(key, pcrs);
    }

    fn verify(
        &mut self,
        attestation: &[u8],
        _key: &[u8; 32],
    ) -> Option<([u8; 48], [u8; 48], [u8; 48])> {
        if attestation == b"good auth" {
            Some(([1u8; 48], [2u8; 48], [3u8; 48]))
        } else {
            None
        }
    }
}

struct Auther {}

impl ScallopAuther for Auther {
    async fn new_auth(&mut self) -> Box<[u8]> {
        b"good auth".to_owned().into()
    }
}

async fn hello(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    println!("server: hello");
    Ok(Response::new(Body::from("Hello World!")))
}

async fn server_task(key: [u8; 32]) -> Result<(), Box<dyn Error + Send + Sync>> {
    let server = TcpListener::bind("127.0.0.1:21000").await?;
    let mut auth_store = AuthStore::default();
    let mut auther = Auther {};

    loop {
        let (stream, _) = server.accept().await?;

        let stream = new_server_async_Noise_IX_25519_ChaChaPoly_BLAKE2b(
            stream,
            &key,
            Some(&mut auth_store),
            Some(&mut auther),
        )
        .await?;

        println!("Client key: {:?}", stream.get_remote_static());

        tokio::task::spawn(async move {
            if let Err(http_err) = Http::new()
                .http1_only(true)
                .http1_keep_alive(true)
                .serve_connection(stream, service_fn(hello))
                .await
            {
                eprintln!("Error while serving HTTP connection: {}", http_err);
            }
        });
    }
}

async fn client_task(key: [u8; 32]) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut auth_store = AuthStore::default();
    let mut auther = Auther {};

    loop {
        let stream = TcpStream::connect("127.0.0.1:21000").await?;

        let stream = new_client_async_Noise_IX_25519_ChaChaPoly_BLAKE2b(
            stream,
            &key,
            Some(&mut auth_store),
            Some(&mut auther),
        )
        .await?;

        println!("Server key: {:?}", stream.get_remote_static());

        let (mut request_sender, connection) = hyper::client::conn::handshake(stream).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Error in connection: {}", e);
            }
        });

        let request = Request::builder().method("GET").body(Body::from(""))?;
        let response = request_sender.send_request(request).await?;
        assert!(response.status() == StatusCode::OK);
        println!("{:?}", hyper::body::to_bytes(response.into_body()).await?);

        request_sender.ready().await?;
        let request = Request::builder().method("GET").body(Body::from(""))?;
        let response = request_sender.send_request(request).await?;
        assert!(response.status() == StatusCode::OK);
        println!("{:?}", hyper::body::to_bytes(response.into_body()).await?);

        sleep(Duration::from_secs(5)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut sign_pk = [0u8; 32];
    let mut sign_sk = [0u8; 64];
    let mut pk = [0u8; 32];
    let mut sk = [0u8; 32];
    unsafe { crypto_sign_keypair(sign_pk.as_mut_ptr(), sign_sk.as_mut_ptr()) };
    unsafe { crypto_sign_ed25519_pk_to_curve25519(pk.as_mut_ptr(), sign_pk.as_ptr()) };
    unsafe { crypto_sign_ed25519_sk_to_curve25519(sk.as_mut_ptr(), sign_sk.as_ptr()) };

    tokio::spawn(server_task(sk));

    sleep(Duration::from_secs(5)).await;
    client_task(sk).await?;

    Ok(())
}
