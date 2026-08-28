import { useCallback } from "react";
import { useMeetingStore } from "../stores/meetingStore";
import { useAIActionsStore } from "../stores/aiActionsStore";
import { useTranslationStore } from "../stores/translationStore";
import { showToast } from "../stores/toastStore";
import { TranscriptPanel } from "./TranscriptPanel";
import { QuestionDetector } from "./QuestionDetector";
import { AIResponsePanel } from "./AIResponsePanel";
import { ModeButtons } from "./ModeButtons";
import { useConfigStore } from "../stores/configStore";
import { useSpeakerDetection } from "../hooks/useSpeakerDetection";
import { useTopicDetection } from "../hooks/useTopicDetection";
import { showLauncherWindow } from "../lib/windows";
import {
  GripHorizontal,
  Minus,
  Settings,
  Square,
  Globe,
  Sparkles,
} from "lucide-react";
import { formatDuration } from "../lib/utils";

// ════════════════════════════════════════════════════════════════
export function OverlayView() {
  const activeMeeting = useMeetingStore((s) => s.activeMeeting);
  const elapsedMs = useMeetingStore((s) => s.elapsedMs);
  const isRecording = useMeetingStore((s) => s.isRecording);
  const endMeetingFlow = useMeetingStore((s) => s.endMeetingFlow);
  const setCurrentView = useMeetingStore((s) => s.setCurrentView);
  const autoTrigger = useAIActionsStore((s) => s.configs.globalDefaults.autoTrigger);
  const overlayOpacity = useConfigStore((s) => s.overlayOpacity);

  const autoTranslateActive = useTranslationStore((s) => s.autoTranslateActive);
  const setAutoTranslateActive = useTranslationStore((s) => s.setAutoTranslateActive);
  const targetLang = useTranslationStore((s) => s.targetLang);

  // Speaker detection from Deepgram diarization events
  useSpeakerDetection();

  // Live topic detection from backend events
  useTopicDetection();

  const handleEndMeeting = useCallback(async () => {
    try { await endMeetingFlow(); showToast("Meeting ended", "info"); }
    catch (err) { showToast(err instanceof Error ? err.message : "Couldn't end meeting", "error"); }
  }, [endMeetingFlow]);

  const handleMinimizeToDashboard = useCallback(() => {
    setCurrentView("launcher");
    showLauncherWindow().catch(() => {});
  }, [setCurrentView]);

  const meetingTitle = activeMeeting?.title || "NexQ";

  return (
    <div className="overlay-bg flex h-full flex-col rounded-xl border border-border/20 shadow-xl" style={{ background: `hsl(var(--background) / ${overlayOpacity})`, backdropFilter: overlayOpacity > 0.7 ? "blur(12px) saturate(1.1)" : "none" }}>

      {/* ═══ HEADER ═══ */}
      <div
        className="no-select flex items-center justify-between gap-2 px-3 py-2 cursor-move"
        data-tauri-drag-region
        style={{ borderBottom: "1px solid hsl(var(--border) / 0.12)" }}
      >
        <div className="flex min-w-0 items-center gap-2" data-tauri-drag-region>
          <GripHorizontal className="h-3 w-3 text-muted-foreground/40" />
          <span className="shrink-0 text-xs font-semibold text-foreground/90">
            NexQ
          </span>
          <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-success" aria-label="Meeting active" />
          <span className="truncate text-xs text-muted-foreground/70" title={meetingTitle}>
            {meetingTitle}
          </span>
          {isRecording && (
            <span className="shrink-0 text-[9px] font-semibold tracking-wider text-destructive">
              REC
            </span>
          )}
          <span className="shrink-0 text-xs tabular-nums font-medium text-muted-foreground/60">
            {elapsedMs > 0 ? formatDuration(elapsedMs) : "00:00"}
          </span>
        </div>

        <div className="flex shrink-0 items-center gap-0.5">
          <button
            onClick={() => setAutoTranslateActive(!autoTranslateActive)}
            className={`flex items-center gap-1 rounded-md px-2 py-1 text-meta font-medium transition-all ${
              autoTranslateActive
                ? "bg-primary/10 text-primary ring-1 ring-primary/20"
                : "text-muted-foreground hover:bg-accent"
            }`}
            title={autoTranslateActive ? "Hide translation" : "Show translation"}
            aria-label={autoTranslateActive ? "Hide translation" : "Show translation"}
            aria-pressed={autoTranslateActive}
          >
            <Globe className="h-3 w-3" />
            {autoTranslateActive ? targetLang.toUpperCase() : "Original"}
          </button>
          <HeaderBtn icon={<Settings className="h-3.5 w-3.5" />} onClick={() => setCurrentView("settings")} tooltip="Settings" />
          <HeaderBtn icon={<Minus className="h-3.5 w-3.5" />} onClick={handleMinimizeToDashboard} tooltip="Minimize to Dashboard" />
          <button
            onClick={handleEndMeeting}
            className="ml-1 flex items-center gap-1 rounded-lg border border-destructive/20 bg-destructive/10 px-2.5 py-1.5 text-meta font-semibold text-destructive transition-all duration-150 hover:bg-destructive/20 cursor-pointer"
            aria-label="End meeting"
          >
            <Square className="h-3 w-3 fill-current" aria-hidden="true" />
            End
          </button>
        </div>
      </div>

      {/* ═══ MAIN ═══ */}
      <div className="relative flex-1 min-h-0">
      <div className="absolute inset-0 flex min-h-0 flex-col gap-2 overflow-hidden px-3 py-2.5">
        {/* ── LIVE SUBTITLES ── */}
        <section className="flex min-h-0 flex-[1.1] flex-col overflow-hidden rounded-xl border border-border/20 bg-card/20">
          <div className="flex shrink-0 items-center justify-between border-b border-border/20 px-3 py-2">
            <span className="text-meta font-semibold uppercase tracking-wider text-muted-foreground/60">Subtitles</span>
            <span className="text-meta text-muted-foreground/40">
              {autoTranslateActive ? `Original + ${targetLang.toUpperCase()}` : "Original only"}
            </span>
          </div>
          <div className="flex min-h-0 flex-1 flex-col overflow-hidden p-2.5">
            <TranscriptPanel compact />
          </div>
        </section>

        {/* ── AI ASSISTANT ── */}
        <section className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border border-border/20 bg-card/20">
          <div className="flex shrink-0 items-center justify-between border-b border-border/20 px-3 py-2">
            <div className="flex items-center gap-1.5">
              <Sparkles className="h-3.5 w-3.5 text-primary/70" aria-hidden="true" />
              <span className="text-meta font-semibold uppercase tracking-wider text-muted-foreground/60">AI Assistant</span>
            </div>
            <span className="text-meta text-muted-foreground/40">
              {autoTrigger ? "Auto + Space" : "Space to ask"}
            </span>
          </div>

          {autoTrigger && (
            <div className="shrink-0 border-b border-border/15 bg-info/5 px-3 py-2">
              <QuestionDetector compact />
            </div>
          )}

          <div className="flex shrink-0 items-center border-b border-border/20 px-2.5 py-1.5">
            <ModeButtons compact />
          </div>
          <div className="flex min-h-0 flex-1 flex-col p-3">
            <AIResponsePanel compact />
          </div>
        </section>
      </div>
      </div>
    </div>
  );
}

// ── Header Button ──
function HeaderBtn({ icon, active, onClick, tooltip }: { icon: React.ReactNode; active?: boolean; onClick: () => void; tooltip: string }) {
  return (
    <button
      onClick={onClick}
      className={`rounded-lg p-2 transition-all duration-150 cursor-pointer ${
        active ? "bg-primary/10 text-primary" : "text-muted-foreground/60 hover:bg-accent/60 hover:text-foreground"
      }`}
      aria-label={tooltip}
      aria-pressed={active}
    >
      {icon}
    </button>
  );
}


