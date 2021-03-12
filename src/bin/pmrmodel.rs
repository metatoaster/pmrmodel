use sqlx::sqlite::SqlitePool;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use structopt::StructOpt;

use pmrmodel::model::workspace::{
    add_workspace,
    update_workspace,
    list_workspaces,
    get_workspaces_by_id,
};
use pmrmodel::model::workspace_sync::{
    get_workspaces_sync_records
};
use pmrmodel::repo::git::{
    git_sync_workspace,
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
        id: i64,
        description: String,
        #[structopt(short = "l", long = "longdesc", default_value = "")]
        long_description: String,
    },
    Sync {
        id: i64,
        #[structopt(short, long)]
        log: bool,
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
    // extend lifetime to scope
    let temp_git_root = fetch_envvar("PMR_GIT_ROOT")?;
    let git_root = Path::new(temp_git_root.as_str());

    let pool = SqlitePool::connect(&fetch_envvar("DATABASE_URL")?).await?;

    stderrlog::new()
        .module(module_path!())
        .verbosity(args.verbose + 1)
        .timestamp(stderrlog::Timestamp::Second)
        .init()
        .unwrap();

    match args.cmd {
        Some(Command::Register { url, description, long_description }) => {
            println!("Registering workspace with url '{}'", &url);
            let workspace_id = add_workspace(&pool, url, description, long_description).await?;
            println!("Registered workspace with id {}", workspace_id);
        }
        Some(Command::Update { id, description, long_description }) => {
            println!("Updating workspace with id {}", id);
            if update_workspace(&pool, id, description, long_description).await? {
                println!("Updated workspace id {}", id);
            }
            else {
                println!("Invalid workspace id {}", id);
            }
        }
        Some(Command::Sync { id, log }) => {
            if log {
                println!("Sync logs for workspace with id {}", id);
                let recs = get_workspaces_sync_records(&pool, id).await?;
                println!("start - end - status");
                for rec in recs {
                    println!("{}", rec);
                }
            }
            else {
                println!("Syncing commits for workspace with id {}", id);
                let workspace = get_workspaces_by_id(&pool, id).await?;
                git_sync_workspace(&pool, &git_root, &workspace).await?;
            }
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
