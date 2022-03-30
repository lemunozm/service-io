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
            match public_ip::addr().await {
                Some(ip_addr) => {
                    let response = Message {
                        user: request.user,
                        service_name: request.service_name,
                        body: format!("{}", ip_addr),
                        ..Default::default()
                    };
                    output.send(response).await?;
                }
                None => log::error!("Failed to get IP address"),
            }
        }
    }
}
