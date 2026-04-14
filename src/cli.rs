use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "Photo Dupe Resolver")]
#[command(about = "Resolve duplicate photos using scenarios", long_about = None)]
pub struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,

    /// Database path
    #[arg(short, long)]
    pub db_path: Option<String>,

    /// Number of scanner threads
    #[arg(long)]
    pub scanner_threads: Option<usize>,

    /// Number of hasher threads
    #[arg(long)]
    pub hasher_threads: Option<usize>,

    /// Paths to scan for photos
    #[arg(value_name = "PATH")]
    pub scan_paths: Vec<String>,

    /// Run tests
    #[arg(long)]
    pub test: bool,
}

pub fn parse_args() -> Args {
    Args::parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_args() {
        let args = vec!["photo_dupe_resolver"];
        let parsed = Args::try_parse_from(&args).unwrap();
        assert_eq!(parsed.config, "config.toml");
        assert_eq!(parsed.scan_paths.len(), 0);
    }

    #[test]
    fn test_parse_args_with_paths() {
        let args = vec!["photo_dupe_resolver", "/home/photos", "/media/photos"];
        let parsed = Args::try_parse_from(&args).unwrap();
        assert_eq!(parsed.scan_paths.len(), 2);
        assert_eq!(parsed.scan_paths[0], "/home/photos");
        assert_eq!(parsed.scan_paths[1], "/media/photos");
    }

    #[test]
    fn test_parse_args_with_options() {
        let args = vec![
            "photo_dupe_resolver",
            "--config",
            "custom.toml",
            "--scanner-threads",
            "8",
        ];
        let parsed = Args::try_parse_from(&args).unwrap();
        assert_eq!(parsed.config, "custom.toml");
        assert_eq!(parsed.scanner_threads, Some(8));
    }
}
