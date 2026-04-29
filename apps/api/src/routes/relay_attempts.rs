use application::{ BeginRelayAttemptCommand, FinishRelayAttemptCommand };
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
pub struct FinishRelayAttemptRequest {
    pub outcome: String,
    pub classification: Option<String>,
    pub reason: Option<String>,
    pub relay_reference: Option<String>,
    pub note: Option<String>,
}

pub async fn begin_relay_attempt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>
) -> Result<(StatusCode, Json<TransferIntentResponse>), ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;

    let transfer = state.relay_attempt_service.begin_attempt(BeginRelayAttemptCommand {
        transfer_id: id,
        started_at: Utc::now(),
    }).await?;

    Ok((StatusCode::OK, Json(to_response(transfer, "relay_attempt_started"))))
}

pub async fn finish_relay_attempt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<FinishRelayAttemptRequest>
) -> Result<(StatusCode, Json<TransferIntentResponse>), ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;

    let transfer = state.relay_attempt_service.finish_attempt(FinishRelayAttemptCommand {
        transfer_id: id,
        outcome: body.outcome,
        classification: body.classification,
        reason: body.reason,
        relay_reference: body.relay_reference,
        note: body.note,
        finished_at: Utc::now(),
    }).await?;

    Ok((StatusCode::OK, Json(to_response(transfer, "relay_attempt_finished"))))
}
