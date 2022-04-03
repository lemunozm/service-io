//! Default connectors comming with `service-io`.

mod mpsc;

mod stdin;
pub use stdin::UserStdin;

mod stdout;
pub use stdout::DebugStdout;

mod imap;
pub use self::imap::ImapClient;

mod smtp;
pub use smtp::SmtpClient;
