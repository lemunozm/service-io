use service_io::connectors::{DebugStdout, UserStdin};
use service_io::engine::Engine;
use service_io::services::Echo;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    Engine::default()
        .input(UserStdin)
        .output(DebugStdout)
        .add_service("s-echo", Echo)
        .run()
        .await;
}
