use clap::{Parser, ValueEnum};
use derive_builder::Builder;

#[derive(Debug, Clone, Parser, Builder)]
#[clap(about = "Create an account on the Syncx server")]
pub struct CreateAccountArgs {
    /// The directory where the packets will be saved
    #[clap(required = true)]
    #[clap(long = "password", short = 'p')]
    pub password: String,
}
