use serde_json::{json, Value};

pub const MODEL_COMPLETE: &str = "model.complete";
pub const NETWORK_FETCH_TEXT: &str = "network.fetch_text";
const MAX_RESPONSE_BYTES: u64 = 512 * 1024;
const MAX_PROMPT_LEN: usize = 32 * 1024;

pub struct NetworkGateway {
    allowlist: Vec<String>,
    model_endpoint: Option<String>,
    model_name: Option<String>,
    model_style: String,
    model_api_key: Option<String>,
    agent: ureq::Agent,
}

impl NetworkGateway {
    pub fn from_env() -> Self {
        let allowlist = std::env::var("NAVEEN_NETWORK_ALLOWLIST")
            .ok()
            .into_iter()
            .flat_map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|entry| !entry.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .collect();

        Self {
            allowlist,
            model_endpoint: std::env::var("NAVEEN_MODEL_ENDPOINT").ok(),
            model_name: std::env::var("NAVEEN_MODEL_NAME").ok(),
            model_style: std::env::var("NAVEEN_MODEL_API_STYLE")
                .unwrap_or_else(|_| "openai_compatible".to_string()),
            model_api_key: std::env::var("NAVEEN_MODEL_API_KEY").ok(),
            agent: ureq::Agent::config_builder()
                .timeout_connect(Some(std::time::Duration::from_secs(5)))
                .timeout_global(Some(std::time::Duration::from_secs(60)))
                .build()
                .into(),
        }
    }

    pub fn model_complete(&self, prompt: &str) -> Result<String, String> {
        if prompt.trim().is_empty() || prompt.len() > MAX_PROMPT_LEN {
            return Err("invalid model prompt".to_string());
        }

        let endpoint = self
            .model_endpoint
            .as_deref()
            .ok_or_else(|| "model provider is not configured".to_string())?;
        let model = self
            .model_name
            .as_deref()
            .ok_or_else(|| "model name is not configured".to_string())?;

        self.ensure_allowed(endpoint)?;

        let body = match self.model_style.as_str() {
            "ollama" => json!({
                "model": model,
                "messages": [{"role": "user", "content": prompt}],
                "stream": false,
            }),
            _ => json!({
                "model": model,
                "messages": [{"role": "user", "content": prompt}],
                "temperature": 0.2,
                "stream": false,
            }),
        };

        let mut request = self.agent.post(endpoint).content_type("application/json");
        if let Some(api_key) = &self.model_api_key {
            request = request.header("Authorization", format!("Bearer {api_key}"));
        }

        let mut response = request
            .send_json(&body)
            .map_err(|error| format!("model request failed: {error}"))?;
        let raw = response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BYTES)
            .read_to_string()
            .map_err(|error| format!("model response read failed: {error}"))?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|_| "model returned malformed JSON".to_string())?;

        let content = if self.model_style == "ollama" {
            value["message"]["content"].as_str()
        } else {
            value["choices"][0]["message"]["content"].as_str()
        }
        .ok_or_else(|| "model response did not contain text".to_string())?;

        if content.len() > MAX_RESPONSE_BYTES as usize {
            return Err("model response is too large".to_string());
        }
        Ok(content.to_string())
    }

    pub fn fetch_text(&self, url: &str) -> Result<String, String> {
        self.ensure_allowed(url)?;
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|error| format!("network request failed: {error}"))?;
        response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BYTES)
            .read_to_string()
            .map_err(|error| format!("network response read failed: {error}"))
    }

    fn ensure_allowed(&self, url: &str) -> Result<(), String> {
        let is_loopback = url.starts_with("http://127.0.0.1:")
            || url.starts_with("http://localhost:")
            || url.starts_with("http://[::1]:");
        if is_loopback || self.allowlist.iter().any(|prefix| url.starts_with(prefix)) {
            return Ok(());
        }
        Err("network target is not allowlisted".to_string())
    }
}

impl Default for NetworkGateway {
    fn default() -> Self {
        Self::from_env()
    }
}
