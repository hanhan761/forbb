import type { LLMProviderType, STTProviderType, TranslationProviderType } from "./types";

/** True for the small installer that delegates AI inference to cloud APIs. */
export const REMOTE_ONLY = import.meta.env.VITE_NEXQ_REMOTE_ONLY === "true";

export const REMOTE_STT_PROVIDERS: readonly STTProviderType[] = [
  "deepgram",
  "whisper_api",
  "azure_speech",
  "groq_whisper",
];

export const REMOTE_LLM_PROVIDERS: readonly LLMProviderType[] = [
  "openai",
  "qwen",
  "anthropic",
  "groq",
  "gemini",
  "openrouter",
  "custom",
];

export const REMOTE_TRANSLATION_PROVIDERS: readonly TranslationProviderType[] = [
  "microsoft",
  "google",
  "deepl",
  "llm",
];

export const DEFAULT_REMOTE_STT_PROVIDER: STTProviderType = "deepgram";

export function isRemoteSttProvider(provider: string): provider is STTProviderType {
  return REMOTE_STT_PROVIDERS.includes(provider as STTProviderType);
}

export function isRemoteLlmProvider(provider: string): provider is LLMProviderType {
  return REMOTE_LLM_PROVIDERS.includes(provider as LLMProviderType);
}

export function isRemoteTranslationProvider(provider: string): provider is TranslationProviderType {
  return REMOTE_TRANSLATION_PROVIDERS.includes(provider as TranslationProviderType);
}
