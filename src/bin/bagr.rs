use std::path::PathBuf;
use std::process::exit;

use bagr::bagit::{create_bag, open_bag, DigestAlgorithm};
use clap::AppSettings::UseLongFormatForHelpSubcommand;
use clap::{Args, Parser, Subcommand};
use log::{error, info, LevelFilter};

// TODO expand docs

/// A CLI for interacting with BagIt bags
#[derive(Debug, Parser)]
#[clap(name = "bagr", author = "Peter Winckles <pwinckles@pm.me>", version)]
#[clap(setting(UseLongFormatForHelpSubcommand))]
pub struct BagrArgs {
    /// Absolute or relative path to the bag's base directory
    ///
    /// By default, this is the current directory.
    #[clap(short, long, value_name = "BAG_PATH")]
    pub bag_path: Option<PathBuf>,

    /// Suppress error messages and other command specific logging
    #[clap(short, long)]
    pub quiet: bool,

    /// Increase log level
    #[clap(short = 'V', long)]
    pub verbose: bool,

    /// Disable all output styling
    #[clap(short = 'S', long)]
    pub no_styles: bool,

    /// Subcommand to execute
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[clap(name = "bag")]
    Bag(BagCmd),
    #[clap(name = "rebag")]
    Rebag(RebagCmd),
}

/// Create a new bag
#[derive(Args, Debug)]
pub struct BagCmd {}

/// Update BagIt manifests to match the current state on disk
#[derive(Args, Debug)]
pub struct RebagCmd {}

fn main() {
    let mut args = BagrArgs::parse();

    let log_level = if args.quiet {
        LevelFilter::Off
    } else if args.verbose {
        LevelFilter::Info
    } else {
        LevelFilter::Warn
    };

    env_logger::builder()
        .filter_level(log_level)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();

    // If the output is being piped then we should disable styling
    if atty::isnt(atty::Stream::Stdout) {
        args.no_styles = true;
    }

    // TODO
    match args.command {
        Command::Bag(_) => {
            let algorithms = &[DigestAlgorithm::Md5, DigestAlgorithm::Sha256];

            if let Err(e) = create_bag(".", algorithms) {
                error!("Failed to create bag: {}", e);
                exit(1);
            }
        }
        Command::Rebag(_) => match open_bag(".") {
            Ok(bag) => {
                info!("Opened bag: {:?}", bag);
            }
            Err(e) => {
                error!("Failed to rebag: {}", e);
                exit(1);
            }
        },
    }
}
