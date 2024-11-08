use clap::Parser;
use methods::{GUEST_ELF, GUEST_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use std::fs::File;
use std::io::Read;
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    url: String,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    println!(
        "GUEST: 0x{}",
        hex::encode(GUEST_ID.map(u32::to_le_bytes).as_flattened())
    );

    let mut attestation_file = File::open("doc").unwrap();
    let mut attestation = Vec::new();
    attestation_file.read_to_end(&mut attestation).unwrap();

    println!("Attestation size: {}", attestation.len());

    let env = ExecutorEnv::builder()
        .write_slice(&attestation)
        .build()
        .unwrap();

    let prover = default_prover();
    // Enable groth16
    let prove_info = prover
        .prove_with_opts(env, GUEST_ELF, &ProverOpts::default())
        .unwrap();

    let receipt = prove_info.receipt;

    //println!("{:?}", receipt);
}
