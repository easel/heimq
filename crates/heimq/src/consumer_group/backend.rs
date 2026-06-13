//! Re-export of the group-coordinator backend trait from heimq-broker.

pub use heimq_broker::consumer_group::backend::{
    GroupCoordinatorBackend, GroupCoordinatorCapabilities, GroupDescription, HeartbeatResult,
    JoinMember, JoinRequest, JoinResult, LeaveResult, MemberDescription, SyncRequest, SyncResult,
};
