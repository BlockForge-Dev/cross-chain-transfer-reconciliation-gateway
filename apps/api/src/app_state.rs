use application::TransferIntentService;
use persistence::PostgresPersistence;

#[derive(Clone)]
pub struct AppState {
    pub service: TransferIntentService<PostgresPersistence>,
    pub api_bearer_token: String,
}
