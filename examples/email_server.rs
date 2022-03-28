use service_io::connectors::{ImapClient, SmtpClient};
use service_io::engine::Engine;
use service_io::services::Echo;

use clap::Parser;

use std::time::Duration;

/// Emulate a server: reads emails by imap as requests and send emails by stmp as responses.
#[derive(Parser, Debug)]
#[clap()]
struct Cli {
    /// Example: imap.gmail.com
    #[clap(long)]
    imap_domain: String,

    /// Example: smtp.gmail.com
    #[clap(long)]
    smtp_domain: String,

    #[clap(long)]
    email: String,

    #[clap(long)]
    password: String,

    /// Waiting time (in secs) to make request to the imap server
    #[clap(long, default_value = "3")]
    polling_time: u64,

    /// Alias name for 'From' address
    #[clap(long)]
    sender_name: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    Engine::default()
        .input(
            ImapClient::default()
                .domain(cli.imap_domain)
                .email(cli.email.clone())
                .password(cli.password.clone())
                .polling_time(Duration::from_secs(cli.polling_time)),
        )
        .output(
            SmtpClient::default()
                .domain(cli.smtp_domain)
                .email(cli.email)
                .password(cli.password)
                .sender_name(cli.sender_name),
        )
        .add_service("s-echo", Echo)
        .run()
        .await;
}
