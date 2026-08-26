//! Read-only preflight checks run immediately before an interview starts.
//!
//! The checks never start capture, inject into a meeting, or send transcript
//! content. They only validate the selected devices and the configured local
//! services so the user can fix setup issues before going live.

use serde::Serialize;
use tauri::{command, State};

use crate::audio::device_manager;
use crate::rag::RagManager;
use crate::state::AppState;
use crate::stt::provider::STTProviderType;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareCheck {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
    pub required: bool,
}

fn check(id: &str, label: &str, status: &str, detail: impl Into<String>, required: bool) -> PrepareCheck {
    PrepareCheck {
        id: id.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        detail: detail.into(),
        required,
    }
}

fn credentials_ready(state: &AppState, provider: &STTProviderType) -> bool {
    let Some(credentials) = state.credentials.as_ref() else {
        return false;
    };
    let Ok(credentials) = credentials.lock() else {
        return false;
    };

    let key_name = match provider {
        STTProviderType::Deepgram => Some("deepgram"),
        STTProviderType::WhisperApi => Some("whisper_api"),
        STTProviderType::AzureSpeech => Some("azure_speech"),
        STTProviderType::GroqWhisper => Some("groq_whisper"),
        _ => None,
    };

    key_name
        .and_then(|name| credentials.get_key(name).ok().flatten())
        .map(|key| !key.trim().is_empty())
        .unwrap_or(false)
}

fn stt_provider_status(state: &AppState, providers: &[String]) -> PrepareCheck {
    if providers.is_empty() {
        return check(
            "stt",
            "Speech-to-text",
            "warning",
            "No party-specific STT provider selected yet",
            true,
        );
    }

    let mut labels = Vec::new();
    let mut errors = Vec::new();
    for value in providers {
        let Some(provider) = STTProviderType::from_str(value) else {
            errors.push(format!("unknown provider: {}", value));
            continue;
        };

        let label = provider.as_str().replace('_', " ");
        labels.push(label);
        let needs_credentials = matches!(
            provider,
            STTProviderType::Deepgram
                | STTProviderType::WhisperApi
                | STTProviderType::AzureSpeech
                | STTProviderType::GroqWhisper
        );
        if needs_credentials && !credentials_ready(state, &provider) {
            errors.push(format!("{} credentials missing", provider.as_str()));
        }
    }

    if !errors.is_empty() {
        return check(
            "stt",
            "Speech-to-text",
            "error",
            errors.join("; "),
            true,
        );
    }

    check(
        "stt",
        "Speech-to-text",
        "ready",
        format!("{} — provider configuration is ready", labels.join(" + ")),
        true,
    )
}

#[command]
pub async fn prepare_interview(
    mic_device_id: Option<String>,
    system_device_id: Option<String>,
    audio_mode: Option<String>,
    stt_providers: Option<Vec<String>>,
    professor_profile: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<PrepareCheck>, String> {
    let devices = device_manager::enumerate_devices()
        .map_err(|error| format!("Audio device check failed: {}", error))?;

    let mic_id = mic_device_id.unwrap_or_default();
    let system_id = system_device_id.unwrap_or_default();
    let mic_available = mic_id.is_empty()
        || mic_id == "default"
        || devices.inputs.iter().any(|device| device.id == mic_id);
    let system_available = system_id.is_empty()
        || system_id == "default"
        || devices.outputs.iter().any(|device| device.id == system_id);
    let online = audio_mode.as_deref() != Some("in_person");

    let audio_status = if devices.inputs.is_empty() || (online && devices.outputs.is_empty()) {
        "error"
    } else if !mic_available || (online && !system_available) {
        "error"
    } else {
        "ready"
    };
    let audio_detail = if online {
        format!(
            "{} microphone(s), {} output device(s); selected devices are available",
            devices.inputs.len(),
            devices.outputs.len()
        )
    } else {
        format!(
            "{} microphone(s); in-person mode does not require system loopback",
            devices.inputs.len()
        )
    };

    let mut checks = vec![
        check("audio", "Microphone + system audio", audio_status, audio_detail, true),
        stt_provider_status(&state, &stt_providers.unwrap_or_default()),
        check(
            "privacy",
            "Passive meeting mode",
            "ready",
            "Only selected audio is read; no meeting controls or automatic input are used",
            true,
        ),
    ];

    // Clone the provider out of the short std-mutex scope before awaiting its
    // connection test. Codex checks its local app-server handshake here.
    let llm_info = state.llm.as_ref().and_then(|llm| {
        let router = llm.lock().ok()?;
        let provider = router.get_provider().ok()?;
        let provider_name = router
            .active_provider_type()
            .map(|provider| provider.display_name().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let model = router.active_model().to_string();
        Some((provider, provider_name, model))
    });

    let llm_check = match llm_info {
        None => check("llm", "Answer model", "error", "No active LLM provider", true),
        Some((_provider, provider_name, model)) if model.trim().is_empty() => check(
            "llm",
            "Answer model",
            "error",
            format!("{} has no model selected", provider_name),
            true,
        ),
        Some((provider, provider_name, model)) => {
            let result = provider.lock().await.test_connection().await;
            match result {
                Ok(true) => check(
                    "llm",
                    "Answer model",
                    "ready",
                    format!("{} · {} is reachable", provider_name, model),
                    true,
                ),
                Ok(false) => check(
                    "llm",
                    "Answer model",
                    "error",
                    format!("{} · {} did not pass its connection check", provider_name, model),
                    true,
                ),
                Err(error) => check("llm", "Answer model", "error", error.to_string(), true),
            }
        }
    };
    checks.push(llm_check);

    let (resource_count, rag_status) = {
        let resource_count = state
            .context
            .as_ref()
            .and_then(|context| context.lock().ok())
            .map(|context| context.list_resources().len())
            .unwrap_or(0);
        let rag_status = state
            .database
            .as_ref()
            .and_then(|database| database.lock().ok())
            .and_then(|database| RagManager::get_status(database.connection()).ok());
        (resource_count, rag_status)
    };

    let context_check = match rag_status {
        Some(status) if resource_count > 0 && status.total_chunks > 0 => check(
            "context",
            "Personal knowledge base",
            "ready",
            format!("{} file(s), {} indexed chunk(s)", resource_count, status.total_chunks),
            false,
        ),
        Some(_) if resource_count > 0 => check(
            "context",
            "Personal knowledge base",
            "warning",
            format!("{} file(s) loaded but the search index is empty", resource_count),
            false,
        ),
        _ => check(
            "context",
            "Personal knowledge base",
            "warning",
            "No personal files loaded; direct answers still work",
            false,
        ),
    };
    checks.push(context_check);

    let translation_check = state
        .translation
        .as_ref()
        .and_then(|translation| translation.lock().ok())
        .and_then(|translation| translation.active_type().map(|kind| kind.to_string()))
        .map(|provider| {
            check(
                "translation",
                "Live translation",
                "ready",
                format!("{} translation provider selected", provider),
                false,
            )
        })
        .unwrap_or_else(|| {
            check(
                "translation",
                "Live translation",
                "warning",
                "Translation provider is optional and has not been initialized",
                false,
            )
        });
    checks.push(translation_check);

    checks.push(match professor_profile {
        Some(profile) if !profile.trim().is_empty() => check(
            "professor_profile",
            "Professor / lab profile",
            "ready",
            "Session profile is loaded for professor-related questions",
            false,
        ),
        _ => check(
            "professor_profile",
            "Professor / lab profile",
            "warning",
            "Optional — add a school, professor, lab, or research profile for more relevant follow-ups",
            false,
        ),
    });

    Ok(checks)
}
