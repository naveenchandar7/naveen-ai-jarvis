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
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter_map(|entry| normalize_origin(&entry))
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

        self.ensure_model_allowed(endpoint)?;

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
        self.ensure_fetch_allowed(url)?;
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

    fn ensure_model_allowed(&self, url: &str) -> Result<(), String> {
        let origin = normalize_origin(url).ok_or_else(|| "invalid model endpoint".to_string())?;
        if is_loopback_origin(&origin)
            || self.allowlist.iter().any(|allowed| origin == *allowed)
        {
            return Ok(());
        }
        Err("model endpoint is not allowlisted".to_string())
    }

    fn ensure_fetch_allowed(&self, url: &str) -> Result<(), String> {
        let origin = normalize_origin(url).ok_or_else(|| "invalid network URL".to_string())?;
        if self.allowlist.iter().any(|allowed| origin == *allowed) {
            return Ok(());
        }
        Err("network target is not allowlisted".to_string())
    }
}

fn normalize_origin(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.bytes().any(|byte| byte.is_ascii_whitespace() || byte == b'\\') {
        return None;
    }

    let (scheme, rest) = trimmed.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }

    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return None;
    }

    Some(format!("{scheme}://{}", authority.to_ascii_lowercase()))
}

fn is_loopback_origin(origin: &str) -> bool {
    let Some((scheme, authority)) = origin.split_once("://") else {
        return false;
    };

    if scheme != "http" && scheme != "https" {
        return false;
    }

    let host = if let Some(stripped) = authority.strip_prefix('[') {
        let Some(end) = stripped.find(']') else {
            return false;
        };
        if !stripped[end + 1..].is_empty() {
            let port = &stripped[end + 2..];
            if !port.is_empty() && !port.bytes().all(|byte| byte.is_ascii_digit()) {
                return false;
            }
        }
        &stripped[..end]
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        host
    } else {
        authority
    };

    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

impl Default for NetworkGateway {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_origin_rejects_userinfo_and_wrong_scheme() {
        assert!(normalize_origin("https://user@example.com").is_none());
        assert!(normalize_origin("ftp://example.com").is_none());
    }

    #[test]
    fn normalize_origin_lowercases_scheme_and_authority() {
        assert_eq!(
            normalize_origin("HTTPS://Example.COM/path"),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn loopback_matching_requires_exact_host() {
        assert!(is_loopback_origin("http://127.0.0.1:11434"));
        assert!(!is_loopback_origin("http://127.0.0.1.evil.example"));
        assert!(is_loopback_origin("http://localhost:3000"));
        assert!(!is_loopback_origin("http://localhost.evil.example"));
        assert!(is_loopback_origin("http://[::1]:8080"));
        assert!(!is_loopback_origin("http://[::1].evil.example"));
    }

    #[test]
    fn loopback_matching_rejects_malformed_ports() {
        assert!(!is_loopback_origin("http://localhost:"));
        assert!(!is_loopback_origin("http://localhost:abc"));
    }
}
