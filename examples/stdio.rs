use service_io::connectors::{DebugStdout, UserStdin};
use service_io::engine::Engine;
use service_io::services::Echo;

#[tokio::main]
async fn main() {
    Engine::default()
        .input(UserStdin("stdin-user"))
        .output(DebugStdout)
        .add_service("s-echo", Echo)
        .run()
        .await;
}
