use clap::Parser;
use derive_builder::Builder;

#[derive(Debug, Clone, Parser, Builder)]
#[clap(about = "Create an account on the Syncx server")]
pub struct CreateAccountArgs {
    #[clap(required = true)]
    #[clap(long = "password", short = 'p')]
    pub password: String,
}

#[derive(Debug, Clone, Parser, Builder)]
#[clap(about = "Upload a list of files to the Syncx server")]
pub struct UploadFilesArgs {
    #[clap(required = true)]
    #[clap(long = "directory", short = 'd')]
    pub directory: String,
}

#[derive(Debug, Clone, Parser, Builder)]
#[clap(about = "Download a file from the syncx server")]
pub struct DownloadFileArgs {
    #[clap(required = true)]
    #[clap(long = "filename", short = 'f')]
    pub filename: String,

    #[clap(required = true)]
    #[clap(long = "directory", short = 'd')]
    pub directory: String,
}
