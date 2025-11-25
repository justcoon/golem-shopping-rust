use async_trait::async_trait;
use goose::goose::{GooseMethod, GooseRequest, GooseResponse, GooseUser, TransactionError};
use reqwest::header::{HeaderMap, ACCEPT, CONTENT_TYPE, HOST};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[async_trait]
pub trait GooseRequestExt {
    async fn get_request(
        &mut self,
        name: &str,
        path: &str,
    ) -> Result<GooseResponse, Box<TransactionError>>;

    async fn post_request<T: Serialize + Send + Sync>(
        &mut self,
        name: &str,
        path: &str,
        json: &T,
    ) -> Result<GooseResponse, Box<TransactionError>>;

    async fn put_request<T: Serialize + Send + Sync>(
        &mut self,
        name: &str,
        path: &str,
        json: &T,
    ) -> Result<GooseResponse, Box<TransactionError>>;

    async fn delete_request(
        &mut self,
        name: &str,
        path: &str,
    ) -> Result<GooseResponse, Box<TransactionError>>;
}

#[async_trait]
impl GooseRequestExt for GooseUser {
    async fn get_request(
        &mut self,
        name: &str,
        path: &str,
    ) -> Result<GooseResponse, Box<TransactionError>> {
        let request_builder =
            self.get_request_builder(&GooseMethod::Get, path)?.headers(get_headers());

        self.request(
            GooseRequest::builder().set_request_builder(request_builder).name(name).build(),
        )
        .await
    }

    async fn post_request<T: Serialize + Send + Sync>(
        &mut self,
        name: &str,
        path: &str,
        json: &T,
    ) -> Result<GooseResponse, Box<TransactionError>> {
        let request_builder =
            self.get_request_builder(&GooseMethod::Post, path)?.headers(get_headers()).json(json);

        self.request(
            GooseRequest::builder().set_request_builder(request_builder).name(name).build(),
        )
        .await
    }

    async fn put_request<T: Serialize + Send + Sync>(
        &mut self,
        name: &str,
        path: &str,
        json: &T,
    ) -> Result<GooseResponse, Box<TransactionError>> {
        let request_builder =
            self.get_request_builder(&GooseMethod::Put, path)?.headers(get_headers()).json(json);

        self.request(
            GooseRequest::builder().set_request_builder(request_builder).name(name).build(),
        )
        .await
    }

    async fn delete_request(
        &mut self,
        name: &str,
        path: &str,
    ) -> Result<GooseResponse, Box<TransactionError>> {
        let request_builder =
            self.get_request_builder(&GooseMethod::Delete, path)?.headers(get_headers());

        self.request(
            GooseRequest::builder().set_request_builder(request_builder).name(name).build(),
        )
        .await
    }
}

fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    if let Ok(host) = std::env::var("API_HOST") {
        headers.insert(HOST, host.parse().unwrap());
    }
    headers
}

#[async_trait]
pub trait GooseResponseExt {
    async fn json<T: DeserializeOwned>(self) -> Result<T, Box<TransactionError>>;
}

#[async_trait]
impl GooseResponseExt for GooseResponse {
    async fn json<T: DeserializeOwned>(self) -> Result<T, Box<TransactionError>> {
        match self.response {
            Ok(response) => response.json().await.map_err(|e| Box::new(e.into())),
            Err(e) => Err(Box::new(e.into())),
        }
    }
}
