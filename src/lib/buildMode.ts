import type { LLMProviderType, STTProviderType, TranslationProviderType } from "./types";

/** NexQ ships as one product: the Simplified Chinese, pure API build. */
export const REMOTE_ONLY = true;

export const REMOTE_STT_PROVIDERS: readonly STTProviderType[] = [
  "qwen_asr",
];

export const REMOTE_LLM_PROVIDERS: readonly LLMProviderType[] = [
  "qwen",
];

export const REMOTE_TRANSLATION_PROVIDERS: readonly TranslationProviderType[] = [
  "llm",
];

export const DEFAULT_REMOTE_STT_PROVIDER: STTProviderType = "qwen_asr";

export function isRemoteSttProvider(provider: string): provider is STTProviderType {
  return REMOTE_STT_PROVIDERS.includes(provider as STTProviderType);
}

export function isRemoteLlmProvider(provider: string): provider is LLMProviderType {
  return REMOTE_LLM_PROVIDERS.includes(provider as LLMProviderType);
}

export function isRemoteTranslationProvider(provider: string): provider is TranslationProviderType {
  return REMOTE_TRANSLATION_PROVIDERS.includes(provider as TranslationProviderType);
}
