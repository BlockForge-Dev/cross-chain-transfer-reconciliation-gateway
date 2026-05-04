use application::{ OpenExceptionCaseCommand, ResolveExceptionCaseCommand };
use axum::{ extract::{ Path, State }, http::{ HeaderMap, StatusCode }, Json };
use chrono::Utc;
use persistence::StoredExceptionCase;
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

use crate::{ app_state::AppState, error::ApiError, routes::transfer_intents::authenticate };

#[derive(Debug, Deserialize)]
pub struct OpenExceptionCaseRequest {
    pub classification: Option<String>,
    pub case_status: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveExceptionCaseRequest {
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExceptionCaseResponse {
    pub case_id: i64,
    pub transfer_id: Uuid,
    pub classification: String,
    pub case_status: String,
    pub note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn open_exception_case(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<OpenExceptionCaseRequest>
) -> Result<(StatusCode, Json<ExceptionCaseResponse>), ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;

    let case = state.exception_case_service.open_case(OpenExceptionCaseCommand {
        transfer_id: id,
        classification: body.classification,
        case_status: body.case_status,
        note: body.note,
        created_at: Utc::now(),
    }).await?;

    Ok((StatusCode::CREATED, Json(to_case_response(case))))
}

pub async fn list_exception_cases(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>
) -> Result<Json<Vec<ExceptionCaseResponse>>, ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;

    let cases = state.exception_case_service.list_cases(id).await?;
    Ok(Json(cases.into_iter().map(to_case_response).collect()))
}

pub async fn resolve_exception_case(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    payload: Option<Json<ResolveExceptionCaseRequest>>
) -> Result<(StatusCode, Json<ExceptionCaseResponse>), ApiError> {
    authenticate(&headers, &state.api_bearer_token)?;

    let note = payload.map(|Json(body)| body.note).unwrap_or(None);

    let case = state.exception_case_service.resolve_latest_case(ResolveExceptionCaseCommand {
        transfer_id: id,
        note,
        resolved_at: Utc::now(),
    }).await?;

    Ok((StatusCode::OK, Json(to_case_response(case))))
}

fn to_case_response(case: StoredExceptionCase) -> ExceptionCaseResponse {
    ExceptionCaseResponse {
        case_id: case.case_id,
        transfer_id: case.transfer_id,
        classification: format!("{:?}", case.classification),
        case_status: case.case_status,
        note: case.note,
        created_at: case.created_at,
        resolved_at: case.resolved_at,
    }
}
