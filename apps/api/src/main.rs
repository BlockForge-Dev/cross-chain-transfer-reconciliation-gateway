mod app_state;
mod error;
mod routes;

use std::{ env, net::SocketAddr };

use application::TransferIntentService;
use axum::{ routing::{ get, post }, Router };
use persistence::{ connect, PostgresPersistence };
use tracing::info;

use crate::app_state::AppState;
use crate::routes::transfer_intents::{ create_transfer_intent, get_transfer_intent };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber
        ::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())
        )
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let api_bearer_token = env::var("API_BEARER_TOKEN").expect("API_BEARER_TOKEN must be set");
    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let pool = connect(&database_url, 10).await?;
    let repo = PostgresPersistence::new(pool);
    let service = TransferIntentService::new(repo).with_supported_chains(
        vec!["ethereum".into(), "solana".into(), "base".into(), "polygon".into(), "arbitrum".into()]
    );

    let state = AppState {
        service,
        api_bearer_token,
    };

    let app = Router::new()
        .route("/transfer-intents", post(create_transfer_intent))
        .route("/transfer-intents/{id}", get(get_transfer_intent))
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("cross-chain transfer api listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
