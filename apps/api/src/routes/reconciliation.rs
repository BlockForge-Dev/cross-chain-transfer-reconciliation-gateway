use application::ReconcileTransferCommand;
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
pub struct ReconcileTransferRequest {
    pub note: Option<String>,
}

pub async fn reconcile_transfer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    payload: Option<Json<ReconcileTransferRequest>>
) -> Result<(StatusCode, Json<TransferIntentResponse>), ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;

    let note = payload.map(|Json(body)| body.note).unwrap_or(None);

    let transfer = state.reconciliation_service.reconcile(ReconcileTransferCommand {
        transfer_id: id,
        note,
        reconciled_at: Utc::now(),
    }).await?;

    Ok((StatusCode::OK, Json(to_response(transfer, "reconciled"))))
}
