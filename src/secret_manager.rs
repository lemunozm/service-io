use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AccessToken, AuthUrl, ClientId, ClientSecret,
    RefreshToken, TokenResponse, TokenUrl,
};

use async_trait::async_trait;
use tokio::sync::Mutex;

use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SecretType {
    Password,
    AccessToken,
}

#[async_trait]
pub trait SecretManager {
    fn secret_type(&self) -> SecretType;
    async fn secret(&self) -> String;
    async fn refresh(&mut self);
}

pub struct PasswordManager {
    pub password: String,
}

impl PasswordManager {
    pub fn new(password: impl Into<String>) -> Self {
        Self {
            password: password.into(),
        }
    }
}

#[async_trait]
impl SecretManager for PasswordManager {
    fn secret_type(&self) -> SecretType {
        SecretType::Password
    }

    async fn secret(&self) -> String {
        self.password.clone()
    }

    async fn refresh(&mut self) {}
}

pub struct Oauth2Manager {
    auth_url: AuthUrl,
    token_url: TokenUrl,
    client_id: ClientId,
    client_secret: ClientSecret,
    refresh_token: RefreshToken,
    access_token: AccessToken,
}

impl Oauth2Manager {
    pub async fn new(
        auth_url: impl Into<String>,
        token_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        refresh_token: impl Into<String>,
    ) -> Self {
        let mut this = Self {
            auth_url: AuthUrl::new(auth_url.into()).unwrap(),
            token_url: TokenUrl::new(token_url.into()).unwrap(),
            client_id: ClientId::new(client_id.into()),
            client_secret: ClientSecret::new(client_secret.into()),
            refresh_token: RefreshToken::new(refresh_token.into()),
            access_token: AccessToken::new("".into()),
        };

        this.refresh().await;
        this
    }
}

#[async_trait]
impl SecretManager for Oauth2Manager {
    fn secret_type(&self) -> SecretType {
        SecretType::AccessToken
    }

    async fn secret(&self) -> String {
        self.access_token.secret().clone()
    }

    async fn refresh(&mut self) {
        let client = BasicClient::new(
            self.client_id.clone(),
            Some(self.client_secret.clone()),
            self.auth_url.clone(),
            Some(self.token_url.clone()),
        );

        let response = client
            .exchange_refresh_token(&self.refresh_token)
            .request_async(async_http_client)
            .await
            .unwrap();

        self.access_token = response.access_token().clone();
    }
}

#[derive(Clone)]
pub struct SecretHandler {
    manager: Arc<Mutex<dyn SecretManager + Sync + Send>>,
    secret_type: SecretType,
}

impl SecretHandler {
    pub fn new(manager: impl SecretManager + Sync + Send + 'static) -> Self {
        Self {
            secret_type: manager.secret_type(),
            manager: Arc::new(Mutex::new(manager)),
        }
    }
}

#[async_trait]
impl SecretManager for SecretHandler {
    fn secret_type(&self) -> SecretType {
        self.secret_type
    }

    async fn secret(&self) -> String {
        self.manager.lock().await.secret().await
    }

    async fn refresh(&mut self) {
        self.manager.lock().await.refresh().await;
    }
}
