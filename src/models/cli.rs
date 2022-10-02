use clap::Parser;

#[derive(clap::Parser)]
pub struct Cli {
    #[arg(short, long, default_value = "manga")]
    pub config_file: String,
}

impl Cli {
    pub fn new() -> Self {
        Cli::parse()
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_test() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
