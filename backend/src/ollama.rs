use anyhow::{Result, ensure};
use reqwest::{Client, Method, redirect::Policy};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::time::Duration;

const HOST: &str = "http://127.0.0.1:11434";

#[derive(Deserialize)]
struct Tags {
    models: Vec<LocalModel>,
}

#[derive(Deserialize)]
struct LocalModel {
    name: String,
    details: ModelDetails,
    size: u64,
    remote_host: Option<String>,
    remote_model: Option<String>,
}

#[derive(Deserialize)]
struct ModelDetails {
    format: String,
}

#[derive(Deserialize)]
struct ModelCard {
    capabilities: Option<Vec<String>>,
    remote_host: Option<String>,
    remote_model: Option<String>,
    details: ModelDetails,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: Message,
    done: bool,
    done_reason: Option<String>,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

pub struct Ollama(Client);

impl Ollama {
    pub fn connect() -> Result<Self> {
        Ok(Self(
            Client::builder()
                .no_proxy()
                .redirect(Policy::none())
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(240))
                .build()?,
        ))
    }

    async fn receive<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        max: usize,
    ) -> Result<T> {
        let mut request = self.0.request(method, format!("{HOST}{path}"));
        if let Some(body) = body {
            request = request.json(&body);
        }
        if path != "/api/chat" {
            request = request.timeout(Duration::from_secs(5));
        }
        let mut response = request.send().await?.error_for_status()?;
        let mut buffer = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                buffer.len() + chunk.len() <= max,
                "Ollama response too large"
            );
            buffer.extend_from_slice(&chunk);
        }
        Ok(serde_json::from_slice(&buffer)?)
    }

    async fn tags(&self) -> Result<Vec<LocalModel>> {
        let tags: Tags = self
            .receive(Method::GET, "/api/tags", None, 512 * 1024)
            .await?;
        Ok(tags
            .models
            .into_iter()
            .filter(|model| {
                model.details.format == "gguf"
                    && model.size > 0
                    && valid_name(&model.name)
                    && local_field(model.remote_host.as_deref())
                    && local_field(model.remote_model.as_deref())
            })
            .collect())
    }

    pub async fn models(&self) -> Result<Vec<String>> {
        Ok(self
            .tags()
            .await?
            .into_iter()
            .map(|model| model.name)
            .collect())
    }

    pub async fn chat<T: DeserializeOwned>(
        &self,
        model: &str,
        prompt: String,
        payload: Value,
        schema: Value,
        available_mib: u64,
    ) -> Result<T> {
        let tag = self
            .tags()
            .await?
            .into_iter()
            .find(|candidate| candidate.name == model);
        let tag = tag.ok_or_else(|| anyhow::anyhow!("The Ollama model is unavailable locally."))?;
        ensure!(
            tag.size / 1_048_576 + 512 < available_mib,
            "Not enough available RAM for this Ollama model."
        );
        let card: ModelCard = self
            .receive(
                Method::POST,
                "/api/show",
                Some(json!({"model":model})),
                2 * 1024 * 1024,
            )
            .await?;
        ensure!(
            local_field(card.remote_host.as_deref())
                && local_field(card.remote_model.as_deref())
                && card.details.format == "gguf"
                && card
                    .capabilities
                    .is_none_or(|caps| caps.iter().any(|cap| cap == "completion")),
            "Select a locally installed completion model."
        );
        let result: ChatResponse = self
            .receive(
                Method::POST,
                "/api/chat",
                Some(json!({
                    "model": model,
                    "stream": false,
                    "keep_alive": 0,
                    "options": { "temperature": 0, "num_ctx": 8192, "num_predict": 1600 },
                    "format": schema,
                    "messages": [
                        {"role":"system","content": prompt},
                        {"role":"user","content": payload.to_string()}
                    ]
                })),
                256 * 1024,
            )
            .await?;
        ensure!(
            result.done && result.done_reason.as_deref() != Some("length"),
            "The local model did not finish its response."
        );
        Ok(serde_json::from_str(&result.message.content)?)
    }
}

fn local_field(value: Option<&str>) -> bool {
    value.is_none_or(str::is_empty)
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 140
        && !name.to_ascii_lowercase().contains("cloud")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/-_:.".contains(&byte))
}
