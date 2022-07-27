use service_io::connectors::{imap, DebugStdout, ImapClient};
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

    #[clap(long, default_value = "", conflicts_with = "access-token")]
    password: String,

    #[clap(long, default_value = "", conflicts_with = "password")]
    access_token: String,

    /// Waiting time (in secs) to make request to the imap server
    #[clap(long, default_value = "3")]
    polling_time: u64,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let imap_access = match cli.access_token.is_empty() {
        true => imap::Access::Password(cli.password),
        false => imap::Access::OAuth2(cli.access_token),
    };

    Engine::default()
        .input(
            ImapClient::default()
                .domain(cli.imap_domain)
                .email(cli.email)
                .access(imap_access)
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
