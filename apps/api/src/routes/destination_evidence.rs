use application::RecordDestinationEvidenceCommand;
use axum::{ extract::{ Path, State }, http::{ HeaderMap, StatusCode }, Json };
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    error::ApiError,
    routes::transfer_intents::{ authenticate, to_response, TransferIntentResponse },
};

#[derive(Debug, Deserialize)]
pub struct RecordDestinationEvidenceRequest {
    pub destination_tx_hash: String,
    pub destination_chain: String,
    pub recipient: String,
    pub asset: String,
    pub quantity: String,
    pub status: String,
    pub note: Option<String>,
}

pub async fn record_destination_evidence(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<RecordDestinationEvidenceRequest>
) -> Result<(StatusCode, Json<TransferIntentResponse>), ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;

    let transfer = state.destination_evidence_service.record_evidence(
        RecordDestinationEvidenceCommand {
            transfer_id: id,
            destination_tx_hash: body.destination_tx_hash,
            destination_chain: body.destination_chain,
            recipient: body.recipient,
            asset: body.asset,
            quantity: body.quantity,
            status: body.status,
            note: body.note,
            recorded_at: Utc::now(),
        }
    ).await?;

    Ok((StatusCode::OK, Json(to_response(transfer, "updated_destination_evidence"))))
}
