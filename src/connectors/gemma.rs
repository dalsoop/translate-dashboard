//! Gemma connector — 기존 TranslateClient 래핑 (자체 TranslateGemma 서버).

use super::{Connector, TranslateResult};
use crate::backend::translate::TranslateClient;
use anyhow::Result;
use async_trait::async_trait;

pub struct GemmaConnector {
    client: TranslateClient,
}

impl GemmaConnector {
    pub fn new(endpoints: Vec<String>) -> Self {
        Self { client: TranslateClient::new(endpoints) }
    }
}

#[async_trait]
impl Connector for GemmaConnector {
    fn name(&self) -> &'static str { "gemma" }

    async fn translate(
        &self,
        text: &str,
        source: &str,
        target: &str,
        context: Option<&str>,
    ) -> Result<TranslateResult> {
        let r = self.client.translate(text, source, target, context).await?;
        Ok(TranslateResult {
            translation: r.translation,
            elapsed_s: r.elapsed_s,
            backend: "gemma".into(),
        })
    }

    async fn health(&self) -> Result<String> {
        // 첫 엔드포인트 health 만 확인
        if let Some(ep) = self.client.endpoints().first() {
            let h = self.client.health(ep).await?;
            Ok(format!("{} vram={:.2}GB", if h.ok { "ok" } else { "down" }, h.vram_gb))
        } else {
            Ok("no endpoints".into())
        }
    }
}
