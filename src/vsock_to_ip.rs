// Based on https://github.com/tokio-rs/tokio/blob/master/examples/proxy.rs
//
// Copyright (c) 2022 Tokio Contributors and Marlin Contributors
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_vsock::{VsockAddr, VsockListener, VsockStream};

mod utils;

/// Creates a vsock proxy for ip server.
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// vsock address of the proxy to be set up <cid:port>
    #[clap(short, long, value_parser)]
    vsock_addr: String,
    /// ip address of the listener <ip:port>
    #[clap(short, long, value_parser)]
    ip_addr: String,
}

#[tokio::main]
pub async fn vsock_to_ip(cid: u32, port: u32, ip_addr: &String) -> Result<()> {
    let listen_addr = VsockAddr::new(cid, port);
    let server_addr = ip_addr;

    println!("Listening on: {:?}", listen_addr);
    println!("Proxying to: {:?}", server_addr);

    let mut listener = VsockListener::bind(listen_addr).expect("listener failed");

    while let Ok((inbound, _)) = listener.accept().await {
        let transfer = transfer(inbound, server_addr.clone()).map(|r| {
            if let Err(e) = r {
                println!("Failed to transfer; error={}", e);
            }
        });

        tokio::spawn(transfer);
    }

    Ok(())
}

async fn transfer(inbound: VsockStream, proxy_addr: String) -> Result<()> {
    let mut outbound = TcpStream::connect(proxy_addr.clone())
        .await
        .context("failed to connect to endpoint")?;

    let inbound_addr = inbound
        .local_addr()
        .context("could not fetch inbound addr")?
        .to_string();

    let (mut ri, mut wi) = io::split(inbound);
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = async {
        io::copy(&mut ri, &mut wo)
            .await
            .context("error in vsock to ip copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        println!("vsock to ip copy exited");
        wo.shutdown().await
    };

    let server_to_client = async {
        io::copy(&mut ro, &mut wi)
            .await
            .context("error in ip to vsock copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        println!("ip to vsock copy exited");
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client).with_context(|| {
        format!(
            "error in connection between {} and {}",
            inbound_addr, proxy_addr
        )
    })?;

    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let x = utils::split_vsock(&cli.vsock_addr).expect("vsock address not valid");
    if let Some((cid, port)) = x {
        let x = vsock_to_ip(cid, port, &cli.ip_addr);
        println!("{:?}", x);
    }
}
