use env_logger::{Builder, Env, Target};
use log::error;
use manga_updater::configuration::{Cli, Settings};
use manga_updater::run::run;
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
    let settings = Settings::new(&args.config_file);
    if let Err(e) = settings {
        error!("Configuration error: {}", e);
        process::exit(1);
    }
    let s = settings.unwrap();

    // Run
    if let Err(e) = run(s).await {
        error!("Application error: {}", e);
        process::exit(1);
    }
}
