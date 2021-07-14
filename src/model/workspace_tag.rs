use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
use std::fmt;

use crate::model::backend::SqliteBackend;

#[async_trait]
pub trait WorkspaceTagBackend {
    async fn index_workspace_tag(&self, workspace_id: i64, name: &str, commit_id: &str) -> anyhow::Result<i64>;
    async fn get_workspace_tags(&self, workspace_id: i64) -> anyhow::Result<Vec<WorkspaceTagRecord>>;
}

pub struct WorkspaceTagRecord {
    pub id: i64,
    pub workspace_id: i64,
    pub name: String,
    pub commit_id: String,
}

impl std::fmt::Display for WorkspaceTagRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - {}",
            &self.commit_id,
            &self.name,
        )
    }
}

#[async_trait]
impl WorkspaceTagBackend for SqliteBackend {

    async fn index_workspace_tag(&self, workspace_id: i64, name: &str, commit_id: &str) -> anyhow::Result<i64> {
        let id = sqlx::query!(
            r#"
    INSERT INTO workspace_tag ( workspace_id, name, commit_id )
    VALUES ( ?1, ?2, ?3 )
    ON CONFLICT (workspace_id, name, commit_id) DO NOTHING
            "#,
            workspace_id,
            name,
            commit_id,
        )
        .execute(&*self.pool)
        .await?
        .last_insert_rowid();

        Ok(id)
    }
    // TODO create test so that the unique indexes are done correctly

    async fn get_workspace_tags(&self, workspace_id: i64) -> anyhow::Result<Vec<WorkspaceTagRecord>> {
        let recs = sqlx::query_as!(WorkspaceTagRecord,
            r#"
    SELECT id, workspace_id, name, commit_id
    FROM workspace_tag
    WHERE workspace_id = ?1
            "#,
            workspace_id,
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(recs)
    }

}
