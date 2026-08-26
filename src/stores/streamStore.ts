import { create } from "zustand";
import type {
  AIResponse,
  AnswerLength,
  IntelligenceMode,
  QueryRoute,
  QuestionType,
  StreamAnswerMetadata,
  StreamSource,
} from "../lib/types";
import { stripThinkTags } from "../lib/utils";

interface StreamState {
  // Current stream
  isStreaming: boolean;
  currentContent: string;
  _rawContent: string; // unfiltered content (includes <think> tags)
  currentMode: IntelligenceMode | null;
  currentModel: string;
  currentProvider: string;
  currentSources: StreamSource[];
  currentRoute: QueryRoute;
  currentQuestionType: QuestionType | null;
  currentAnswerSource: string | null;
  currentConfidence: string | null;
  currentAnswerLength: AnswerLength;
  error: string | null;
  latencyMs: number | null;

  // Response history (last 3) + pinned responses
  responseHistory: AIResponse[];
  pinnedResponses: AIResponse[];

  // Actions
  setStreaming: (streaming: boolean) => void;
  appendToken: (token: string) => void;
  startStream: (mode: IntelligenceMode, model: string, provider: string, metadata?: Partial<StreamAnswerMetadata>) => void;
  setSources: (sources: StreamSource[]) => void;
  endStream: (latencyMs: number) => void;
  setError: (error: string | null) => void;
  clearCurrent: () => void;
  pinResponse: (id: string) => void;
  unpinResponse: (id: string) => void;
}

export const useStreamStore = create<StreamState>((set, get) => ({
  isStreaming: false,
  currentContent: "",
  _rawContent: "",
  currentMode: null,
  currentModel: "",
  currentProvider: "",
  currentSources: [],
  currentRoute: "auto",
  currentQuestionType: null,
  currentAnswerSource: null,
  currentConfidence: null,
  currentAnswerLength: "normal",
  error: null,
  latencyMs: null,
  responseHistory: [],
  pinnedResponses: [],

  setStreaming: (streaming) => set({ isStreaming: streaming }),

  appendToken: (token) =>
    set((state) => {
      const raw = state._rawContent + token;
      return {
        _rawContent: raw,
        currentContent: stripThinkTags(raw),
      };
    }),

  startStream: (mode, model, provider, metadata) =>
    set({
      isStreaming: true,
      currentContent: "",
      _rawContent: "",
      currentMode: mode,
      currentModel: model,
      currentProvider: provider,
      currentSources: [],
      currentRoute: metadata?.route ?? "auto",
      currentQuestionType: metadata?.question_type ?? null,
      currentAnswerSource: metadata?.answer_source ?? null,
      currentConfidence: metadata?.confidence ?? null,
      currentAnswerLength: metadata?.answer_length ?? "normal",
      error: null,
      latencyMs: null,
    }),

  setSources: (sources) => set({ currentSources: sources }),

  endStream: (latencyMs) => {
    const state = get();
    // Strip <think> tags from the final stored content
    const content = stripThinkTags(state._rawContent);
    const response: AIResponse = {
      id: crypto.randomUUID(),
      content,
      mode: state.currentMode!,
      timestamp: Date.now(),
      pinned: false,
      model: state.currentModel,
      provider: state.currentProvider,
      latency_ms: latencyMs,
      sources: state.currentSources.length > 0 ? state.currentSources : undefined,
      route: state.currentRoute,
      question_type: state.currentQuestionType ?? undefined,
      answer_source: state.currentAnswerSource ?? undefined,
      confidence: state.currentConfidence ?? undefined,
      answer_length: state.currentAnswerLength,
    };

    set((s) => ({
      isStreaming: false,
      currentContent: content,
      latencyMs,
      responseHistory: [response, ...s.responseHistory].slice(0, 5),
    }));
  },

  setError: (error) => set({ error, isStreaming: false }),
  clearCurrent: () =>
    set({ currentContent: "", _rawContent: "", currentMode: null, currentModel: "", currentProvider: "", currentSources: [], currentRoute: "auto", currentQuestionType: null, currentAnswerSource: null, currentConfidence: null, currentAnswerLength: "normal", error: null, latencyMs: null }),

  pinResponse: (id) => {
    const state = get();
    const response = state.responseHistory.find((r) => r.id === id);
    if (response && !state.pinnedResponses.find((r) => r.id === id)) {
      set({
        pinnedResponses: [
          ...state.pinnedResponses,
          { ...response, pinned: true },
        ],
      });
    }
  },

  unpinResponse: (id) =>
    set((state) => ({
      pinnedResponses: state.pinnedResponses.filter((r) => r.id !== id),
    })),
}));
