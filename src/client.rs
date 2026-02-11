use crate::storage::HttpRequest;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub async fn execute_request(req_data: &HttpRequest) -> Result<HttpResponse, String> {
    let client = reqwest::Client::new();
    
    let method = reqwest::Method::from_str(&req_data.method)
        .map_err(|e| format!("Invalid method: {}", e))?;
    
    let mut headers = HeaderMap::new();
    for (k, v) in &req_data.headers {
        if let (Ok(name), Ok(value)) = (HeaderName::from_str(k), HeaderValue::from_str(v)) {
            headers.insert(name, value);
        }
    }

    let response = client
        .request(method, &req_data.url)
        .headers(headers)
        .body(req_data.body.clone())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status().as_u16();
    let status_text = response.status().to_string();
    
    let mut res_headers = Vec::new();
    for (name, value) in response.headers() {
        res_headers.push((
            name.to_string(),
            value.to_str().unwrap_or("").to_string()
        ));
    }

    let body = response.text().await.unwrap_or_default();

    Ok(HttpResponse {
        status,
        status_text,
        headers: res_headers,
        body,
    })
}
