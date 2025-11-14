use anyhow::Result;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt;

use crate::cli::GlobalOptions;

pub fn init(options: &GlobalOptions) -> Result<()> {
    let level = if options.quiet {
        LevelFilter::ERROR
    } else {
        match options.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        }
    };

    let subscriber = fmt()
        .with_max_level(level)
        .with_target(false)
        .with_writer(std::io::stderr)
        .without_time()
        .finish();

    // Ignore error if subscriber was already set (e.g., in tests).
    let _ = tracing::subscriber::set_global_default(subscriber);

    Ok(())
}
