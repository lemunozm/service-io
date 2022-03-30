use service_io::connectors::{DebugStdout, UserStdin};
use service_io::engine::Engine;
use service_io::services::{Alarm, Echo, PublicIp};

#[tokio::main]
async fn main() {
    Engine::default()
        .input(UserStdin("stdin-user"))
        .output(DebugStdout)
        .add_service("s-echo", Echo)
        .add_service("s-public-ip", PublicIp)
        .add_service("s-alarm", Alarm)
        .run()
        .await;
}
