import { useCallback, useEffect, useMemo, useState } from "react";
import { useStreamStore } from "../stores/streamStore";
import { useAIActionsStore } from "../stores/aiActionsStore";
import { generateAssist, cancelGeneration } from "../lib/ipc";
import { showToast } from "../stores/toastStore";
import type { AnswerLength, IntelligenceMode, QueryRoute } from "../lib/types";
import {
  Loader2,
  Sparkles,
  MessageSquare,
  Scissors,
  CornerUpRight,
  ListChecks,
  HelpCircle,
  Wand2,
  Send,
  X,
} from "lucide-react";

// Icon mapping for built-in modes
const MODE_ICONS: Record<string, typeof Sparkles> = {
  Assist: Sparkles,
  WhatToSay: MessageSquare,
  Shorten: Scissors,
  FollowUp: CornerUpRight,
  Recap: ListChecks,
  AskQuestion: HelpCircle,
};

// Built-in shortcuts
const MODE_SHORTCUTS: Record<string, string> = {
  Assist: "Space",
  WhatToSay: "1",
  Shorten: "2",
  FollowUp: "3",
  Recap: "4",
  AskQuestion: "5",
};

// Ordered built-in modes — AskQuestion now included
const BUILT_IN_ORDER = ["Assist", "WhatToSay", "Shorten", "FollowUp", "Recap", "AskQuestion"];

export function ModeButtons() {
  const currentMode = useStreamStore((s) => s.currentMode);
  const isStreaming = useStreamStore((s) => s.isStreaming);
  const actions = useAIActionsStore((s) => s.configs.actions);
  const [askInputText, setAskInputText] = useState("");
  const [askInputVisible, setAskInputVisible] = useState(false);
  const [queryRoute, setQueryRoute] = useState<QueryRoute>("auto");
  const [answerLength, setAnswerLength] = useState<AnswerLength>("normal");

  // Listen for keyboard shortcut (Digit5) to toggle ask input
  useEffect(() => {
    const handler = () => setAskInputVisible((v) => !v);
    window.addEventListener("nexq:toggle-ask-input", handler);
    return () => window.removeEventListener("nexq:toggle-ask-input", handler);
  }, []);

  // Build ordered list: built-in modes first (in order), then custom actions
  const visibleModes = useMemo(() => {
    const result: { mode: string; label: string; shortcut: string; icon: typeof Sparkles; isCustom: boolean }[] = [];

    // Add built-in modes in order
    for (const modeKey of BUILT_IN_ORDER) {
      const cfg = actions[modeKey];
      if (cfg && cfg.visible) {
        result.push({
          mode: cfg.mode,
          label: cfg.name,
          shortcut: MODE_SHORTCUTS[modeKey] || "",
          icon: MODE_ICONS[modeKey] || Wand2,
          isCustom: false,
        });
      }
    }

    // Add custom actions
    for (const [key, cfg] of Object.entries(actions)) {
      if (!cfg.isBuiltIn && cfg.visible) {
        result.push({
          mode: cfg.mode,
          label: cfg.name,
          shortcut: "",
          icon: Wand2,
          isCustom: true,
        });
      }
    }

    return result;
  }, [actions]);

  const handleClick = useCallback(
    (mode: string) => {
      if (isStreaming) {
        if (currentMode === mode) cancelGeneration().catch(() => showToast("Couldn't cancel generation", "error"));
        return;
      }
      // AskQuestion mode: toggle ask input instead of direct generation
      if (mode === "AskQuestion") {
        setAskInputVisible((v) => !v);
        return;
      }
      generateAssist(mode, undefined, queryRoute, answerLength).catch((err) => showToast(err instanceof Error ? err.message : "Couldn't generate AI response", "error"));
    },
    [isStreaming, currentMode, queryRoute, answerLength]
  );

  const handleAskSubmit = useCallback(() => {
    const text = askInputText.trim();
    if (!text || isStreaming) return;
    generateAssist("AskQuestion", text, queryRoute, answerLength).catch((err) =>
      showToast(err instanceof Error ? err.message : "Couldn't send question", "error")
    );
    setAskInputText("");
    setAskInputVisible(false);
  }, [askInputText, isStreaming, queryRoute, answerLength]);

  return (
    <div className="flex flex-col gap-1">
      <div className="flex flex-wrap items-center gap-0.5">
        {visibleModes.map(({ mode, label, shortcut, icon: Icon, isCustom }) => {
          const isActive = currentMode === mode && isStreaming;
          const isAskActive = mode === "AskQuestion" && askInputVisible && !isStreaming;
          return (
            <button
              key={mode}
              onClick={() => handleClick(mode)}
              disabled={isStreaming && !isActive}
              aria-label={shortcut ? `${label} (${shortcut})` : label}
              aria-pressed={isActive || isAskActive}
              className={`flex items-center gap-1 rounded-lg px-2 py-1 text-meta font-medium transition-all duration-150 cursor-pointer ${
                isActive
                  ? "bg-primary/20 text-primary ring-1 ring-primary/30 shadow-sm shadow-primary/10"
                  : isAskActive
                    ? "bg-info/10 text-info ring-1 ring-info/20 shadow-sm shadow-info/10"
                    : isCustom
                      ? "text-warning/60 hover:bg-warning/10 hover:text-warning border border-warning/10"
                      : "text-muted-foreground/60 hover:bg-accent/50 hover:text-foreground"
              } ${isStreaming && !isActive ? "opacity-40 cursor-not-allowed" : ""}`}
            >
              {isActive ? <Loader2 className="h-3 w-3 animate-spin" aria-hidden="true" /> : <Icon className="h-3 w-3" aria-hidden="true" />}
              <span>{label}</span>
            </button>
          );
        })}
        <label className="ml-auto flex items-center gap-1 rounded-lg border border-border/20 px-1.5 py-1 text-[0.6rem] text-muted-foreground/60" title="Choose where the assistant should look for an answer">
          <span>Source</span>
          <select
            value={queryRoute}
            onChange={(e) => setQueryRoute(e.target.value as QueryRoute)}
            disabled={isStreaming}
            aria-label="Answer source"
            className="max-w-[6.5rem] cursor-pointer bg-transparent text-[0.6rem] text-foreground/70 outline-none disabled:cursor-not-allowed disabled:opacity-50"
          >
            <option value="auto">Auto</option>
            <option value="quick_answer">Quick</option>
            <option value="search_files">My files</option>
            <option value="search_web">Web</option>
            <option value="ask_codex">Codex</option>
          </select>
        </label>
        <label className="flex items-center gap-1 rounded-lg border border-border/20 px-1.5 py-1 text-[0.6rem] text-muted-foreground/60" title="Choose answer length">
          <span>Length</span>
          <select
            value={answerLength}
            onChange={(e) => setAnswerLength(e.target.value as AnswerLength)}
            disabled={isStreaming}
            aria-label="Answer length"
            className="max-w-[4.8rem] cursor-pointer bg-transparent text-[0.6rem] text-foreground/70 outline-none disabled:cursor-not-allowed disabled:opacity-50"
          >
            <option value="short">Short</option>
            <option value="normal">Normal</option>
            <option value="detailed">Detailed</option>
          </select>
        </label>
      </div>

      {/* Inline Ask input */}
      {askInputVisible && !isStreaming && (
        <div className="flex items-center gap-1.5 rounded-lg border border-info/20 bg-info/5 px-2 py-1 slide-down-enter">
          <input
            type="text"
            value={askInputText}
            onChange={(e) => setAskInputText(e.target.value)}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === "Enter") { e.preventDefault(); handleAskSubmit(); }
              if (e.key === "Escape") { e.preventDefault(); setAskInputVisible(false); }
            }}
            placeholder="Ask about the meeting..."
            aria-label="Ask a question"
            autoFocus
            maxLength={2000}
            className="flex-1 min-w-0 bg-transparent text-xs text-foreground/90 placeholder:text-muted-foreground/50 outline-none"
          />
          <button
            onClick={handleAskSubmit}
            disabled={!askInputText.trim()}
            className="rounded-md p-1 text-info/60 hover:bg-info/10 hover:text-info disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            aria-label="Send question"
          >
            <Send className="h-3 w-3" aria-hidden="true" />
          </button>
          <button
            onClick={() => setAskInputVisible(false)}
            className="rounded-md p-1 text-muted-foreground/60 hover:bg-accent hover:text-muted-foreground transition-colors"
            aria-label="Close question input"
          >
            <X className="h-3 w-3" aria-hidden="true" />
          </button>
        </div>
      )}
    </div>
  );
}
