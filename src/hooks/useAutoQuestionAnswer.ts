import { useEffect, useRef } from "react";
import { onQuestionDetected } from "../lib/events";
import { generateAssist } from "../lib/ipc";
import { useAIActionsStore } from "../stores/aiActionsStore";
import { useMeetingStore } from "../stores/meetingStore";
import { useStreamStore } from "../stores/streamStore";
import { showToast } from "../stores/toastStore";
import type { DetectedQuestion } from "../lib/types";

interface AutoQuestionAnswerOptions {
  enabled?: boolean;
}

function questionKey(question: DetectedQuestion): string {
  const normalized = question.text
    .replace(/[?？。！!\s]+$/g, "")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();

  // The timestamp makes two identical questions asked at different points in
  // one meeting independent, while still deduplicating the same backend event
  // if another frontend path observes it.
  return `${question.source}:${question.timestamp_ms}:${normalized}`;
}

function isRemoteQuestion(question: DetectedQuestion): boolean {
  return question.source === "Them" || question.source === "Interviewer";
}

/**
 * Own automatic answer scheduling in the launcher window.
 *
 * The overlay is a display surface and can be created after audio capture has
 * already started. Keeping this listener at the launcher/App level means a
 * pure API Qwen question does not depend on the overlay's mount timing.
 */
export function useAutoQuestionAnswer({ enabled = true }: AutoQuestionAnswerOptions = {}) {
  const autoTrigger = useAIActionsStore((state) => state.configs.globalDefaults.autoTrigger);
  const isStreaming = useStreamStore((state) => state.isStreaming);
  const meetingId = useMeetingStore((state) => state.activeMeeting?.id ?? null);

  const enabledRef = useRef(enabled);
  const autoTriggerRef = useRef(autoTrigger);
  const isStreamingRef = useRef(isStreaming);
  const meetingIdRef = useRef(meetingId);
  const queueRef = useRef<DetectedQuestion[]>([]);
  const seenRef = useRef<Set<string>>(new Set());
  const inFlightRef = useRef(false);
  const mountedRef = useRef(true);
  const drainRef = useRef<() => void>(() => {});

  useEffect(() => {
    enabledRef.current = enabled;
    autoTriggerRef.current = autoTrigger;
    isStreamingRef.current = isStreaming;
    meetingIdRef.current = meetingId;

    if (!meetingId) {
      queueRef.current = [];
      seenRef.current.clear();
    }

    drainRef.current();
  }, [enabled, autoTrigger, isStreaming, meetingId]);

  useEffect(() => {
    mountedRef.current = true;

    drainRef.current = () => {
      if (
        !enabledRef.current ||
        !autoTriggerRef.current ||
        !meetingIdRef.current ||
        inFlightRef.current ||
        isStreamingRef.current
      ) {
        return;
      }

      const question = queueRef.current.shift();
      if (!question) return;

      inFlightRef.current = true;
      let retryDelayMs = 0;
      void generateAssist("AskQuestion", question.text, "auto", "short")
        .catch((error) => {
          const message = error instanceof Error ? error.message : String(error);
          // A stream-start race is transient. Put that question back so it is
          // retried after the active answer finishes; real API errors are
          // surfaced once and are not retried indefinitely.
          if (message.toLowerCase().includes("already in progress")) {
            queueRef.current.unshift(question);
            retryDelayMs = 250;
          } else {
            showToast(message || "自动回答失败，请检查 Qwen API 配置", "error");
          }
        })
        .finally(() => {
          inFlightRef.current = false;
          if (mountedRef.current) {
            // Let the stream store publish its final state before draining the
            // next queued question. This also keeps answers ordered.
            window.setTimeout(() => drainRef.current(), retryDelayMs);
          }
        });
    };

    let unlisten: (() => void) | null = null;
    let listenerMounted = true;
    const setup = async () => {
      const cleanup = await onQuestionDetected((question) => {
        if (!listenerMounted || !enabledRef.current || !isRemoteQuestion(question)) {
          return;
        }

        const key = questionKey(question);
        if (seenRef.current.has(key)) return;
        seenRef.current.add(key);
        queueRef.current.push(question);
        // Keep a short bounded backlog if the interviewer asks several things
        // while Qwen is still producing the previous answer.
        if (queueRef.current.length > 3) queueRef.current.shift();
        drainRef.current();
      });

      if (listenerMounted) {
        unlisten = cleanup;
      } else {
        cleanup();
      }
    };

    void setup();

    return () => {
      listenerMounted = false;
      mountedRef.current = false;
      queueRef.current = [];
      if (unlisten) unlisten();
    };
  }, []);
}
