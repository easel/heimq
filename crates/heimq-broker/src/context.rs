//! Per-request context threaded through backend trait calls.

/// Caller identity supplied by an embedding broker.
///
/// heimq-broker carries this value to storage, offset, and group-coordinator
/// backends but does not interpret it. Authentication, tenant selection, RBAC,
/// and quota decisions remain the embedder/backend's responsibility.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestContext {
    pub principal: Option<String>,
    pub tenant: Option<String>,
    pub client_id: Option<String>,
}

impl RequestContext {
    pub const ANONYMOUS: Self = Self {
        principal: None,
        tenant: None,
        client_id: None,
    };

    pub fn new(
        principal: impl Into<Option<String>>,
        tenant: impl Into<Option<String>>,
        client_id: impl Into<Option<String>>,
    ) -> Self {
        Self {
            principal: principal.into(),
            tenant: tenant.into(),
            client_id: client_id.into(),
        }
    }

    pub fn anonymous() -> Self {
        Self::default()
    }

    pub fn with_client_id(client_id: impl Into<Option<String>>) -> Self {
        Self {
            client_id: client_id.into(),
            ..Self::default()
        }
    }
}
