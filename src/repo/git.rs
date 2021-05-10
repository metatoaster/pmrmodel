use anyhow::bail;
use futures::stream::StreamExt;
use futures::stream::futures_unordered::FuturesUnordered;
use std::io::Write;
use git2::{Repository, Object, ObjectType, Tree};
use sqlx::sqlite::SqlitePool;
use std::path::Path;

use crate::model::workspace::WorkspaceRecord;
use crate::model::workspace_sync::{
    WorkspaceSyncStatus,
    begin_sync,
    complete_sync,
    fail_sync,
};
use crate::model::workspace_tag::{index_workspace_tag};


// TODO replace git_root with the struct, or refactor this into a class?
pub async fn git_sync_workspace(pool: &SqlitePool, git_root: &Path, workspace: &WorkspaceRecord) -> anyhow::Result<()> {
    let repo_dir = git_root.join(workspace.id.to_string());
    let repo_check = Repository::open_bare(&repo_dir);

    info!("Syncing local {:?} with remote <{}>...", repo_dir, &workspace.url);
    let sync_id = begin_sync(&pool, workspace.id).await?;
    match repo_check {
        Ok(repo) => {
            info!("Found existing repo at {:?}, synchronizing...", repo_dir);
            let mut remote = repo.find_remote("origin")?;
            match remote.fetch(&[] as &[&str], None, None) {
                Ok(_) => info!("Repository synchronized"),
                Err(e) => fail_sync(&pool, sync_id, format!("Failed to synchronize: {}", e)).await?,
            };
        },
        Err(ref e) if e.class() == git2::ErrorClass::Repository => fail_sync(
            &pool, sync_id, format!(
                "Invalid data at local {:?} - expected bare repo", repo_dir)).await?,
        Err(_) => {
            info!("Cloning new repository at {:?}...", repo_dir);
            let mut builder = git2::build::RepoBuilder::new();
            builder.bare(true);
            match builder.clone(&workspace.url, &repo_dir) {
                Ok(_) => info!("Repository cloned"),
                Err(e) => fail_sync(&pool, sync_id, format!("Failed to clone: {}", e)).await?,
            };
        }
    }

    complete_sync(&pool, sync_id, WorkspaceSyncStatus::Completed).await?;
    index_tags(&pool, &git_root, &workspace).await?;

    Ok(())
}

pub async fn index_tags(pool: &SqlitePool, git_root: &Path, workspace: &WorkspaceRecord) -> anyhow::Result<()> {
    let repo_dir = git_root.join(workspace.id.to_string());
    let repo = Repository::open_bare(&repo_dir)?;

    // collect all the tags for processing later
    let mut tags = Vec::new();
    repo.tag_foreach(|oid, name| {
        // swapped position for next part.
        tags.push((String::from_utf8(name.into()).unwrap(), format!("{}", oid)));
        true
    })?;

    tags.iter().map(|(name, oid)| async move {
        match index_workspace_tag(&pool, workspace.id, &name, &oid).await {
            Ok(_) => info!("indexed tag: {}", name),
            Err(e) => warn!("tagging error: {:?}", e),
        }
    }).collect::<FuturesUnordered<_>>().collect::<Vec<_>>().await;

    Ok(())
}

pub async fn get_blob(pool: &SqlitePool, git_root: &Path, workspace: &WorkspaceRecord, spec: &str) -> anyhow::Result<()> {
    let repo_dir = git_root.join(workspace.id.to_string());
    let repo = Repository::open_bare(&repo_dir)?;
    let obj = repo.revparse_single(spec)?;
    info!("Found object {} {}", obj.kind().unwrap().str(), obj.id());
    Ok(())
}

pub async fn get_pathinfo(pool: &SqlitePool, git_root: &Path, workspace: &WorkspaceRecord, commit_id: Option<&str>, path: Option<&str>, processor: fn(&Object) -> ()) -> anyhow::Result<()> {
    let repo_dir = git_root.join(workspace.id.to_string());
    let repo = Repository::open_bare(&repo_dir)?;
    // TODO the default value should be the default (main?) branch.
    let obj = repo.revparse_single(commit_id.unwrap_or("origin/HEAD"))?;
    // TODO streamline this a bit.
    match obj.kind() {
        Some(ObjectType::Commit) => {
            info!("Found {} {}", obj.kind().unwrap().str(), obj.id());
        }
        Some(_) | None => bail!("'{}' does not refer to a valid commit", commit_id.unwrap_or(""))
    }
    let tree = obj.as_commit().unwrap().tree()?;
    info!("Found tree {}", tree.id());
    // TODO only further navigate into tree_entry if path
    let git_object = match path {
        Some(s) => {
            let tree_entry = tree.get_path(Path::new(s))?;
            info!("Found tree_entry {} {}", tree_entry.kind().unwrap().str(), tree_entry.id());
            tree_entry.to_object(&repo)?
        },
        None => {
            info!("No path provided; using root tree entry");
            tree.into_object()
        }
    };
    info!("using git_object {} {}", git_object.kind().unwrap().str(), git_object.id());
    processor(&git_object);
    Ok(())
}

// a simple function to decode and write result to stream
// TODO break this up.
pub fn stream_object_to_info(mut writer: impl Write, git_object: &Object) -> std::io::Result<()>
{
    writer.write(format!(
        "processing git_object {} {}\n", git_object.kind().unwrap().str(), git_object.id()
    ).as_bytes())?;
    Ok(())
}
