// Qwen3-ASR-Flash speech-to-text provider.
//
// Qwen's OpenAI-compatible ASR endpoint accepts a small base64 data URL. The
// audio pipeline already provides 16 kHz mono PCM, so buffering five seconds
// keeps the request well below Qwen's 10 MB input limit while preserving the
// existing cloud-STT behavior.

use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::audio::AudioChunk;
use crate::stt::provider::{STTProvider, STTProviderType, TranscriptResult};

const SEGMENT_DURATION_SECS: f32 = 5.0;
const SAMPLE_RATE: u32 = 16_000;
const DEFAULT_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

#[derive(Debug, Deserialize)]
struct QwenResponse {
    choices: Vec<QwenChoice>,
}

#[derive(Debug, Deserialize)]
struct QwenChoice {
    message: QwenMessage,
}

#[derive(Debug, Deserialize)]
struct QwenMessage {
    content: serde_json::Value,
}

pub struct QwenAsrSTT {
    api_key: String,
    language: String,
    is_streaming: bool,
    result_tx: Option<mpsc::Sender<TranscriptResult>>,
    stop_flag: Arc<AtomicBool>,
    start_time: Option<Instant>,
    audio_buffer: Vec<i16>,
    segment_sample_threshold: usize,
}

impl QwenAsrSTT {
    pub fn new() -> Self {
        Self {
            api_key: String::new(),
            language: "en-US".to_string(),
            is_streaming: false,
            result_tx: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
            start_time: None,
            audio_buffer: Vec::new(),
            segment_sample_threshold: (SAMPLE_RATE as f32 * SEGMENT_DURATION_SECS) as usize,
        }
    }

    pub fn with_api_key(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            ..Self::new()
        }
    }

    pub fn set_api_key(&mut self, api_key: &str) {
        self.api_key = api_key.to_string();
    }

    fn endpoint() -> String {
        let base = std::env::var("DASHSCOPE_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        format!("{}/chat/completions", base.trim_end_matches('/'))
    }

    fn encode_wav(samples: &[i16]) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let file_len = 36 + data_len;
        let mut buf = Vec::with_capacity(44 + data_len as usize);

        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_len.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        buf.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&16u16.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            buf.extend_from_slice(&sample.to_le_bytes());
        }
        buf
    }

    fn response_text(content: &serde_json::Value) -> String {
        if let Some(text) = content.as_str() {
            return text.trim().to_string();
        }

        content
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|part| {
                part.get("text")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| part.get("content").and_then(serde_json::Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string()
    }

    async fn send_segment(
        api_key: &str,
        language: &str,
        samples: Vec<i16>,
        timestamp_ms: u64,
        result_tx: mpsc::Sender<TranscriptResult>,
    ) {
        let wav_data = Self::encode_wav(&samples);
        let audio_data = format!(
            "data:audio/wav;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(wav_data)
        );

        let mut asr_options = serde_json::json!({ "enable_itn": true });
        if let Some(language_code) = language.split('-').next().filter(|code| *code != "auto" && !code.is_empty()) {
            asr_options["language"] = serde_json::json!(language_code);
        }

        let body = serde_json::json!({
            "model": "qwen3-asr-flash",
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "input_audio",
                    "input_audio": { "data": audio_data }
                }]
            }],
            "stream": false,
            "asr_options": asr_options
        });

        let response = reqwest::Client::new()
            .post(Self::endpoint())
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<QwenResponse>().await {
                    Ok(payload) => {
                        let text = payload
                            .choices
                            .first()
                            .map(|choice| Self::response_text(&choice.message.content))
                            .unwrap_or_default();
                        if !text.is_empty() {
                            let _ = result_tx
                                .send(TranscriptResult {
                                    text,
                                    is_final: true,
                                    confidence: 0.95,
                                    timestamp_ms,
                                    speaker: None,
                                    language: Some(language.to_string()),
                                    segment_id: None,
                                })
                                .await;
                        }
                    }
                    Err(error) => log::error!("QwenAsrSTT: failed to parse response: {}", error),
                }
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                log::error!("QwenAsrSTT: API returned status {}: {}", status, body);
            }
            Err(error) => log::error!("QwenAsrSTT: request failed: {}", error),
        }
    }
}

#[async_trait]
impl STTProvider for QwenAsrSTT {
    fn provider_name(&self) -> &str {
        "Qwen ASR"
    }

    fn provider_type(&self) -> STTProviderType {
        STTProviderType::QwenAsr
    }

    async fn start_stream(
        &mut self,
        result_tx: mpsc::Sender<TranscriptResult>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_streaming {
            return Err("Stream already active".into());
        }
        if self.api_key.trim().is_empty() {
            return Err("Qwen API key not configured".into());
        }

        self.result_tx = Some(result_tx);
        self.is_streaming = true;
        self.stop_flag.store(false, Ordering::SeqCst);
        self.start_time = Some(Instant::now());
        self.audio_buffer.clear();
        log::info!("QwenAsrSTT: started (language: {})", self.language);
        Ok(())
    }

    async fn feed_audio(
        &mut self,
        chunk: AudioChunk,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_streaming || self.stop_flag.load(Ordering::SeqCst) {
            return Ok(());
        }

        if !chunk.pcm_data.is_empty() {
            let rms = chunk
                .pcm_data
                .iter()
                .map(|&sample| (sample as f64) * (sample as f64))
                .sum::<f64>()
                / chunk.pcm_data.len() as f64;
            if rms < 0.25 {
                return Ok(());
            }
        }

        self.audio_buffer.extend_from_slice(&chunk.pcm_data);
        if self.audio_buffer.len() >= self.segment_sample_threshold {
            let segment = std::mem::take(&mut self.audio_buffer);
            let timestamp_ms = self
                .start_time
                .map(|started| started.elapsed().as_millis() as u64)
                .unwrap_or(chunk.timestamp_ms);

            if let Some(sender) = &self.result_tx {
                let _ = sender
                    .send(TranscriptResult {
                        text: "[识别中…]".to_string(),
                        is_final: false,
                        confidence: 0.0,
                        timestamp_ms,
                        speaker: None,
                        language: Some(self.language.clone()),
                        segment_id: None,
                    })
                    .await;

                let api_key = self.api_key.clone();
                let language = self.language.clone();
                let result_tx = sender.clone();
                tokio::spawn(async move {
                    Self::send_segment(&api_key, &language, segment, timestamp_ms, result_tx).await;
                });
            }
        }
        Ok(())
    }

    async fn stop_stream(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_streaming {
            return Ok(());
        }

        self.stop_flag.store(true, Ordering::SeqCst);
        if !self.audio_buffer.is_empty() {
            let segment = std::mem::take(&mut self.audio_buffer);
            let timestamp_ms = self
                .start_time
                .map(|started| started.elapsed().as_millis() as u64)
                .unwrap_or(0);
            if let Some(sender) = &self.result_tx {
                let api_key = self.api_key.clone();
                let language = self.language.clone();
                let result_tx = sender.clone();
                tokio::spawn(async move {
                    Self::send_segment(&api_key, &language, segment, timestamp_ms, result_tx).await;
                });
            }
        }

        self.is_streaming = false;
        self.result_tx = None;
        self.start_time = None;
        log::info!("QwenAsrSTT: stopped");
        Ok(())
    }

    async fn test_connection(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if self.api_key.trim().is_empty() {
            return Err("No Qwen API key configured".into());
        }

        let response = reqwest::Client::new()
            .get(format!("{}/models", Self::endpoint().trim_end_matches("/chat/completions")))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        Ok(response.status().is_success())
    }

    fn set_language(&mut self, language: &str) {
        self.language = language.to_string();
    }
}
