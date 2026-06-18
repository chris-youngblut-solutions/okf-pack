//! KC-10 — the embedder seam (D3: explicit-required, no default).
//!
//! Indexing/querying needs an embedder, but there is **no default**: a Space must
//! declare `[seat.knowledge].embedder` (`frontier` | `local`) or
//! [`KnowledgeConfig::resolve`] errors. `local` declares `net_mode = "none"` (the
//! corpus never leaves the box); `frontier` is external and must additionally
//! pass the privilege gate (KC-8) before any content is sent.
//!
//! Both modes speak the same minimal HTTP contract: `POST endpoint` with
//! `{"model": ..., "input": [texts]}` → `{"embeddings": [[f32]]}`. Tests use a
//! localhost stub; no real network is touched.

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;

/// Two-method embedder seam. Implementations are swappable by config.
pub trait Embedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn count_tokens(&self, text: &str) -> usize;
}

/// `[seat.knowledge]` config, resolved from the Space manifest.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct KnowledgeConfig {
    /// `frontier` | `local`. **No default** (D3).
    pub embedder: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub net_mode: Option<String>,
}

impl KnowledgeConfig {
    /// The declared network mode: `none` for a local embedder, else `frontier`.
    /// Mirrors the pack's `capabilities.net`.
    #[must_use]
    pub fn net_mode(&self) -> &str {
        match self.embedder.as_deref() {
            Some("local") => "none",
            _ => self.net_mode.as_deref().unwrap_or("frontier"),
        }
    }

    /// Resolve a boxed embedder. **D3: errors if `embedder` is unset.**
    pub fn resolve(&self) -> Result<Box<dyn Embedder>> {
        let endpoint = || {
            self.endpoint
                .clone()
                .ok_or_else(|| anyhow!("[seat.knowledge].endpoint is required for this embedder"))
        };
        match self.embedder.as_deref() {
            None => bail!(
                "no embedder configured — set [seat.knowledge].embedder (frontier|local) before indexing or querying"
            ),
            Some("frontier") => Ok(Box::new(HttpEmbedder {
                endpoint: endpoint()?,
                model: self.model.clone().unwrap_or_default(),
                api_key: std::env::var("OKFPACK_EMBED_API_KEY").ok(),
            })),
            Some("local") => Ok(Box::new(HttpEmbedder {
                endpoint: endpoint()?,
                model: self.model.clone().unwrap_or_default(),
                api_key: None,
            })),
            Some(other) => bail!("unknown embedder `{other}` (expected frontier|local)"),
        }
    }
}

/// An embedder that POSTs to an HTTP endpoint (frontier or local).
pub struct HttpEmbedder {
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

impl Embedder for HttpEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let client = reqwest::blocking::Client::new();
        let mut request = client
            .post(&self.endpoint)
            .json(&serde_json::json!({ "model": self.model, "input": texts }));
        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }
        let response: EmbedResponse = request.send()?.error_for_status()?.json()?;
        Ok(response.embeddings)
    }

    fn count_tokens(&self, text: &str) -> usize {
        approx_tokens(text)
    }
}

/// A whitespace-word token approximation (used until a real tokenizer is wired).
#[must_use]
pub fn approx_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}
