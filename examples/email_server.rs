use service_io::connectors::{ImapClient, SmtpClient};
use service_io::engine::Engine;
use service_io::message::util;
use service_io::secret_manager::{Oauth2Manager, PasswordManager, SecretHandler};
use service_io::services::{Alarm, Echo, Process, PublicIp};

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

    #[clap(long, default_value = "")]
    password: String,

    #[clap(long, default_value = "")]
    oauth2_path_url: String,

    #[clap(long, default_value = "")]
    oauth2_token_url: String,

    #[clap(long, default_value = "")]
    oauth2_client_id: String,

    #[clap(long, default_value = "")]
    oauth2_client_secret: String,

    #[clap(long, default_value = "")]
    oauth2_refresh_token: String,

    /// Waiting time (in secs) to make requests to the imap server
    #[clap(long, default_value = "3")]
    polling_time: u64,

    /// Alias name for 'From' address when send an email.
    #[clap(long)]
    sender_name: Option<String>,

    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    configure_logger(cli.verbose.log_level_filter()).unwrap();

    let secret_handler = match cli.password.is_empty() {
        false => SecretHandler::new(PasswordManager::new(cli.password)),
        true => SecretHandler::new(
            Oauth2Manager::new(
                cli.oauth2_path_url,
                cli.oauth2_token_url,
                cli.oauth2_client_id,
                cli.oauth2_client_secret,
                cli.oauth2_refresh_token,
            )
            .await,
        ),
    };

    Engine::default()
        .input(
            ImapClient::default()
                .domain(cli.imap_domain)
                .email(&cli.email)
                .secret_manager(secret_handler.clone())
                .polling_time(Duration::from_secs(cli.polling_time)),
        )
        .output(
            SmtpClient::default()
                .domain(cli.smtp_domain)
                .email(cli.email)
                .secret_manager(secret_handler)
                .sender_name(cli.sender_name),
        )
        .map_input(util::service_name_first_char_to_lowercase)
        .add_service("s-echo", Echo)
        .add_service("s-alarm", Alarm)
        .add_service("s-public-ip", PublicIp)
        .add_service("s-process", Process)
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
                "[{}] [{}] {}: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                record.level(),
                record.target(),
                message
            ))
        })
        .chain(std::io::stdout())
        .apply()
}
