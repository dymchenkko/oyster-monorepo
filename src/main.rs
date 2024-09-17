use anyhow::Result;
use clap::command;
use clap::Parser;
use diesel::Connection;
use diesel::PgConnection;
use dotenvy::dotenv;

use oyster_indexer::event_loop;
use oyster_indexer::start_from;
use oyster_indexer::AlloyProvider;
use tracing::debug;
use tracing::error;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// RPC URL
    #[arg(short, long)]
    rpc: String,

    /// Market contract
    #[arg(short, long)]
    contract: String,
}

fn run() -> Result<()> {
    let args = Args::parse();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
    let provider = AlloyProvider {
        url: args.rpc.parse()?,
        contract: args.contract.parse()?,
    };
    let is_start_set = start_from(&mut conn, 87252070)?;
    debug!("is_start_set: {}", is_start_set);
    event_loop(&mut conn, provider)
}

fn main() -> Result<()> {
    dotenv().ok();

    // seems messy, see if there is a better way
    let mut filter = EnvFilter::new("info");
    if let Ok(var) = std::env::var("RUST_LOG") {
        filter = filter.add_directive(var.parse()?);
    }
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_env_filter(filter)
        .init();

    run().inspect_err(|e| error!(?e, "run error"))
}
