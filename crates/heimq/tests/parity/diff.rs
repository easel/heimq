use crate::driver::{Observation, ObservationEvent};
use crate::exemptions::Exemptions;
pub use heimq::test_support::DiffRecord;

/// Diff two normalized observation lists from the same workload: heimq against a
/// single reference broker (`oracle`, e.g. "redpanda" or "kafka").
/// Returns one DiffRecord per diverging field.
pub fn diff(
    workload: &str,
    oracle: &str,
    heimq: &[Observation],
    oracle_obs: &[Observation],
    exemptions: &Exemptions,
) -> Vec<DiffRecord> {
    let mut diffs = Vec::new();

    // Length mismatch: report missing/extra at each step beyond the shared prefix.
    let shared = heimq.len().min(oracle_obs.len());
    for i in shared..heimq.len() {
        diffs.push(DiffRecord {
            workload: workload.to_string(),
            oracle: oracle.to_string(),
            step: heimq[i].step,
            field: "observation".to_string(),
            heimq_value: serde_json::json!(format!("{:?}", event_kind(&heimq[i].event))),
            oracle_value: serde_json::Value::Null,
            divergence: "extra_in_heimq".to_string(),
            exemption: None,
        });
    }
    for i in shared..oracle_obs.len() {
        diffs.push(DiffRecord {
            workload: workload.to_string(),
            oracle: oracle.to_string(),
            step: oracle_obs[i].step,
            field: "observation".to_string(),
            heimq_value: serde_json::Value::Null,
            oracle_value: serde_json::json!(format!("{:?}", event_kind(&oracle_obs[i].event))),
            divergence: "missing_in_heimq".to_string(),
            exemption: None,
        });
    }

    // Per-step field comparison.
    for (h, r) in heimq.iter().zip(oracle_obs.iter()) {
        diff_events(workload, oracle, h.step, &h.event, &r.event, exemptions, &mut diffs);
    }

    diffs
}

fn event_kind(e: &ObservationEvent) -> &'static str {
    match e {
        ObservationEvent::RecordConsumed { .. } => "RecordConsumed",
        ObservationEvent::ErrorCode { .. } => "ErrorCode",
        ObservationEvent::GroupState { .. } => "GroupState",
        ObservationEvent::TxnOutcome { .. } => "TxnOutcome",
    }
}

fn diff_events(
    workload: &str,
    oracle: &str,
    step: u32,
    h: &ObservationEvent,
    r: &ObservationEvent,
    exemptions: &Exemptions,
    out: &mut Vec<DiffRecord>,
) {
    match (h, r) {
        (
            ObservationEvent::RecordConsumed {
                key: hk,
                value: hv,
                partition: hp,
                offset: ho,
                ..
            },
            ObservationEvent::RecordConsumed {
                key: rk,
                value: rv,
                partition: rp,
                offset: ro,
                ..
            },
        ) => {
            maybe_diff_bytes(workload, oracle, step, "record.key", hk.as_deref(), rk.as_deref(), exemptions, out);
            maybe_diff_bytes(workload, oracle, step, "record.value", hv.as_deref(), rv.as_deref(), exemptions, out);
            maybe_diff(workload, oracle, step, "record.partition", hp, rp, exemptions, out);
            maybe_diff(workload, oracle, step, "record.offset", ho, ro, exemptions, out);
        }
        (
            ObservationEvent::GroupState {
                state: hs,
                member_count: hm,
                ..
            },
            ObservationEvent::GroupState {
                state: rs,
                member_count: rm,
                ..
            },
        ) => {
            maybe_diff(workload, oracle, step, "group_state.state", hs, rs, exemptions, out);
            maybe_diff(workload, oracle, step, "group_state.member_count", hm, rm, exemptions, out);
        }
        (
            ObservationEvent::ErrorCode { api: ha, code: hc },
            ObservationEvent::ErrorCode { api: ra, code: rc },
        ) => {
            maybe_diff(workload, oracle, step, "error_code.api", ha, ra, exemptions, out);
            maybe_diff(workload, oracle, step, "error_code.code", hc, rc, exemptions, out);
        }
        _ => {
            // Event type mismatch.
            out.push(DiffRecord {
                workload: workload.to_string(),
                oracle: oracle.to_string(),
                step,
                field: "event_type".to_string(),
                heimq_value: serde_json::json!(event_kind(h)),
                oracle_value: serde_json::json!(event_kind(r)),
                divergence: "value_mismatch".to_string(),
                exemption: None,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn maybe_diff<T>(
    workload: &str,
    oracle: &str,
    step: u32,
    field: &str,
    h: &T,
    r: &T,
    exemptions: &Exemptions,
    out: &mut Vec<DiffRecord>,
) where
    T: PartialEq + serde::Serialize,
{
    if h == r {
        return;
    }
    let exemption = exemptions.find(field, workload, oracle).map(str::to_string);
    out.push(DiffRecord {
        workload: workload.to_string(),
        oracle: oracle.to_string(),
        step,
        field: field.to_string(),
        heimq_value: serde_json::to_value(h).unwrap_or(serde_json::Value::Null),
        oracle_value: serde_json::to_value(r).unwrap_or(serde_json::Value::Null),
        divergence: "value_mismatch".to_string(),
        exemption,
    });
}

#[allow(clippy::too_many_arguments)]
fn maybe_diff_bytes(
    workload: &str,
    oracle: &str,
    step: u32,
    field: &str,
    h: Option<&[u8]>,
    r: Option<&[u8]>,
    exemptions: &Exemptions,
    out: &mut Vec<DiffRecord>,
) {
    if h == r {
        return;
    }
    let exemption = exemptions.find(field, workload, oracle).map(str::to_string);
    let to_val = |b: Option<&[u8]>| match b {
        None => serde_json::Value::Null,
        Some(b) => serde_json::Value::String(String::from_utf8_lossy(b).into_owned()),
    };
    out.push(DiffRecord {
        workload: workload.to_string(),
        oracle: oracle.to_string(),
        step,
        field: field.to_string(),
        heimq_value: to_val(h),
        oracle_value: to_val(r),
        divergence: "value_mismatch".to_string(),
        exemption,
    });
}
