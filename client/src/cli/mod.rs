pub mod subcommands;

use crate::core::{context::Context, service::client};
use proto::syncx::syncx_client::SyncxClient;
use subcommands::*;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(name = "syncx client", author = "0xphen", version)]
struct Arguments {
    #[clap(subcommand)]
    sub: Subcommands,
}

#[derive(Debug, Subcommand)]
#[clap(
    about = "A CLI client for the Syncx network",
    after_help = "For more information, read the README: https://github.com/0xphen/syncx"
)]
#[allow(clippy::large_enum_variant)]
enum Subcommands {
    #[clap(
        name = "create_account",
        about = "Create an account on the Syncx server"
    )]
    CreateAccount(CreateAccountArgs),
    #[clap(name = "upload", about = "Upload files to the Syncx server")]
    UploadFiles(UploadFilesArgs),
    // #[clap(name = "stream", about = "Captures and live streams network packets")]
    // LiveStream(LiveStreamArgs),
}

pub async fn run(syncx_client: &mut SyncxClient<tonic::transport::Channel>, context: &mut Context) {
    let args = Arguments::parse();
    match args.sub {
        Subcommands::CreateAccount(args) => {
            client::register_client(syncx_client, args.password, context).await;
        }
        Subcommands::UploadFiles(args) => {
            client::upload_files(syncx_client, &args.directory, context).await
        }
    }
}
