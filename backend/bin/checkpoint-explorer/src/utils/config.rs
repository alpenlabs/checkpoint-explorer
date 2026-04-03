use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "Checkpoint explorer",
    author = "Your Name",
    version = "1.0",
    about = "A Checkpoint explorer Application"
)]
pub struct Config {
    /// The URL of the Strata Fullnode
    #[arg(
        long,
        env = "STRATA_FULLNODE",
        default_value = "http://localhost:58000/",
        help = "Strata fullnode URL"
    )]
    pub strata_fullnode: String,

    /// The URL of the MariaDB database
    #[arg(
        long,
        env = "APP_DATABASE_URL",
        default_value = "mysql://root:password@localhost:3306/checkpoint_explorer_db",
        help = "MariaDB database URL"
    )]
    pub database_url: String,

    /// The fetch interval in seconds
    #[arg(
        long,
        env = "APP_FETCH_INTERVAL",
        default_value_t = 30,
        help = "Fetch interval in seconds"
    )]
    pub fetch_interval: u64,

    /// The status update interval in seconds
    #[arg(
        long,
        env = "APP_STATUS_UPDATE_INTERVAL",
        default_value_t = 30,
        help = "Status update interval in seconds"
    )]
    pub status_update_interval: u64,

    #[arg(
        long,
        env = "STRATA_URL",
        default_value = "https://stratabtc.org",
        help = "Strata URL"
    )]
    pub strata_url: String,

    /// The port the HTTP server listens on
    #[arg(
        long,
        env = "APP_SERVER_PORT",
        default_value_t = 3000,
        help = "HTTP server port"
    )]
    pub server_port: u16,
}
