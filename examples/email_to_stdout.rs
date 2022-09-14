use service_io::connectors::{DebugStdout, ImapClient};
use service_io::engine::Engine;
use service_io::message::util;
use service_io::services::{Alarm, Echo, Process, PublicIp};

use clap::Parser;

use std::time::Duration;

/// Reads emails by imap and show it by the stdout
#[derive(Parser, Debug)]
#[clap()]
struct Cli {
    /// Example: imap.gmail.com
    #[clap(long)]
    imap_domain: String,

    #[clap(long)]
    email: String,

    #[clap(long)]
    secret: String,

    #[clap(long)]
    oauth2: bool,

    #[clap(long, default_value = "", conflicts_with = "password")]
    output: String,

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
                .email(cli.email)
                .secret(cli.secret)
                .oauth2(cli.oauth2)
                .polling_time(Duration::from_secs(cli.polling_time)),
        )
        .output(DebugStdout)
        .map_input(util::service_name_first_char_to_lowercase)
        .add_service("s-echo", Echo)
        .add_service("s-public-ip", PublicIp)
        .add_service("s-alarm", Alarm)
        .add_service("s-process", Process)
        .run()
        .await;
}
