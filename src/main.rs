use clap::Parser;
use env_logger::{Builder, Env, Target};
use log::error;
use manga_updater::configuration::Settings;
use manga_updater::run::run;
use std::process;

#[derive(clap::Parser)]
struct Args {
    #[clap(short, long, default_value = "manga.json")]
    config_file: String,
}

#[tokio::main]
async fn main() {
    // Init logging
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));
    builder.target(Target::Stdout);
    builder.init();

    // Parse Args
    let args = Args::parse();

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn clap_test() {
        use clap::CommandFactory;
        Args::command().debug_assert()
    }
}
