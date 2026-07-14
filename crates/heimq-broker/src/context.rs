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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // @covers US-016-AC4
    // @covers US-017-AC1
    fn request_context_carries_identity_without_policy_decisions() {
        let ctx = RequestContext::new(
            Some("principal-a".to_string()),
            Some("tenant-a".to_string()),
            Some("client-a".to_string()),
        );

        assert_eq!(ctx.principal.as_deref(), Some("principal-a"));
        assert_eq!(ctx.tenant.as_deref(), Some("tenant-a"));
        assert_eq!(ctx.client_id.as_deref(), Some("client-a"));
    }

    #[test]
    // @covers US-016-AC4
    fn anonymous_context_has_no_embedder_policy_identity() {
        assert_eq!(RequestContext::anonymous(), RequestContext::ANONYMOUS);
        assert!(RequestContext::ANONYMOUS.principal.is_none());
        assert!(RequestContext::ANONYMOUS.tenant.is_none());
        assert!(RequestContext::ANONYMOUS.client_id.is_none());
    }
}
