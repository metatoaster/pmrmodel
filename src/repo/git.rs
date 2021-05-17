use anyhow::bail;
use futures::stream::StreamExt;
use futures::stream::futures_unordered::FuturesUnordered;
use std::io::Write;
use git2::{Repository, Blob, Commit, Object, ObjectType, Tree};
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

pub struct GitResultSet<'a> {
    repo: &'a Repository,
    commit: &'a Commit<'a>,
    path: &'a str,
    object: Object<'a>,
}

// For blob?
#[derive(Debug)]
pub enum ObjectInfo {
    FileInfo {
        path: String,
        basename: String,
        commit_id: String,
        size: u64,
    },
    TreeInfo {
        filecount: u64,
    },
    CommitInfo {
        commit_id: String,
        author: String,
        committer: String,
    },
}


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

pub async fn get_pathinfo(pool: &SqlitePool, git_root: &Path, workspace: &WorkspaceRecord, commit_id: Option<&str>, path: Option<&str>, processor: fn(&GitResultSet) -> ()) -> anyhow::Result<()> {
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
    let commit = obj.as_commit().unwrap();
    let tree = commit.tree()?;
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
    let git_result_set = GitResultSet {
        repo: &repo,
        commit: commit,
        path: path.unwrap_or(""),
        object: git_object,
    };
    processor(&git_result_set);
    Ok(())
}

fn blob_to_info(blob: &Blob) -> ObjectInfo {
    ObjectInfo::FileInfo {
        path: "path".to_string(),
        basename: "basename".to_string(),
        commit_id: "commit_id".to_string(),
        size: blob.size() as u64,
    }
}

fn tree_to_info(tree: &Tree) -> ObjectInfo {
    ObjectInfo::TreeInfo {
        filecount: tree.len() as u64,
    }
}

fn commit_to_info(commit: &Commit) -> ObjectInfo {
    ObjectInfo::CommitInfo {
        commit_id: format!("{}", commit.id()),
        author: format!("{}", commit.author()),
        committer: format!("{}", commit.committer()),
    }
}

pub fn object_to_info(git_object: &Object) -> Option<ObjectInfo> {
    // TODO split off to a formatter version?
    // alternatively, produce some structured data?
    match git_object.kind() {
        Some(ObjectType::Blob) => {
            Some(blob_to_info(git_object.as_blob().unwrap()))
        }
        Some(ObjectType::Tree) => {
            Some(tree_to_info(git_object.as_tree().unwrap()))
        }
        Some(ObjectType::Commit) => {
            Some(commit_to_info(git_object.as_commit().unwrap()))
        }
        Some(ObjectType::Tag) => {
            None
        }
        Some(ObjectType::Any) | None => {
            None
        }
    }
}

pub fn stream_git_result_set(mut writer: impl Write, git_result_set: &GitResultSet) -> () {
    // TODO split off to a formatter version?
    // alternatively, produce some structured data?
    writer.write(format!("
        have repo at {:?}
        have commit {:?}
        have commit_object {:?}
        using repopath {:?}
        have git_object {:?}
        \n",
        git_result_set.repo.path(),
        &git_result_set.commit.id(),
        commit_to_info(&git_result_set.commit),
        git_result_set.path,
        object_to_info(&git_result_set.object),
    ).as_bytes()).unwrap();
}
