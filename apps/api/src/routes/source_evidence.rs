use application::RecordSourceEvidenceCommand;
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
pub struct RecordSourceEvidenceRequest {
    pub source_tx_hash: String,
    pub status: String,
    pub note: Option<String>,
}

pub async fn record_source_evidence(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<RecordSourceEvidenceRequest>
) -> Result<(StatusCode, Json<TransferIntentResponse>), ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;

    let command = RecordSourceEvidenceCommand {
        transfer_id: id,
        source_tx_hash: body.source_tx_hash,
        note: body.note,
        recorded_at: Utc::now(),
    };

    let status = body.status.trim().to_lowercase();

    let transfer = match status.as_str() {
        "observed" => state.source_evidence_service.record_observed(command).await?,
        "confirmed" => state.source_evidence_service.record_confirmed(command).await?,
        _ => {
            return Err(
                ApiError::BadRequest("status must be either 'observed' or 'confirmed'".to_string())
            );
        }
    };

    Ok((StatusCode::OK, Json(to_response(transfer, "updated_source_evidence"))))
}
