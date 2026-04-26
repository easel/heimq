//! Postgres-backed durable [`OffsetStore`] implementation.
//!
//! Feature-gated behind `backend-postgres`. Stores committed consumer-group
//! offsets in a single `heimq_committed_offsets` table and survives process
//! restarts.

use crate::error::{HeimqError, Result};
use crate::storage::{CommittedOffset, Durability, OffsetStore, OffsetStoreCapabilities};
use parking_lot::Mutex;
use postgres::{Client, NoTls};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;

const PG_CAPABILITIES: OffsetStoreCapabilities = OffsetStoreCapabilities {
    name: "postgres",
    version: env!("CARGO_PKG_VERSION"),
    durability: Durability::WalFsync,
    survives_restart: true,
};

/// Postgres-backed offset store.
pub struct PostgresOffsetStore {
    client: Mutex<Client>,
    schema: String,
    capabilities: OffsetStoreCapabilities,
}

impl PostgresOffsetStore {
    /// Connect to Postgres and idempotently initialize the schema/table.
    ///
    /// `url` is a libpq-style URL with an optional `?schema=<name>` query
    /// parameter (default: `public`). The `schema=` parameter is stripped
    /// before the URL is handed to the Postgres driver.
    pub fn connect(url: &str) -> Result<Arc<dyn OffsetStore>> {
        let (cleaned_url, schema) = parse_postgres_url(url)?;
        let mut client = Client::connect(&cleaned_url, NoTls).map_err(|e| {
            HeimqError::Storage(format!("postgres connect failed for `{}`: {}", url, e))
        })?;
        Self::initialize(&mut client, &schema)?;
        Ok(Arc::new(Self {
            client: Mutex::new(client),
            schema,
            capabilities: PG_CAPABILITIES,
        }))
    }

    fn initialize(client: &mut Client, schema: &str) -> Result<()> {
        let stmt = format!(
            "CREATE SCHEMA IF NOT EXISTS \"{schema}\"; \
             CREATE TABLE IF NOT EXISTS \"{schema}\".heimq_committed_offsets ( \
                 group_id TEXT NOT NULL, \
                 topic TEXT NOT NULL, \
                 partition INT NOT NULL, \
                 committed_offset BIGINT NOT NULL, \
                 metadata TEXT, \
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
                 PRIMARY KEY (group_id, topic, partition) \
             )",
            schema = schema
        );
        client.batch_execute(&stmt).map_err(|e| {
            HeimqError::Storage(format!("postgres schema initialize failed: {}", e))
        })?;
        Ok(())
    }
}

impl OffsetStore for PostgresOffsetStore {
    fn commit(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
        offset: i64,
        _leader_epoch: i32,
        metadata: Option<String>,
    ) {
        let sql = format!(
            "INSERT INTO \"{schema}\".heimq_committed_offsets \
                 (group_id, topic, partition, committed_offset, metadata) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (group_id, topic, partition) DO UPDATE SET \
                 committed_offset = EXCLUDED.committed_offset, \
                 metadata = EXCLUDED.metadata, \
                 updated_at = now()",
            schema = self.schema
        );
        let mut client = self.client.lock();
        if let Err(e) = client.execute(
            sql.as_str(),
            &[&group_id, &topic, &partition, &offset, &metadata],
        ) {
            error!(error = %e, group = group_id, topic, partition, "postgres offset commit failed");
        }
    }

    fn fetch(&self, group_id: &str, topic: &str, partition: i32) -> Option<CommittedOffset> {
        let sql = format!(
            "SELECT committed_offset, metadata, \
                    (EXTRACT(EPOCH FROM updated_at) * 1000)::BIGINT \
             FROM \"{schema}\".heimq_committed_offsets \
             WHERE group_id = $1 AND topic = $2 AND partition = $3",
            schema = self.schema
        );
        let mut client = self.client.lock();
        match client.query_opt(sql.as_str(), &[&group_id, &topic, &partition]) {
            Ok(Some(row)) => Some(CommittedOffset {
                offset: row.get(0),
                leader_epoch: 0,
                metadata: row.get(1),
                commit_timestamp: row.get(2),
            }),
            Ok(None) => None,
            Err(e) => {
                error!(error = %e, group = group_id, topic, partition, "postgres offset fetch failed");
                None
            }
        }
    }

    fn fetch_all_for_group(
        &self,
        group_id: &str,
    ) -> HashMap<(String, i32), CommittedOffset> {
        let sql = format!(
            "SELECT topic, partition, committed_offset, metadata, \
                    (EXTRACT(EPOCH FROM updated_at) * 1000)::BIGINT \
             FROM \"{schema}\".heimq_committed_offsets \
             WHERE group_id = $1",
            schema = self.schema
        );
        let mut client = self.client.lock();
        let rows = match client.query(sql.as_str(), &[&group_id]) {
            Ok(rows) => rows,
            Err(e) => {
                error!(error = %e, group = group_id, "postgres fetch_all_for_group failed");
                return HashMap::new();
            }
        };
        rows.into_iter()
            .map(|row| {
                let topic: String = row.get(0);
                let partition: i32 = row.get(1);
                let offset: i64 = row.get(2);
                let metadata: Option<String> = row.get(3);
                let commit_timestamp: i64 = row.get(4);
                (
                    (topic, partition),
                    CommittedOffset {
                        offset,
                        leader_epoch: 0,
                        metadata,
                        commit_timestamp,
                    },
                )
            })
            .collect()
    }

    fn delete_group(&self, group_id: &str) {
        let sql = format!(
            "DELETE FROM \"{schema}\".heimq_committed_offsets WHERE group_id = $1",
            schema = self.schema
        );
        let mut client = self.client.lock();
        if let Err(e) = client.execute(sql.as_str(), &[&group_id]) {
            error!(error = %e, group = group_id, "postgres delete_group failed");
        }
    }

    fn capabilities(&self) -> &OffsetStoreCapabilities {
        &self.capabilities
    }
}

/// Split a heimq postgres URL into `(libpq_url, schema_name)`.
///
/// Recognizes a single custom query parameter, `schema=<name>`, which is
/// stripped before the URL is passed to the postgres driver. The schema
/// name is validated to contain only alphanumeric and underscore
/// characters so it is safe to interpolate into DDL.
pub(crate) fn parse_postgres_url(url: &str) -> Result<(String, String)> {
    let mut schema = String::from("public");
    let cleaned = if let Some((base, query)) = url.split_once('?') {
        let mut other: Vec<&str> = Vec::new();
        for part in query.split('&').filter(|p| !p.is_empty()) {
            match part.split_once('=') {
                Some(("schema", value)) => schema = value.to_string(),
                _ => other.push(part),
            }
        }
        if other.is_empty() {
            base.to_string()
        } else {
            format!("{}?{}", base, other.join("&"))
        }
    } else {
        url.to_string()
    };

    if schema.is_empty() || !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(HeimqError::Config(format!(
            "invalid postgres `schema=` value `{}`: must be non-empty and contain only alphanumeric or underscore characters",
            schema
        )));
    }

    Ok((cleaned, schema))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_without_query_uses_public_schema() {
        let (cleaned, schema) =
            parse_postgres_url("postgres://u:p@h:5432/db").unwrap();
        assert_eq!(cleaned, "postgres://u:p@h:5432/db");
        assert_eq!(schema, "public");
    }

    #[test]
    fn parse_url_extracts_schema_param() {
        let (cleaned, schema) =
            parse_postgres_url("postgres://u:p@h:5432/db?schema=heimq").unwrap();
        assert_eq!(cleaned, "postgres://u:p@h:5432/db");
        assert_eq!(schema, "heimq");
    }

    #[test]
    fn parse_url_preserves_other_query_params() {
        let (cleaned, schema) = parse_postgres_url(
            "postgres://u:p@h:5432/db?sslmode=disable&schema=heimq&connect_timeout=5",
        )
        .unwrap();
        assert_eq!(
            cleaned,
            "postgres://u:p@h:5432/db?sslmode=disable&connect_timeout=5"
        );
        assert_eq!(schema, "heimq");
    }

    #[test]
    fn parse_url_rejects_invalid_schema_name() {
        assert!(parse_postgres_url("postgres://h/db?schema=bad;name").is_err());
        assert!(parse_postgres_url("postgres://h/db?schema=").is_err());
    }

    #[test]
    fn parse_url_rejects_quoted_schema() {
        assert!(parse_postgres_url("postgres://h/db?schema=\"x\"").is_err());
    }
}
