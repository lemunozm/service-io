use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::Service;
use crate::message::Message;

use async_trait::async_trait;
use tokio::time;

use std::time::Duration;

/// Allow to create alarms given a name and a time in minutes.
/// Once the time is over, a response is generated.
pub struct Alarm;

#[async_trait]
impl Service for Alarm {
    async fn run(
        self: Box<Self>,
        mut input: Receiver,
        output: Sender,
    ) -> Result<(), ClosedChannel> {
        loop {
            let request = input.recv().await?;
            let args = request.args.iter().map(|s| s.as_str()).collect::<Vec<_>>();

            if let [name, minutes] = args.as_slice() {
                if let Ok(minutes) = minutes.parse::<u64>() {
                    tokio::spawn({
                        let output = output.clone();
                        let response = Message::response(&request).args([*name]);
                        async move {
                            time::sleep(Duration::from_secs(minutes * 60)).await;
                            output.send(response).await.ok();
                        }
                    });
                    continue;
                }
            }

            let response = Message::response(&request)
                .args(["format error"])
                .body("Expected args: <name> <minutes: POSITIVE_NUMBER>");

            output.send(response).await?;
        }
    }
}
