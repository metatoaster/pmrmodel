use git2::Object;
use sqlx::sqlite::SqlitePool;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use structopt::StructOpt;

use pmrmodel::model::workspace::{
    add_workspace,
    update_workspace,
    list_workspaces,
    get_workspace_by_id,
};
use pmrmodel::model::workspace_sync::{
    get_workspaces_sync_records
};
use pmrmodel::model::workspace_tag::{
    get_workspace_tags,
};
use pmrmodel::repo::git::{
    GitPmrAccessor,

    git_sync_workspace,
    index_tags,
    get_blob,
    get_pathinfo,

    stream_git_result_set,
};

#[derive(StructOpt)]
struct Args {
    #[structopt(subcommand)]
    cmd: Option<Command>,

    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
}

#[derive(StructOpt)]
enum Command {
    Register {
        url: String,
        description: String,
        #[structopt(short = "l", long = "longdesc", default_value = "")]
        long_description: String,
    },
    Update {
        workspace_id: i64,
        description: String,
        #[structopt(short = "l", long = "longdesc", default_value = "")]
        long_description: String,
    },
    Sync {
        workspace_id: i64,
        #[structopt(short, long)]
        log: bool,
    },
    Tags {
        workspace_id: i64,
        #[structopt(short, long)]
        index: bool,
    },
    Blob {
        workspace_id: i64,
        #[structopt(short, long)]
        obj_id: String,
    },
    Info {
        workspace_id: i64,
        #[structopt(short, long)]
        commit_id: Option<String>,
        #[structopt(short, long)]
        path: Option<String>,
    },
}

fn fetch_envvar(key: &str) -> anyhow::Result<String> {
    match env::var(&key) {
        Err(e) => {
            writeln!(&mut io::stderr(), "couldn't interpret {}: {}", key, e)?;
            process::exit(1);
        },
        Ok(val) => Ok(val),
    }
}

#[async_std::main]
#[paw::main]
async fn main(args: Args) -> anyhow::Result<()> {
    // TODO make this be sourced from a configuration file of sort...
    let git_root = PathBuf::from(fetch_envvar("PMR_GIT_ROOT")?);
    let pool = SqlitePool::connect(&fetch_envvar("DATABASE_URL")?).await?;

    stderrlog::new()
        .module(module_path!())
        .verbosity(args.verbose + 1)
        .timestamp(stderrlog::Timestamp::Second)
        .init()
        .unwrap();

    match args.cmd {
        Some(Command::Register { url, description, long_description }) => {
            println!("Registering workspace with url '{}'...", &url);
            let workspace_id = add_workspace(&pool, &url, &description, &long_description).await?;
            println!("Registered workspace with id {}", workspace_id);
        }
        Some(Command::Update { workspace_id, description, long_description }) => {
            println!("Updating workspace with id {}...", workspace_id);
            if update_workspace(&pool, workspace_id, &description, &long_description).await? {
                println!("Updated workspace id {}", workspace_id);
            }
            else {
                println!("Invalid workspace id {}", workspace_id);
            }
        }
        Some(Command::Sync { workspace_id, log }) => {
            if log {
                println!("Listing of sync logs for workspace with id {}", workspace_id);
                let recs = get_workspaces_sync_records(&pool, workspace_id).await?;
                println!("start - end - status");
                for rec in recs {
                    println!("{}", rec);
                }
            }
            else {
                println!("Syncing commits for workspace with id {}...", workspace_id);
                let workspace = get_workspace_by_id(&pool, workspace_id).await?;
                let git_pmr_accessor = GitPmrAccessor::new(pool, git_root, workspace);
                git_sync_workspace(&git_pmr_accessor).await?;
            }
        }
        Some(Command::Tags { workspace_id, index }) => {
            if index {
                println!("Indexing tags for workspace with id {}...", workspace_id);
                let workspace = get_workspace_by_id(&pool, workspace_id).await?;
                let git_pmr_accessor = GitPmrAccessor::new(pool, git_root, workspace);
                index_tags(&git_pmr_accessor).await?;
            }
            else {
                println!("Listing of indexed tags workspace with id {}", workspace_id);
                let recs = get_workspace_tags(&pool, workspace_id).await?;
                println!("commit_id - tag");
                for rec in recs {
                    println!("{}", rec);
                }
            }
        }
        Some(Command::Blob { workspace_id, obj_id }) => {
            let workspace = get_workspace_by_id(&pool, workspace_id).await?;
            let git_pmr_accessor = GitPmrAccessor::new(pool, git_root, workspace);
            get_blob(&git_pmr_accessor, &obj_id).await?;
        }
        Some(Command::Info { workspace_id, commit_id, path }) => {
            let workspace = get_workspace_by_id(&pool, workspace_id).await?;
            let git_pmr_accessor = GitPmrAccessor::new(pool, git_root, workspace);
            get_pathinfo(
                &git_pmr_accessor, commit_id.as_deref(), path.as_deref(),
                (|git_result_set| stream_git_result_set(io::stdout(), git_result_set))
            ).await?;
        }
        None => {
            println!("Printing list of all workspaces");
            let recs = list_workspaces(&pool).await?;
            println!("id - url - description");
            for rec in recs {
                println!("{}", rec);
            }
        }
    }

    Ok(())
}
