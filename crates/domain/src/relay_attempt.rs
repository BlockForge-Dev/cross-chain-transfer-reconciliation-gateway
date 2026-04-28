use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

use crate::{ AttemptNumber, RelayAttemptOutcome, RelayReference };

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayAttempt {
    pub attempt_no: AttemptNumber,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub outcome: Option<RelayAttemptOutcome>,
    pub relay_reference: Option<RelayReference>,
    pub note: Option<String>,
}

impl RelayAttempt {
    pub fn started(attempt_no: AttemptNumber, started_at: DateTime<Utc>) -> Self {
        Self {
            attempt_no,
            started_at,
            ended_at: None,
            outcome: None,
            relay_reference: None,
            note: None,
        }
    }

    pub fn finish(
        mut self,
        ended_at: DateTime<Utc>,
        outcome: RelayAttemptOutcome,
        relay_reference: Option<RelayReference>,
        note: Option<String>
    ) -> Self {
        self.ended_at = Some(ended_at);
        self.outcome = Some(outcome);
        self.relay_reference = relay_reference;
        self.note = note;
        self
    }
}
