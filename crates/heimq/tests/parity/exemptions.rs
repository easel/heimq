use anyhow::Result;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
pub struct ExemptionEntry {
    pub id: String,
    pub field: String,
    pub scope: String,
    /// Reference broker this exemption applies to: "all", "redpanda", or "kafka".
    /// Defaults to "all" for entries written before the multi-oracle split.
    #[serde(default = "default_oracle")]
    pub oracle: String,
    /// Human-readable justification — part of the documented TOML schema; not read
    /// by the diff engine (which keys off `field`/`scope`/`oracle`/`status`).
    #[allow(dead_code)]
    pub reason: String,
    pub prd_ref: String,
    pub status: String,
}

fn default_oracle() -> String {
    "all".to_string()
}

#[derive(Debug, Deserialize, Default)]
struct ExemptionsFile {
    #[serde(default)]
    exemption: Vec<ExemptionEntry>,
}

pub struct Exemptions {
    pub entries: Vec<ExemptionEntry>,
}

impl Exemptions {
    /// Find an active exemption by field, workload, and oracle. Scope "all" matches
    /// any workload; oracle "all" matches any reference broker.
    pub fn find(&self, field: &str, workload: &str, oracle: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| {
                e.field == field
                    && e.status == "active"
                    && (e.scope == "all" || e.scope == workload)
                    && (e.oracle == "all" || e.oracle == oracle)
            })
            .map(|e| e.id.as_str())
    }
}

/// Load and validate exemptions.toml from the parity test directory.
/// Fails fast on duplicate ids or missing prd_ref (SD-003 §Quality Gates).
pub fn load() -> Result<Exemptions> {
    // Single source of truth, shared with the containerized conformance runner.
    // This harness is being ported out; see docs/conformance/port-plan.md.
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/conformance/exemptions.toml"
    );

    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Exemptions { entries: vec![] });
        }
        Err(e) => return Err(e.into()),
    };

    let file: ExemptionsFile = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse exemptions.toml: {}", e))?;

    let mut ids = HashSet::new();
    for e in &file.exemption {
        if !ids.insert(e.id.as_str()) {
            anyhow::bail!("Duplicate exemption id: {}", e.id);
        }
        if e.prd_ref.is_empty() {
            anyhow::bail!("Exemption '{}' has empty prd_ref", e.id);
        }
    }

    Ok(Exemptions {
        entries: file.exemption,
    })
}
