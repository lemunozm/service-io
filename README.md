<p align="center">
  <img src="images/title.png" title="service-io">
</p>

`service-io` is a library to build servers that offering services with really little effort.

1. Choose an input method
2. Choose an output method.
3. Choose your services.
4. Run it!

One of the main use-cases is to offer services [without a hosting server](#no-hosting-server).

### How it works?
<p align="center">
  <img src="images/library-schema.png" title="schema">
</p>

All of them, **inputs** / **outputs** and **services** "speak" the same language:
the [`Message`](https://docs.rs/message-io/latest/service_io/message/Message.html) type.

Inputs obtain and transform input data into `Message`.
Outputs transform a `Message` into output data and deliver it.
Services receive `Message`s and generate other `Message`s usually doing some kind of processing.

Check the current built-in [connectors](https://docs.rs/message-io/latest/service_io/connectors/index.html)
and [services](https://docs.rs/message-io/latest/service_io/services/index.html).

## Features
- **Easy to use**. Running a server with a bunch of services with (really) few lines of code.
- **Hostingless**. Run custom server code without hosting server using the existing email infrastructure
  using the IMAP/SMTP connectors.
- **Scalable**. Create your own inputs/outputs/services implementing a trait with a single method.
  [Check docs](https://docs.rs/message-io/latest/service_io/interface/index.html)
- **Multiplatform**. Run your local service-server in any computer you have.

## Getting Started
- [API Docs](https://docs.rs/message-io/latest/service_io/)
- [Examples](examples)

Add the following to your `Cargo.toml`
```toml
service-io = "0.1"
```

## No hosting server use-case <span id="no-hosting-server"/>
If you want to offer some custom service that uses *custom server code*
you are forced to pay and maintain a hosting server,
even if the service you are offering is eventual or does not use many resources.

To solve this problem, you can use the already existent email infrastructure
using the IMAP and SMTP protocols to handle the emails as requests / responses and link them with your services.

`service-io` helps in this context.
Run locally an instance of `service-io` with IMAP/SMTP connectors.
The IMAP connector will fetch periodically the emails your clients sends,
then your services will process those emails and generate a response,
and finally the SMTP connector will deliver the response emails back to the user.

Anyone from any device with an email client can interact with your local server deployment.
There is **no hosting maintenance** and **no front-end app development**.

// Image

## Example
Simply send an email with `public-ip` in the subject and you will obtain a response with your public ip!

```rust,no_run
use service_io::engine::Engine;
use service_io::connectors::{ImapClient, SmtpClient};
use service_io::services::PublicIp;

#[tokio::main]
async fn main() {
    Engine::default()
        .input(
            ImapClient::default()
                .domain("imap.domain.com")
                .email("services@domain.com")
                .password("1234")
        )
        .output(
            SmtpClient::default()
                .domain("smtp.domain.com")
                .email("services@domain.com")
                .password("1234")
        )
        .add_service("public-ip", PublicIp) // Add any other service you want
        .run()
        .await;
}
```

Any email sent to `services@domain.com` will be interpreted as a request by the `ImapClient` connector.
If the first word of the subject matches `public-ip`, the request will be processed by the `PublicIp` service.
The service `PublicIp` will generate a response that `SmtpClient` will be delivered by email
to the originator of the request email.

Check the [Engine](https://docs.rs/message-io/latest/service_io/interface/index.html) type
for additional methods as input mapping/filters or adding whitelists to your services.

Test it yourself with [examples/email_server.rs](examples/email_server.rs).
Run `cargo run --example email_server -- --help` to see all config options.

## Configuring a gmail account to use with `service-io`.
For use `service-io` with IMAP and SMTP connectors with gmail you need to configure some points
of your gmail account:
- Enable IMAP in account settings: Check this [Step 1](https://support.google.com/mail/answer/7126229?hl=en#zippy=%2Cpaso-comprueba-que-imap-est%C3%A9-activado%2Cstep-check-that-imap-is-turned-on).
- Enable [unsecure app access](https://support.google.com/accounts/answer/6010255?hl=en)
  to allow login with password from an app.
  (pending work to make it available through *oauth2* and avoid this point).

## Contribute
- *Have you implemented a **service** or **connector**?*
  If its functionallity is not private, share it with others!
  Make a *Pull Request* so everyone can use it :)

- *Do you have any cool idea, found a bug or have any question/doubt?*
  Do not hesitate and open an issue!
