import { useEffect, useState, useRef, useCallback } from "react";
import { HelpCircle, Sparkles, Check, Clock, X } from "lucide-react";
import { onQuestionDetected } from "../lib/events";
import { generateAssist } from "../lib/ipc";
import { useTranscriptStore } from "../stores/transcriptStore";
import { useAIActionsStore } from "../stores/aiActionsStore";
import { useStreamStore } from "../stores/streamStore";
import { showToast } from "../stores/toastStore";
import type { DetectedQuestion } from "../lib/types";

function looksLikeQuestion(text: string): boolean {
  const trimmed = text.trim();
  if (trimmed.endsWith("?") || trimmed.endsWith("？")) return true;
  const lower = trimmed.toLowerCase();
  const qWords = [
    "what ", "how ", "why ", "when ", "where ", "who ", "which ",
    "can you", "could you", "would you", "do you", "are you",
    "is there", "have you", "tell me", "explain",
  ];
  if (qWords.some((w) => lower.startsWith(w))) return true;

  // Qwen ASR returns Chinese questions with or without punctuation.
  const chineseSignals = [
    "什么", "为什么", "怎么", "如何", "怎样", "哪里", "哪儿", "哪个", "哪些", "谁",
    "多少", "是否", "能否", "可否", "请问", "请介绍", "介绍一下", "请解释", "解释一下",
    "谈谈", "说说", "你怎么看", "你认为", "你会如何",
  ];
  return chineseSignals.some((signal) => lower.includes(signal)) || /吗$|呢$/.test(trimmed);
}

function questionKey(q: DetectedQuestion): string {
  return `${q.source}:${q.text.replace(/[?？。！!\s]+$/g, "").replace(/\s+/g, " ").trim().toLowerCase()}`;
}

interface TrackedQuestion extends DetectedQuestion {
  assisted: boolean;
}

export function QuestionDetector({ compact = false }: { compact?: boolean }) {
  const [questions, setQuestions] = useState<TrackedQuestion[]>([]);
  const processedIdsRef = useRef<Set<string>>(new Set());
  const autoAssistedKeysRef = useRef<Set<string>>(new Set());
  const autoAssistInFlightRef = useRef(false);
  const [pendingAutoQuestions, setPendingAutoQuestions] = useState<DetectedQuestion[]>([]);
  const [autoAnswerCompleted, setAutoAnswerCompleted] = useState(0);
  const segments = useTranscriptStore((s) => s.segments);
  const autoTrigger = useAIActionsStore((s) => s.configs.globalDefaults.autoTrigger);
  const isStreaming = useStreamStore((s) => s.isStreaming);
  const autoTriggerRef = useRef(autoTrigger);
  const isStreamingRef = useRef(isStreaming);

  useEffect(() => {
    autoTriggerRef.current = autoTrigger;
    isStreamingRef.current = isStreaming;
  }, [autoTrigger, isStreaming]);

  const answerQuestion = useCallback(async (question: DetectedQuestion, automatic: boolean) => {
    if (automatic) {
      if (!autoTriggerRef.current) return;
      if (autoAssistInFlightRef.current || isStreamingRef.current) {
        setPendingAutoQuestions((pending) => [...pending, question].slice(-3));
        return;
      }
      autoAssistInFlightRef.current = true;
    }

    try {
      // Automatic answers always use the local knowledge base first and stay
      // short enough to read aloud. Qwen then supplies the final wording.
      await generateAssist("AskQuestion", question.text, "search_files", "short");
    } catch (err) {
      setQuestions((prev) =>
        prev.map((item) =>
          questionKey(item) === questionKey(question) ? { ...item, assisted: false } : item,
        ),
      );
      if (automatic) {
        showToast(
          err instanceof Error ? err.message : "自动回答失败，请检查 Qwen API 或知识库配置",
          "error",
        );
      }
    } finally {
      if (automatic) {
        autoAssistInFlightRef.current = false;
        setAutoAnswerCompleted((count) => count + 1);
      }
    }
  }, []);

  useEffect(() => {
    if (pendingAutoQuestions.length === 0 || isStreaming || autoAssistInFlightRef.current) return;
    const [next, ...rest] = pendingAutoQuestions;
    setPendingAutoQuestions(rest);
    void answerQuestion(next, true);
  }, [answerQuestion, autoAnswerCompleted, isStreaming, pendingAutoQuestions]);

  const addQuestion = useCallback((q: DetectedQuestion) => {
    const key = questionKey(q);
    const shouldAutoAssist =
      autoTriggerRef.current &&
      !autoAssistedKeysRef.current.has(key);

    if (shouldAutoAssist) {
      autoAssistedKeysRef.current.add(key);
      void answerQuestion(q, true);
    }

    setQuestions((prev) => {
      if (prev.length > 0 && prev[0].text === q.text) return prev;
      return [{ ...q, assisted: shouldAutoAssist }, ...prev].slice(0, 10);
    });

  }, [answerQuestion]);

  useEffect(() => {
    const p = onQuestionDetected((event) => {
      if (event.source === "Them" || event.source === "Interviewer") {
        addQuestion(event);
      }
    });
    return () => { p.then((u) => u()); };
  }, [addQuestion]);

  useEffect(() => {
    for (const seg of segments) {
      if (seg.is_final && !processedIdsRef.current.has(seg.id) && (seg.speaker === "Them" || seg.speaker === "Interviewer") && looksLikeQuestion(seg.text)) {
        processedIdsRef.current.add(seg.id);
        addQuestion({ text: seg.text, confidence: 0.8, timestamp_ms: seg.timestamp_ms, source: seg.speaker });
      }
      if (seg.is_final) processedIdsRef.current.add(seg.id);
    }
  }, [segments, addQuestion]);

  const handleAssist = useCallback((index: number) => {
    const question = questions[index];
    if (!question) return;
    setQuestions((prev) =>
      prev.map((q, i) => i === index ? { ...q, assisted: true } : q)
    );
    void answerQuestion(question, false);
  }, [answerQuestion, questions]);

  const handleDismiss = useCallback((index: number, e: React.MouseEvent) => {
    e.stopPropagation();
    setQuestions((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const latest = questions.length > 0 ? questions[0] : null;
  const previousQuestions = questions.slice(1, 4);

  return (
    <div className="flex flex-col gap-2.5" role="region" aria-label="自动识别的问题">
      {/* Latest question — prominent card */}
      <div
        className={`group flex items-start gap-3 rounded-lg transition-all duration-200 ${
          latest ? "cursor-pointer hover:bg-info/10 question-card-enter" : ""
        }`}
        onClick={() => latest && handleAssist(0)}
        onKeyDown={(e) => { if (latest && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); handleAssist(0); } }}
        role={latest ? "button" : undefined}
        tabIndex={latest ? 0 : undefined}
        aria-label={latest ? `问题：${latest.text}。${latest.assisted ? "已回答" : "点击获取回答"}` : undefined}
      >
        <div className="relative mt-0.5 shrink-0" aria-hidden="true">
          <HelpCircle className={`h-5 w-5 transition-colors ${latest ? "text-info" : "text-muted-foreground/50"}`} />
          {latest && !latest.assisted && (
            <span className="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-info animate-pulse" />
          )}
          {latest?.assisted && (
            <span className="absolute -top-0.5 -right-0.5 flex h-3 w-3 items-center justify-center rounded-full bg-success">
              <Check className="h-2 w-2 text-white" />
            </span>
          )}
        </div>

        <div className="flex-1 min-w-0">
          {latest ? (
            <p className="text-sm leading-relaxed font-medium text-foreground/90">
              &ldquo;{latest.text}&rdquo;
            </p>
          ) : (
            <p className="text-xs text-muted-foreground/50">
              正在监听对方的问题
            </p>
          )}
        </div>

        {latest && (
          <div className="flex shrink-0 items-center gap-1">
            <button
              onClick={(e) => { e.stopPropagation(); handleAssist(0); }}
              aria-label={latest.assisted ? "已回答" : "获取这个问题的回答"}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold transition-all duration-150 cursor-pointer ${
                latest.assisted
                  ? "bg-success/10 border border-success/20 text-success"
                  : "bg-info/10 border border-info/20 text-info hover:bg-info/20"
              }`}
            >
              {latest.assisted ? (
                <><Check className="h-3.5 w-3.5" aria-hidden="true" />已回答</>
              ) : (
                <><Sparkles className="h-3.5 w-3.5" aria-hidden="true" />回答</>
              )}
            </button>
            <button
              onClick={(e) => handleDismiss(0, e)}
              aria-label="忽略问题"
              className="rounded-lg p-1.5 text-muted-foreground/30 opacity-0 transition-all duration-150 hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100 cursor-pointer"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        )}
      </div>

      {/* Previous questions — useful in the full detector, hidden in the minimal overlay */}
      {!compact && previousQuestions.length > 0 && (
        <div className="flex flex-col gap-1">
          {previousQuestions.map((q, idx) => {
            const realIdx = idx + 1;
            return (
              <div
                key={`q-${realIdx}-${q.timestamp_ms}`}
                className={`group/q flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left transition-all duration-150 question-card-enter ${
                  q.assisted
                    ? "bg-success/5 border border-success/10"
                    : "bg-card/20 hover:bg-card/40 hover:border-border/20 cursor-pointer"
                }`}
                onClick={() => !q.assisted && handleAssist(realIdx)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); handleAssist(realIdx); } }}
                title={q.text}
              >
                <div className="shrink-0">
                  {q.assisted ? (
                    <div className="flex h-4 w-4 items-center justify-center rounded-full bg-success/20">
                      <Check className="h-2.5 w-2.5 text-success" />
                    </div>
                  ) : (
                    <div className="flex h-4 w-4 items-center justify-center rounded-full bg-muted/30">
                      <Clock className="h-2.5 w-2.5 text-muted-foreground/60" />
                    </div>
                  )}
                </div>

                <span className={`flex-1 truncate text-xs leading-snug transition-colors ${
                  q.assisted
                    ? "text-success/70 font-medium"
                    : "text-muted-foreground/60 group-hover/q:text-foreground/80"
                }`}>
                  {q.text}
                </span>

                {!q.assisted && (
                  <Sparkles className="h-3 w-3 shrink-0 text-info/0 group-hover/q:text-info/60 transition-colors" />
                )}
                <button
                  onClick={(e) => handleDismiss(realIdx, e)}
                  aria-label="忽略问题"
                  className="rounded p-0.5 text-muted-foreground/0 opacity-0 transition-all duration-150 hover:bg-destructive/10 hover:text-destructive group-hover/q:opacity-100 group-hover/q:text-muted-foreground/30 cursor-pointer"
                >
                  <X className="h-3 w-3" />
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
