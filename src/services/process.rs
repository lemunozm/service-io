use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::Service;
use crate::message::Message;

use async_trait::async_trait;
use tokio::process::Command;

/// Allow to run any process.
/// Each arg of the message is interpreted as a process arg, being arg0 the name of the process.
/// The stdout of the process once finalized will be returned as a message body.
pub struct Process;

#[async_trait]
impl Service for Process {
    async fn run(
        self: Box<Self>,
        mut input: Receiver,
        output: Sender,
    ) -> Result<(), ClosedChannel> {
        loop {
            let request = input.recv().await?;
            match request.args.get(0) {
                Some(_) => spawn_process(request, output.clone()),
                None => {
                    let response = Message::response(&request)
                        .args(["format error"])
                        .body(format!("You need to specify a process to run"));

                    output.send(response).await?;
                }
            }
        }
    }
}

fn spawn_process(request: Message, output: Sender) {
    let mut program_args = request.args.iter();
    let arg0 = program_args.next().unwrap();
    let child = Command::new(arg0).args(program_args).output();

    tokio::spawn({
        let output = output.clone();
        async move {
            let cmd_str = request.args.join(" ");
            if let Ok(child_output) = child.await {
                let response = Message::response(&request)
                    .args([format!("Terminated ({}): {}", child_output.status, cmd_str)])
                    .body(std::str::from_utf8(&child_output.stdout).unwrap_or("[binary]"));

                output.send(response).await.ok();
            } else {
                let response = Message::response(&request)
                    .args(["error"])
                    .body(format!("Error while running: {}", cmd_str));

                output.send(response).await.ok();
            }
        }
    });
}
