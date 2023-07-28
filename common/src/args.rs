//re-export with pub so people that include ::args will include this
pub use clap::Parser;

/// Chess program
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// One of TRACE, DEBUG, INFO, WARN, or ERROR
    #[arg(short, long, default_value = "INFO")]
    pub log_level: log::LevelFilter,
}
