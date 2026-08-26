import { useCallback, useEffect, useRef, useState } from "react";
import {
  cancelGeneration,
  generateAssist,
  resetLLMSession,
} from "../lib/ipc";
import {
  onStreamEnd,
  onStreamError,
  onStreamStart,
  onStreamToken,
} from "../lib/events";
import {
  CheckCircle2,
  Loader2,
  MessageSquare,
  Play,
  RotateCcw,
  Send,
  Square,
  X,
} from "lucide-react";

interface MockInterviewPanelProps {
  onClose: () => void;
}

type MockTurn = {
  role: "interviewer" | "candidate";
  kind?: "question" | "feedback";
  text: string;
};

/**
 * A text-only practice loop. It intentionally does not start audio capture,
 * create a meeting record, or touch a meeting application.
 */
export function MockInterviewPanel({ onClose }: MockInterviewPanelProps) {
  const [topic, setTopic] = useState("");
  const [answer, setAnswer] = useState("");
  const [turns, setTurns] = useState<MockTurn[]>([]);
  const [streamText, setStreamText] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [started, setStarted] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeStream = useRef(false);
  const pendingKind = useRef<"question" | "feedback">("question");
  const streamTextRef = useRef("");

  useEffect(() => {
    const cleanups = [
      onStreamStart((event) => {
        if (event.mode !== "MockInterview") return;
        activeStream.current = true;
        setIsStreaming(true);
        setStreamText("");
      }),
      onStreamToken((event) => {
        if (!activeStream.current) return;
        streamTextRef.current += event.token;
        setStreamText(streamTextRef.current);
      }),
      onStreamEnd(() => {
        if (!activeStream.current) return;
        const text = streamTextRef.current.trim();
        if (text) {
          setTurns((current) => [
            ...current,
            { role: "interviewer", kind: pendingKind.current, text },
          ]);
        }
        activeStream.current = false;
        setIsStreaming(false);
        setStreamText("");
      }),
      onStreamError((message) => {
        if (!activeStream.current) return;
        activeStream.current = false;
        setIsStreaming(false);
        setError(message);
      }),
    ];

    return () => {
      cleanups.forEach((promise) => promise.then((unlisten) => unlisten()));
    };
    // The event listeners intentionally close over refs, not render state.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    streamTextRef.current = streamText;
  }, [streamText]);

  const runTurn = useCallback(async (prompt: string) => {
    setError(null);
    setStreamText("");
    streamTextRef.current = "";
    setIsStreaming(true);
    try {
      await generateAssist("MockInterview", prompt, "ask_codex", "normal");
    } catch (err) {
      activeStream.current = false;
      setIsStreaming(false);
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const startPractice = useCallback(async () => {
    const cleanTopic = topic.trim();
    if (!cleanTopic || isStreaming) return;
    setError(null);
    setTurns([]);
    setAnswer("");
    setStarted(true);
    pendingKind.current = "question";
    try {
      await resetLLMSession();
    } catch {
      // A stale provider session should not prevent text-only practice.
    }
    await runTurn(
      `Start a mock PhD interview about: ${cleanTopic}. Ask exactly one substantive interviewer question. Do not give feedback yet. Output only the question.`
    );
  }, [isStreaming, runTurn, topic]);

  const submitAnswer = useCallback(async () => {
    const cleanAnswer = answer.trim();
    if (!cleanAnswer || isStreaming || !started) return;
    setTurns((current) => [...current, { role: "candidate", text: cleanAnswer }]);
    setAnswer("");
    pendingKind.current = "feedback";
    await runTurn(
      `The candidate just answered: "${cleanAnswer}". Evaluate this answer briefly across five dimensions: technical correctness, English clarity, structure, confidence, and consistency with the supplied personal materials. Then ask exactly one concise follow-up question. Use the headings "Feedback" and "Next question". Do not invent facts about the candidate.`
    );
  }, [answer, isStreaming, runTurn, started]);

  const stopStreaming = useCallback(async () => {
    activeStream.current = false;
    setIsStreaming(false);
    try {
      await cancelGeneration();
    } catch {
      // Non-critical.
    }
  }, []);

  const resetPractice = useCallback(async () => {
    await stopStreaming();
    setTurns([]);
    setAnswer("");
    setError(null);
    setStarted(false);
  }, [stopStreaming]);

  const close = useCallback(async () => {
    if (isStreaming) await stopStreaming();
    onClose();
  }, [isStreaming, onClose, stopStreaming]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/65 p-4 backdrop-blur-sm">
      <div className="flex max-h-[min(760px,90vh)] w-[min(680px,95vw)] flex-col overflow-hidden rounded-2xl border border-border/40 bg-card shadow-2xl">
        <div className="flex items-center justify-between border-b border-border/20 px-5 py-4">
          <div className="flex items-center gap-2.5">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10">
              <MessageSquare className="h-4 w-4 text-primary" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-foreground">Mock Interview</h2>
              <p className="text-[11px] text-muted-foreground/60">Text-only practice · no audio or meeting control</p>
            </div>
          </div>
          <button onClick={close} className="rounded-lg p-1.5 text-muted-foreground/50 hover:bg-accent hover:text-foreground" aria-label="Close mock interview">
            <X className="h-4 w-4" />
          </button>
        </div>

        {!started ? (
          <div className="space-y-4 p-5">
            <div>
              <label className="mb-2 block text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/60">
                Practice topic
              </label>
              <textarea
                value={topic}
                onChange={(event) => setTopic(event.target.value)}
                onKeyDown={(event) => {
                  if ((event.ctrlKey || event.metaKey) && event.key === "Enter") startPractice();
                }}
                rows={4}
                maxLength={1000}
                placeholder="e.g. federated learning, your main project, or a PhD research proposal…"
                className="w-full resize-none rounded-xl border border-border/30 bg-secondary/10 px-3.5 py-3 text-sm leading-relaxed text-foreground outline-none placeholder:text-muted-foreground/40 focus:border-primary/50"
              />
            </div>
            {error && <p className="rounded-lg bg-destructive/10 px-3 py-2 text-xs text-destructive">{error}</p>}
            <button
              onClick={startPractice}
              disabled={!topic.trim() || isStreaming}
              className="flex w-full items-center justify-center gap-2 rounded-xl bg-primary py-3 text-sm font-semibold text-primary-foreground transition-colors hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-40"
            >
              {isStreaming ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" fill="currentColor" />}
              Start practice
            </button>
          </div>
        ) : (
          <>
            <div className="flex items-center justify-between border-b border-border/20 px-5 py-2.5">
              <p className="max-w-[470px] truncate text-xs text-muted-foreground/70" title={topic}>{topic}</p>
              <div className="flex items-center gap-2">
                {isStreaming && (
                  <button onClick={stopStreaming} className="flex items-center gap-1 rounded-md px-2 py-1 text-[11px] text-warning hover:bg-warning/10">
                    <Square className="h-3 w-3" /> Stop
                  </button>
                )}
                <button onClick={resetPractice} className="flex items-center gap-1 rounded-md px-2 py-1 text-[11px] text-muted-foreground/60 hover:bg-secondary/40 hover:text-foreground">
                  <RotateCcw className="h-3 w-3" /> Restart
                </button>
              </div>
            </div>

            <div className="min-h-0 flex-1 space-y-3 overflow-y-auto px-5 py-4">
              {turns.map((turn, index) => (
                <div key={`${turn.role}-${index}`} className={`rounded-xl border px-3.5 py-3 ${turn.role === "candidate" ? "ml-8 border-info/20 bg-info/5" : "mr-8 border-primary/20 bg-primary/5"}`}>
                  <div className="mb-1.5 flex items-center gap-2 text-[10px] font-semibold uppercase tracking-wider">
                    {turn.role === "candidate" ? "You" : turn.kind === "feedback" ? "Coach" : "Interviewer"}
                    {turn.kind === "feedback" && <CheckCircle2 className="h-3 w-3 text-success" />}
                  </div>
                  <p className="whitespace-pre-wrap text-sm leading-relaxed text-foreground/80">{turn.text}</p>
                </div>
              ))}
              {streamText && (
                <div className="mr-8 rounded-xl border border-primary/20 bg-primary/5 px-3.5 py-3">
                  <div className="mb-1.5 flex items-center gap-2 text-[10px] font-semibold uppercase tracking-wider text-primary">
                    {pendingKind.current === "feedback" ? "Coach" : "Interviewer"}
                    <Loader2 className="h-3 w-3 animate-spin" />
                  </div>
                  <p className="whitespace-pre-wrap text-sm leading-relaxed text-foreground/80">{streamText}</p>
                </div>
              )}
              {error && <p className="rounded-lg bg-destructive/10 px-3 py-2 text-xs text-destructive">{error}</p>}
            </div>

            <div className="border-t border-border/20 p-4">
              <div className="flex items-end gap-2 rounded-xl border border-border/30 bg-secondary/10 px-3 py-2">
                <textarea
                  value={answer}
                  onChange={(event) => setAnswer(event.target.value)}
                  onKeyDown={(event) => {
                    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") submitAnswer();
                  }}
                  rows={3}
                  maxLength={5000}
                  placeholder="Type your answer… (Ctrl/⌘ + Enter to submit)"
                  disabled={isStreaming}
                  className="min-h-[64px] flex-1 resize-none bg-transparent text-sm leading-relaxed text-foreground outline-none placeholder:text-muted-foreground/40 disabled:opacity-50"
                />
                <button
                  onClick={submitAnswer}
                  disabled={!answer.trim() || isStreaming}
                  className="rounded-lg bg-primary p-2 text-primary-foreground transition-colors hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-30"
                  aria-label="Submit answer"
                >
                  <Send className="h-4 w-4" />
                </button>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
