use application::{
    DestinationEvidenceService,
    ExceptionCaseService,
    ReconciliationService,
    RelayAttemptService,
    SourceEvidenceService,
    TransferIntentService,
};
use persistence::PostgresPersistence;

#[derive(Clone)]
pub struct AppState {
    pub transfer_intent_service: TransferIntentService<PostgresPersistence>,
    pub source_evidence_service: SourceEvidenceService<PostgresPersistence>,
    pub relay_attempt_service: RelayAttemptService<PostgresPersistence>,
    pub destination_evidence_service: DestinationEvidenceService<PostgresPersistence>,
    pub reconciliation_service: ReconciliationService<PostgresPersistence>,
    pub exception_case_service: ExceptionCaseService<PostgresPersistence>,
    pub api_bearer_token: String,
}
