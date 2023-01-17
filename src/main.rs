use env_logger::{Builder, Env, Target};
use log::error;
use manga_updater::run;
use manga_updater::{Cli, Settings};
use std::process;

#[tokio::main]
async fn main() {
    // Init logging
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));
    builder.target(Target::Stdout);
    builder.init();

    // Parse Args
    let args = Cli::new();

    // Parse Settings
    let Ok(settings) = Settings::new(&args.config_file) else {
        error!("Configuration error parsing: {settings}");
        process::exit(1);
    };

    // Run
    if let Err(e) = run(settings).await {
        error!("Application error: {}", e);
        process::exit(1);
    }
}
