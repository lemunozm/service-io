use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::Service;
use crate::message::Message;

use async_trait::async_trait;

#[derive(Clone)]
pub struct PublicIp;

#[async_trait]
impl Service for PublicIp {
    async fn run(
        self: Box<Self>,
        mut input: Receiver<Message>,
        output: Sender<Message>,
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
