use application::{ RelayAttemptService, SourceEvidenceService, TransferIntentService };
use persistence::PostgresPersistence;

#[derive(Clone)]
pub struct AppState {
    pub transfer_intent_service: TransferIntentService<PostgresPersistence>,
    pub source_evidence_service: SourceEvidenceService<PostgresPersistence>,
    pub relay_attempt_service: RelayAttemptService<PostgresPersistence>,
    pub api_bearer_token: String,
}
