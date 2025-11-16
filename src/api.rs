use std::collections::HashMap;
use std::env;
use std::fmt::Debug;

use anyhow::{bail, Context, Result};
#[cfg(not(feature = "no-internet"))]
use reqwest::{header, Response};
#[cfg(not(feature = "no-internet"))]
use reqwest::{multipart::Form, Client, RequestBuilder};
#[cfg(feature = "no-internet")]
use crate::no_internet;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::config::Config;

#[derive(Debug, Deserialize)]
pub struct RecordingResponse {
    pub url: String,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamResponse {
    pub id: u64,
    pub ws_producer_url: String,
    pub url: String,
}

#[derive(Default, Serialize)]
pub struct StreamChangeset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_type: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_version: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Option<HashMap<String, String>>>,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    message: String,
}

pub fn get_auth_url(config: &mut Config) -> Result<Url> {
    let mut url = config.get_server_url()?;
    url.set_path(&format!("connect/{}", config.get_install_id()?));

    Ok(url)
}

#[cfg(not(feature = "no-internet"))]
pub async fn create_recording(path: &str, config: &mut Config) -> Result<RecordingResponse> {
    let server_url = &config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = create_recording_request(server_url, path, install_id)
        .await?
        .send()
        .await?;

    if response.status().as_u16() == 413 {
        match response.json::<ErrorResponse>().await {
            Ok(json) => {
                bail!("{}", json.message);
            }

            Err(_) => {
                bail!("The recording exceeds the server-configured size limit");
            }
        }
    } else {
        response.error_for_status_ref()?;
    }

    Ok(response.json::<RecordingResponse>().await?)
}

#[cfg(feature = "no-internet")]
pub async fn create_recording(_path: &str, _config: &mut Config) -> Result<RecordingResponse> {
    Err(no_internet::disabled().into())
}

#[cfg(not(feature = "no-internet"))]
async fn create_recording_request(
    server_url: &Url,
    path: &str,
    install_id: String,
) -> Result<RequestBuilder> {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path("api/v1/recordings");
    let form = Form::new().file("file", path).await?;
    let builder = client.post(url).multipart(form);

    Ok(add_headers(builder, &install_id))
}

#[cfg(not(feature = "no-internet"))]
pub async fn list_user_streams(prefix: &str, config: &mut Config) -> Result<Vec<StreamResponse>> {
    let server_url = config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = list_user_streams_request(&server_url, prefix, &install_id)
        .send()
        .await
        .context("cannot obtain stream producer endpoint - is the server down?")?;

    parse_stream_response(response, &server_url).await
}

#[cfg(feature = "no-internet")]
pub async fn list_user_streams(_prefix: &str, _config: &mut Config) -> Result<Vec<StreamResponse>> {
    Err(no_internet::disabled().into())
}

#[cfg(not(feature = "no-internet"))]
fn list_user_streams_request(server_url: &Url, prefix: &str, install_id: &str) -> RequestBuilder {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path("api/v1/user/streams");
    url.set_query(Some(&format!("prefix={prefix}&limit=10")));

    add_headers(client.get(url), install_id)
}

#[cfg(not(feature = "no-internet"))]
pub async fn create_stream(
    changeset: StreamChangeset,
    config: &mut Config,
) -> Result<StreamResponse> {
    let server_url = config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = create_stream_request(&server_url, &install_id, changeset)
        .send()
        .await
        .context("cannot obtain stream producer endpoint - is the server down?")?;

    parse_stream_response(response, &server_url).await
}

#[cfg(feature = "no-internet")]
pub async fn create_stream(
    _changeset: StreamChangeset,
    _config: &mut Config,
) -> Result<StreamResponse> {
    Err(no_internet::disabled().into())
}

#[cfg(not(feature = "no-internet"))]
fn create_stream_request(
    server_url: &Url,
    install_id: &str,
    changeset: StreamChangeset,
) -> RequestBuilder {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path("api/v1/streams");
    let builder = client.post(url);
    let builder = add_headers(builder, install_id);

    builder.json(&changeset)
}

#[cfg(not(feature = "no-internet"))]
pub async fn update_stream(
    stream_id: u64,
    changeset: StreamChangeset,
    config: &mut Config,
) -> Result<StreamResponse> {
    let server_url = config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = update_stream_request(&server_url, &install_id, stream_id, changeset)
        .send()
        .await
        .context("cannot obtain stream producer endpoint - is the server down?")?;

    parse_stream_response(response, &server_url).await
}

#[cfg(feature = "no-internet")]
pub async fn update_stream(
    _stream_id: u64,
    _changeset: StreamChangeset,
    _config: &mut Config,
) -> Result<StreamResponse> {
    Err(no_internet::disabled().into())
}

#[cfg(not(feature = "no-internet"))]
fn update_stream_request(
    server_url: &Url,
    install_id: &str,
    stream_id: u64,
    changeset: StreamChangeset,
) -> RequestBuilder {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path(&format!("api/v1/streams/{stream_id}"));
    let builder = client.patch(url);
    let builder = add_headers(builder, install_id);

    builder.json(&changeset)
}

#[cfg(not(feature = "no-internet"))]
async fn parse_stream_response<T: DeserializeOwned>(
    response: Response,
    server_url: &Url,
) -> Result<T> {
    let server_hostname = server_url.host().unwrap();

    match response.status().as_u16() {
        401 => bail!(
            "this CLI hasn't been authenticated with {server_hostname} - run `asciinema auth` first"
        ),

        404 => match response.json::<ErrorResponse>().await {
            Ok(json) => bail!("{}", json.message),
            Err(_) => bail!("{server_hostname} doesn't support streaming"),
        },

        422 => match response.json::<ErrorResponse>().await {
            Ok(json) => bail!("{}", json.message),
            Err(_) => bail!("{server_hostname} doesn't support streaming"),
        },

        _ => {
            response.error_for_status_ref()?;
        }
    }

    response.json::<T>().await.map_err(|e| e.into())
}

#[cfg(not(feature = "no-internet"))]
fn add_headers(builder: RequestBuilder, install_id: &str) -> RequestBuilder {
    builder
        .basic_auth(get_username(), Some(install_id))
        .header(header::USER_AGENT, build_user_agent())
        .header(header::ACCEPT, "application/json")
}

fn get_username() -> String {
    env::var("USER").unwrap_or("".to_owned())
}

pub fn build_user_agent() -> String {
    let ua = concat!(
        "asciinema/",
        env!("CARGO_PKG_VERSION"),
        " target/",
        env!("TARGET")
    );

    ua.to_owned()
}

#[cfg(feature = "no-internet")]
pub fn upload_cast<T>(_path: &str) -> anyhow::Result<T> {
    // stub: compilation succeeds but runtime returns an error if used.
    Err(no_internet::disabled().into())
}
