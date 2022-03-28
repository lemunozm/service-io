use service_io::connectors::{DebugStdout, ImapClient};
use service_io::engine::Engine;
use service_io::services::Echo;

use clap::Parser;

use std::time::Duration;

#[derive(Parser, Debug)]
#[clap()]
struct Cli {
    /// Example: imap.gmail.com
    #[clap(long)]
    imap_domain: String,

    #[clap(long)]
    email: String,

    #[clap(long)]
    password: String,

    /// Waiting time (in secs) to make request to the imap server
    #[clap(long, default_value = "3")]
    polling_time: u64,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    Engine::default()
        .input(
            ImapClient::default()
                .domain(cli.imap_domain)
                .email(&cli.email)
                .password(cli.password)
                .polling_time(Duration::from_secs(cli.polling_time)),
        )
        .output(DebugStdout)
        .add_service("s-echo", Echo)
        .run()
        .await;
}
