use crate::err::WickResult;
use hyper::{Client, body::HttpBody as _, Request, Body};
use hyper::client::connect::HttpConnector;
use hyper_tls::HttpsConnector;
use bytes::BytesMut;

pub struct HttpService {
    client: Client<HttpsConnector<HttpConnector>>,
}

impl HttpService {
    pub fn new() -> Self {
        let https = HttpsConnector::new().unwrap();
        let client = Client::builder().build::<_, hyper::Body>(https);

        Self {
            client,
        }
    }

    async fn process_request(&self, mut response: http::response::Response<Body>) -> WickResult<BytesMut> {
        let content_length: usize = match response.headers().get(hyper::header::CONTENT_LENGTH) {
            Some(val) => val.to_str()?.parse()?,
            None => 0,
        };
        let mut result = BytesMut::with_capacity(std::cmp::max(content_length, 1024));
        while let Some(chunk) = response.body_mut().data().await {
            let chunk = chunk?;
            result.extend_from_slice(&chunk);
        }

        Ok(result)
    }

    pub async fn get_url(&self, url: &str) -> WickResult<BytesMut> {
        let res = self.client.get(url.parse().unwrap()).await?;
        self.process_request(res).await
    }

    pub async fn get_url_string(&self, url: &str) -> WickResult<String> {
        let bytes = self.get_url(url).await?;

        Ok(std::str::from_utf8(&bytes)?.to_owned())
    }

    pub async fn post_url(&self, request: Request<Body>) -> WickResult<BytesMut> {
        let res = self.client.request(request).await?;
        self.process_request(res).await
    }
    
    pub async fn post_url_string(&self, request: Request<Body>) -> WickResult<String> {
        let bytes = self.post_url(request).await?;

        Ok(std::str::from_utf8(&bytes)?.to_owned())
    }
}