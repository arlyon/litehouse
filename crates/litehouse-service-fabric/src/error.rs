use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingError {
    NotFound,
    Ambiguous,
    StaleHandle,
    EndpointRemoved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationError {
    Denied,
    ExpiredGrant,
    InsufficientScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityError {
    Unavailable,
    Reconnecting,
    ShuttingDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationOrigin {
    Caller,
    Provider,
    Host,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingError {
    DeadlineExceeded { deadline_monotonic_nanoseconds: u64 },
    Cancelled(CancellationOrigin),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityError {
    Overloaded {
        retry_after_milliseconds: Option<u64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionError {
    ProviderTrapped,
    ProtocolFailed,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownOutcomeCause {
    DeadlineExceeded,
    Cancelled(CancellationOrigin),
    Disconnected,
    ProviderFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeUnknownError {
    pub cause: UnknownOutcomeCause,
    pub request_id: String,
    pub attempt: u32,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    Binding(BindingError),
    Authorization(AuthorizationError),
    Availability(AvailabilityError),
    Timing(TimingError),
    Capacity(CapacityError),
    Execution(ExecutionError),
    OutcomeUnknown(OutcomeUnknownError),
    ContractError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAdvice {
    Never,
    Reconnect,
    Rebind,
    RetryAfter(Option<u64>),
}

/// A host-synthesized error using the same typed core and retry model as
/// `litehouse:errors@1`. Contract adapters add their own typed detail record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceError {
    pub inner: CoreError,
    pub retry: RetryAdvice,
    pub message: Option<String>,
    pub error_id: String,
}

impl ServiceError {
    pub(crate) fn new(inner: CoreError, retry: RetryAdvice, sequence: u64) -> Self {
        Self {
            inner,
            retry,
            message: None,
            error_id: format!("fabric-{sequence:016x}"),
        }
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "service fabric error {}: {:?}",
            self.error_id, self.inner
        )
    }
}

impl Error for ServiceError {}
