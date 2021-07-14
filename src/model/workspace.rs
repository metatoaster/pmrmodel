use async_trait::async_trait;
use chrono::Utc;
use std::fmt;

use crate::model::backend::SqliteBackend;

#[async_trait]
pub trait WorkspaceBackend {
    async fn add_workspace(
        &self, url: &str, description: &str, long_description: &str
    ) -> anyhow::Result<i64>;
    async fn update_workspace(
        &self, id: i64, description: &str, long_description: &str
    ) -> anyhow::Result<bool>;
    async fn list_workspaces(&self) -> anyhow::Result<Vec<WorkspaceRecord>>;
    async fn get_workspace_by_id(&self, id: i64) -> anyhow::Result<WorkspaceRecord>;
}

pub struct WorkspaceRecord {
    pub id: i64,
    pub url: String,
    pub description: Option<String>,
}

impl std::fmt::Display for WorkspaceRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - {} - {}",
            self.id,
            &self.url,
            match &self.description {
                Some(v) => v,
                None => "<empty>",
            },
        )
    }
}

#[async_trait]
impl WorkspaceBackend for SqliteBackend {
    async fn add_workspace(&self, url: &str, description: &str, long_description: &str) -> anyhow::Result<i64> {
        let ts = Utc::now().timestamp();

        let id = sqlx::query!(
            r#"
INSERT INTO workspace ( url, description, long_description, created )
VALUES ( ?1, ?2, ?3, ?4 )
            "#,
            url,
            description,
            long_description,
            ts,
        )
        .execute(&*self.pool)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    async fn update_workspace(&self, id: i64, description: &str, long_description: &str) -> anyhow::Result<bool> {
        let rows_affected = sqlx::query!(
            r#"
UPDATE workspace
SET description = ?1, long_description = ?2
WHERE id = ?3
            "#,
            description,
            long_description,
            id,
        )
        .execute(&*self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    async fn list_workspaces(&self) -> anyhow::Result<Vec<WorkspaceRecord>> {
        let recs = sqlx::query_as!(WorkspaceRecord,
            r#"
SELECT id, url, description
FROM workspace
ORDER BY id
            "#
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(recs)
    }

    async fn get_workspace_by_id(&self, id: i64) -> anyhow::Result<WorkspaceRecord> {
        // ignoring superceded_by_id for now?
        let rec = sqlx::query_as!(WorkspaceRecord,
            r#"
SELECT id, url, description
FROM workspace
WHERE id = ?1
            "#,
            id,
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(rec)
    }
}
