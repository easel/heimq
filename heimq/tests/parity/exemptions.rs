use anyhow::Result;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
pub struct ExemptionEntry {
    pub id: String,
    pub field: String,
    pub scope: String,
    pub reason: String,
    pub prd_ref: String,
    pub status: String,
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
    /// Find an active exemption by exact field name. Returns the exemption id.
    pub fn find(&self, field: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.field == field && e.status == "active")
            .map(|e| e.id.as_str())
    }
}

/// Load and validate exemptions.toml from the parity test directory.
/// Fails fast on duplicate ids or missing prd_ref (HARNESS-001 §Quality Gates).
pub fn load() -> Result<Exemptions> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/parity/exemptions.toml"
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
