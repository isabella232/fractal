use crate::error::Error;
use crate::globals;
use async_trait::async_trait;

use gio::prelude::*;
use matrix_sdk::{
    reqwest, Client as MatrixClient, ClientConfig as MatrixClientConfig, Error as MatrixSdkError,
    HttpError, HttpSend,
};
use url::Url;

use std::convert::TryInto;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Special URI used by gio to indicate no proxy
const PROXY_DIRECT_URI: &str = "direct://";

#[derive(Debug, Default, Eq, PartialEq)]
pub struct ProxySettings {
    http_proxy: Vec<String>,
    https_proxy: Vec<String>,
}

impl ProxySettings {
    fn new<I, T>(http_proxy: T, https_proxy: T) -> Self
    where
        I: PartialEq<str> + Into<String>,
        T: IntoIterator<Item = I>,
    {
        Self {
            http_proxy: http_proxy
                .into_iter()
                .filter(|proxy| proxy != PROXY_DIRECT_URI)
                .map(Into::into)
                .collect(),
            https_proxy: https_proxy
                .into_iter()
                .filter(|proxy| proxy != PROXY_DIRECT_URI)
                .map(Into::into)
                .collect(),
        }
    }

    pub fn current() -> Result<Self, Error> {
        Ok(Self::new(
            PROXY_RESOLVER.with(|resolver| resolver.lookup("http://", gio::NONE_CANCELLABLE))?,
            PROXY_RESOLVER.with(|resolver| resolver.lookup("https://", gio::NONE_CANCELLABLE))?,
        ))
    }

    pub fn apply_to_client_builder(
        &self,
        mut builder: matrix_sdk::reqwest::ClientBuilder,
    ) -> matrix_sdk::reqwest::ClientBuilder {
        // Reqwest only supports one proxy for each type
        if let Some(http_proxy) = self
            .http_proxy
            .get(0)
            .map(reqwest::Proxy::http)
            .and_then(Result::ok)
        {
            builder = builder.proxy(http_proxy);
        }
        if let Some(https_proxy) = self
            .https_proxy
            .get(0)
            .map(reqwest::Proxy::https)
            .and_then(Result::ok)
        {
            builder = builder.proxy(https_proxy);
        }

        builder
    }
}

// gio::ProxyResolver can't be sent or shared
thread_local! {
    static PROXY_RESOLVER: gio::ProxyResolver =
        gio::ProxyResolver::get_default().expect("Couldn't get proxy resolver");
}

#[derive(Debug)]
struct ClientInner {
    client: matrix_sdk::reqwest::Client,
    proxy_settings: ProxySettings,
}

#[derive(Debug)]
pub struct Client {
    inner: Mutex<ClientInner>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ClientInner {
                client: Self::build(matrix_sdk::reqwest::Client::builder()),
                proxy_settings: Default::default(),
            }),
        }
    }

    pub fn get_client(&self) -> matrix_sdk::reqwest::Client {
        // Lock first so we don't overwrite proxy settings with outdated information
        let mut inner = self.inner.lock().unwrap();

        let new_proxy_settings = ProxySettings::current().unwrap_or_default();

        if inner.proxy_settings != new_proxy_settings {
            let mut builder = matrix_sdk::reqwest::Client::builder();
            builder = new_proxy_settings.apply_to_client_builder(builder);
            let client = Self::build(builder);

            inner.client = client;
            inner.proxy_settings = new_proxy_settings;
        }

        inner.client.clone()
    }

    fn build(builder: matrix_sdk::reqwest::ClientBuilder) -> matrix_sdk::reqwest::Client {
        builder
            .timeout(globals::TIMEOUT)
            .build()
            .expect("Couldn't create a http client")
    }
}

#[async_trait]
impl HttpSend for Client {
    async fn send_request(
        &self,
        req: http::Request<Vec<u8>>,
        duration: Option<Duration>,
    ) -> Result<http::Response<Vec<u8>>, HttpError> {
        self.get_client().send_request(req, duration).await
    }
}

pub fn get_matrix_client<U>(homeserver_url: U) -> Result<MatrixClient, MatrixSdkError>
where
    U: TryInto<Url>,
{
    let client = Arc::from(Client::new());
    let config = MatrixClientConfig::new().client(client);
    MatrixClient::new_with_config(homeserver_url, config)
}
