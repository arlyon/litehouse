use crate::identity::{EndpointIdentity, EventId, Generation, SourceEventId};

/// Identity of one endpoint-local state observation. Revisions are comparable
/// only for the same endpoint and provider generation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateIdentity {
    pub endpoint: EndpointIdentity,
    pub generation: Generation,
    pub revision: u64,
}

/// Stable identity for at-least-once event delivery. Retransmissions preserve
/// `event_id`, optional provider/source identity, endpoint, and generation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventIdentity {
    pub endpoint: EndpointIdentity,
    pub generation: Generation,
    pub event_id: EventId,
    pub source_event_id: Option<SourceEventId>,
}
