use service_io::connectors::{ImapClient, SmtpClient};
use service_io::engine::Engine;
use service_io::message::util;
use service_io::services::{Alarm, Echo, PublicIp};

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

    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    configure_logger(cli.verbose.log_level_filter()).unwrap();

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
        .map_input(util::service_name_first_char_to_lowercase)
        .add_service("s-echo", Echo)
        .add_service("s-alarm", Alarm)
        .add_service("s-public-ip", PublicIp)
        .run()
        .await;
}

fn configure_logger(level_filter: log::LevelFilter) -> Result<(), log::SetLoggerError> {
    let crate_filter = clap::crate_name!().replace("-", "_");
    fern::Dispatch::new()
        .level(level_filter)
        .filter(move |metadata| metadata.target().starts_with(&crate_filter))
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                record.level(),
                message
            ))
        })
        .chain(std::io::stdout())
        .apply()
}
