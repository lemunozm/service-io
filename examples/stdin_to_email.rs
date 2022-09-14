use service_io::connectors::{SmtpClient, UserStdin};
use service_io::engine::Engine;
use service_io::services::{Alarm, Echo, Process, PublicIp};

use clap::Parser;

/// Creates an email using the first line read by the stdin
/// and send it to the same address as configured by smtp
#[derive(Parser, Debug)]
#[clap()]
struct Cli {
    /// Example: smtp.gmail.com
    #[clap(long)]
    smtp_domain: String,

    #[clap(long)]
    email: String,

    #[clap(long)]
    secret: String,

    #[clap(long)]
    oauth2: bool,

    /// Alias name for 'From' address
    #[clap(long)]
    sender_name: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    Engine::default()
        .input(UserStdin(cli.email.clone()))
        .output(
            SmtpClient::default()
                .domain(cli.smtp_domain)
                .email(cli.email)
                .secret(cli.secret)
                .oauth2(cli.oauth2)
                .sender_name(cli.sender_name),
        )
        .add_service("s-echo", Echo)
        .add_service("s-alarm", Alarm)
        .add_service("s-public-ip", PublicIp)
        .add_service("s-process", Process)
        .run()
        .await;
}
