use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::Service;
use crate::message::Message;

use async_trait::async_trait;
use tokio::process::Command;

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

            let mut program_args = request.args.iter();
            match program_args.next() {
                Some(arg0) => {
                    let child = Command::new(arg0).args(program_args).output();
                    tokio::spawn({
                        let output = output.clone();
                        async move {
                            let cmd_str = request.args.join(" ");
                            if let Ok(child_output) = child.await {
                                let response = Message::response(&request)
                                    .args([format!(
                                        "Terminated ({}): {}",
                                        child_output.status, cmd_str
                                    )])
                                    .body(
                                        std::str::from_utf8(&child_output.stdout)
                                            .unwrap_or("[binary]"),
                                    );

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
                None => {
                    let response = Message::response(&request)
                        .args(["format error"])
                        .body(format!("You need to specify a process to run"));

                    output.send(response).await.ok();
                }
            }
        }
    }
}
