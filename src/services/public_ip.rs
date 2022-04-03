use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::Service;
use crate::message::Message;

use async_trait::async_trait;

pub struct PublicIp;

#[async_trait]
impl Service for PublicIp {
    async fn run(
        self: Box<Self>,
        mut input: Receiver,
        output: Sender,
    ) -> Result<(), ClosedChannel> {
        loop {
            let request = input.recv().await?;
            let response = match public_ip::addr().await {
                Some(ip_addr) => Message::response(&request).body(format!("{}", ip_addr)),
                None => {
                    let msg = "Failed to get IP address";
                    log::error!("{}", msg);
                    Message::response(&request).args(["error"]).body(msg)
                }
            };
            output.send(response).await?;
        }
    }
}
