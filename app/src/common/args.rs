use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "INFO")]
    log_level: log::LevelFilter,
}

pub fn get_log_level() -> log::LevelFilter {
    Args::parse().log_level
}
