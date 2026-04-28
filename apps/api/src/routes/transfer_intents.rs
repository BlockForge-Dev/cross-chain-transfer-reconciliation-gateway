use application::{ CreateTransferIntentCommand, CreateTransferIntentResult };
use axum::{
    extract::{ Path, State },
    http::{ header::AUTHORIZATION, HeaderMap, StatusCode },
    Json,
};
use chrono::Utc;
use domain::TransferIntent;
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

use crate::{ app_state::AppState, error::ApiError };

#[derive(Debug, Deserialize)]
pub struct CreateTransferIntentRequest {
    pub client_transfer_reference: String,
    pub source_chain: String,
    pub destination_chain: String,
    pub source_address: String,
    pub destination_recipient: String,
    pub asset: String,
    pub quantity: String,
}

#[derive(Debug, Serialize)]
pub struct TransferIntentResponse {
    pub transfer_id: Uuid,
    pub client_transfer_reference: String,
    pub source_chain: String,
    pub destination_chain: String,
    pub source_address: String,
    pub destination_recipient: String,
    pub asset: String,
    pub quantity: String,
    pub state: String,
    pub latest_failure_classification: Option<String>,
    pub latest_exception_classification: Option<String>,
    pub idempotency_status: String,
    pub receipt_url: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_transfer_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateTransferIntentRequest>
) -> Result<(StatusCode, Json<TransferIntentResponse>), ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;
    let idempotency_key = extract_idempotency_key(&headers)?;

    let command = CreateTransferIntentCommand {
        client_transfer_reference: body.client_transfer_reference,
        source_chain: body.source_chain,
        destination_chain: body.destination_chain,
        source_address: body.source_address,
        destination_recipient: body.destination_recipient,
        asset: body.asset,
        quantity: body.quantity,
        idempotency_key,
        received_at: Utc::now(),
    };

    let result = state.service.create_transfer(command).await?;

    match result {
        CreateTransferIntentResult::Created(transfer) =>
            Ok((StatusCode::CREATED, Json(to_response(transfer, "created")))),
        CreateTransferIntentResult::Existing(transfer) =>
            Ok((StatusCode::OK, Json(to_response(transfer, "existing")))),
    }
}

pub async fn get_transfer_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>
) -> Result<Json<TransferIntentResponse>, ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;
    let transfer = state.service.get_transfer(id).await?;
    Ok(Json(to_response(transfer, "queried")))
}

fn to_response(transfer: TransferIntent, idempotency_status: &str) -> TransferIntentResponse {
    TransferIntentResponse {
        transfer_id: transfer.id,
        client_transfer_reference: transfer.client_transfer_reference.0,
        source_chain: transfer.source_chain.0,
        destination_chain: transfer.destination_chain.0,
        source_address: transfer.source_address.0,
        destination_recipient: transfer.destination_recipient.0,
        asset: transfer.asset_amount.asset.0,
        quantity: transfer.asset_amount.quantity,
        state: format!("{:?}", transfer.state),
        latest_failure_classification: transfer.latest_failure.map(|f| format!("{:?}", f)),
        latest_exception_classification: transfer.latest_exception.map(|e| format!("{:?}", e)),
        idempotency_status: idempotency_status.to_string(),
        receipt_url: format!("/transfer-intents/{}/receipt", transfer.id),
        created_at: transfer.created_at,
        updated_at: transfer.updated_at,
    }
}

fn authenticate(headers: &HeaderMap, expected_token: &str) -> Result<(), ApiError> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let expected = format!("Bearer {}", expected_token);
    if auth != expected {
        return Err(ApiError::Unauthorized);
    }

    Ok(())
}

fn extract_idempotency_key(headers: &HeaderMap) -> Result<String, ApiError> {
    let key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::BadRequest("Idempotency-Key header is required".to_string()))?;

    Ok(key.to_string())
}
