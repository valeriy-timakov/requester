use crate::storage::HttpRequest;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;
use std::error::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub async fn execute_request(req_data: &HttpRequest) -> Result<HttpResponse, String> {
    println!("\n========== HTTP REQUEST ==========");
    println!("Method: {}", req_data.method);
    println!("URL: {}", req_data.url);

    let client = reqwest::Client::new();

    let method = reqwest::Method::from_str(&req_data.method)
        .map_err(|e| {
            let err = format!("Invalid method: {}", e);
            eprintln!("ERROR: {}", err);
            err
        })?;

    println!("\nRequest Headers:");
    let mut headers = HeaderMap::new();
    for (k, v) in &req_data.headers {
        match (HeaderName::from_str(k), HeaderValue::from_str(v)) {
            (Ok(name), Ok(value)) => {
                println!("  {}: {}", k, v);
                headers.insert(name, value);
            },
            (Err(e), _) => {
                eprintln!("  WARNING: Invalid header name '{}': {}", k, e);
            },
            (_, Err(e)) => {
                eprintln!("  WARNING: Invalid header value for '{}': {}", k, e);
            }
        }
    }

    if !req_data.body.is_empty() {
        println!("\nRequest Body ({} bytes):", req_data.body.len());
        if req_data.body.len() <= 1000 {
            println!("{}", req_data.body);
        } else {
            println!("{}... [truncated]", &req_data.body[..1000]);
        }
    } else {
        println!("\nRequest Body: (empty)");
    }

    println!("\n--- Sending request ---");

    let response = client
        .request(method, &req_data.url)
        .headers(headers)
        .body(req_data.body.clone())
        .send()
        .await
        .map_err(|e| {
            let err_msg = if e.is_timeout() {
                format!("Request timeout: {}", e)
            } else if e.is_connect() {
                format!("Connection error: {} (Check if server is reachable)", e)
            } else if e.is_request() {
                format!("Request error: {} (Check URL and request format)", e)
            } else if e.is_redirect() {
                format!("Redirect error: {}", e)
            } else if e.is_decode() {
                format!("Response decode error: {}", e)
            } else {
                format!("Network error: {}", e)
            };
            eprintln!("\nERROR: {}", err_msg);

            // Additional debugging info
            if let Some(url) = e.url() {
                eprintln!("Failed URL: {}", url);
            }
            if let Some(source) = e.source() {
                eprintln!("Error source: {}", source);
            }

            err_msg
        })?;

    let status = response.status().as_u16();
    let status_text = response.status().to_string();

    println!("\n========== HTTP RESPONSE ==========");
    println!("Status: {} {}", status, status_text);

    println!("\nResponse Headers:");
    let mut res_headers = Vec::new();
    for (name, value) in response.headers() {
        let value_str = value.to_str().unwrap_or("<binary>");
        println!("  {}: {}", name, value_str);
        res_headers.push((
            name.to_string(),
            value_str.to_string()
        ));
    }

    let body = response.text().await
        .map_err(|e| {
            let err = format!("Failed to read response body: {}", e);
            eprintln!("ERROR: {}", err);
            err
        })?;

    println!("\nResponse Body ({} bytes):", body.len());
    if body.len() <= 1000 {
        println!("{}", body);
    } else {
        println!("{}... [truncated]", &body[..1000]);
    }
    println!("==================================\n");

    Ok(HttpResponse {
        status,
        status_text,
        headers: res_headers,
        body,
    })
}
