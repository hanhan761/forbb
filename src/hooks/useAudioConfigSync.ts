// Watches meetingAudioConfig changes during an active meeting
// and restarts the Rust audio capture pipeline with the new config.
// This enables "hot-swap" of STT provider and audio source mid-meeting.
//
// Key design: debounce (300ms) + sequential restart. Two separate refs:
// - appliedConfigRef: what Rust currently has running (only set after successful restart)
// - pendingConfigRef: set immediately to prevent duplicate debounce scheduling

import { useEffect, useRef } from "react";
import { useConfigStore } from "../stores/configStore";
import { useMeetingStore } from "../stores/meetingStore";
import { useDevLogStore } from "../stores/devLogStore";
import { useTranscriptStore } from "../stores/transcriptStore";
import { stopCapture, startCapturePerParty } from "../lib/ipc";
import { onAudioCaptureStatus } from "../lib/events";

export function useAudioConfigSync() {
  const isRecording = useMeetingStore((s) => s.isRecording);
  const meetingAudioConfig = useConfigStore((s) => s.meetingAudioConfig);

  // What Rust currently has running — only updated after successful restart
  const appliedConfigRef = useRef<string | null>(null);
  // What we've already scheduled a restart for — prevents duplicate debounces
  const pendingConfigRef = useRef<string | null>(null);
  // Guard against concurrent restart attempts
  const restartingRef = useRef(false);
  // Guard against duplicate backend recovery requests while a restart is in flight
  const recoveringRef = useRef(false);
  // Debounce timer to batch rapid config changes
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Surface backend device-loss recovery in the existing diagnostic/status
  // path so the user knows why the transcript may briefly pause.
  useEffect(() => {
    const unlistenPromise = onAudioCaptureStatus((event) => {
      if (!useMeetingStore.getState().isRecording) return;
      const addLog = useDevLogStore.getState().addEntry;
      const reason = event.reason ? `: ${event.reason}` : "";
      if (event.status === "degraded") {
        addLog("warn", "audio", `Audio capture degraded${reason}`);
      } else if (event.status === "recovering") {
        addLog("info", "audio", `Reconnecting audio capture${reason}`);
        if (recoveringRef.current || restartingRef.current) return;
        recoveringRef.current = true;
        void (async () => {
          try {
            const latestConfig = useConfigStore.getState().meetingAudioConfig;
            if (!latestConfig || !useMeetingStore.getState().isRecording) return;

            useTranscriptStore.getState().finalizeAllInterim();
            await stopCapture();
            await new Promise((resolve) => setTimeout(resolve, 200));

            if (!useMeetingStore.getState().isRecording) return;
            const freshConfig = useConfigStore.getState().meetingAudioConfig;
            if (!freshConfig) return;
            await startCapturePerParty(freshConfig.you, freshConfig.them);
            const freshKey = JSON.stringify(freshConfig);
            appliedConfigRef.current = freshKey;
            pendingConfigRef.current = freshKey;
            addLog("info", "audio", "Audio capture recovered");
          } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            addLog("error", "audio", `Audio capture recovery failed: ${message}`);
          } finally {
            recoveringRef.current = false;
          }
        })();
      } else if (event.status === "recovered") {
        addLog("info", "audio", "Audio capture recovered");
      } else if (event.status === "failed") {
        addLog("error", "audio", `Audio capture recovery failed${reason}`);
      }
    });
    return () => { unlistenPromise.then((unlisten) => unlisten()); };
  }, []);

  useEffect(() => {
    if (!isRecording || !meetingAudioConfig) return;

    const configKey = JSON.stringify(meetingAudioConfig);

    // On first run (meeting just started), record the applied config
    if (appliedConfigRef.current === null) {
      appliedConfigRef.current = configKey;
      pendingConfigRef.current = configKey;
      return;
    }

    // Already applied — nothing to do
    if (appliedConfigRef.current === configKey) return;

    // Already scheduled a restart for this exact config
    if (pendingConfigRef.current === configKey) return;

    // Mark this config as pending (prevents duplicate debounce scheduling)
    pendingConfigRef.current = configKey;

    const log = useDevLogStore.getState().addEntry;
    const desc = `you=${meetingAudioConfig.you.stt_provider}` +
      (meetingAudioConfig.you.local_model_id ? `(${meetingAudioConfig.you.local_model_id})` : "") +
      `, them=${meetingAudioConfig.them.stt_provider}` +
      (meetingAudioConfig.them.local_model_id ? `(${meetingAudioConfig.them.local_model_id})` : "");
    log("info", "config", `STT config changed → ${desc}`);

    // Cancel any existing debounce — the new config supersedes it
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    debounceRef.current = setTimeout(async () => {
      debounceRef.current = null;

      // If already restarting, clear pending so next effect re-schedules
      if (restartingRef.current) {
        log("warn", "config", "Hot-swap already in progress — queued for retry");
        pendingConfigRef.current = null;
        return;
      }

      restartingRef.current = true;

      // Read the latest config (may have changed during debounce wait)
      const latestConfig = useConfigStore.getState().meetingAudioConfig;
      if (!latestConfig) {
        restartingRef.current = false;
        return;
      }

      const latestKey = JSON.stringify(latestConfig);

      // If the latest config matches what's already running, skip
      if (appliedConfigRef.current === latestKey) {
        pendingConfigRef.current = latestKey;
        restartingRef.current = false;
        return;
      }

      log("info", "config", "Hot-swap: stopping current capture...");

      try {
        // Finalize any interim (non-final) transcript segments before restarting,
        // so they don't stay stuck as gray italic after the new provider takes over.
        useTranscriptStore.getState().finalizeAllInterim();

        // Stop current capture and wait for full cleanup
        await stopCapture();
        log("info", "config", "Hot-swap: capture stopped, waiting for resource release...");

        // Let Rust fully release WASAPI/audio resources
        await new Promise((r) => setTimeout(r, 200));

        // Re-read config in case it changed during the stop
        const freshConfig = useConfigStore.getState().meetingAudioConfig;
        if (!freshConfig) {
          log("warn", "config", "Hot-swap: no config available after stop");
          return;
        }

        log("info", "config", "Hot-swap: starting new capture pipeline...");
        await startCapturePerParty(freshConfig.you, freshConfig.them);
        const freshKey = JSON.stringify(freshConfig);
        appliedConfigRef.current = freshKey;
        pendingConfigRef.current = freshKey;
        log("info", "config", "Hot-swap complete — new STT pipeline active");
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        log("error", "config", `Hot-swap FAILED: ${msg}`);
        // Reset both refs so next config change retries
        appliedConfigRef.current = null;
        pendingConfigRef.current = null;
      } finally {
        restartingRef.current = false;
      }
    }, 300);
  }, [isRecording, meetingAudioConfig]);

  // Reset when meeting ends
  useEffect(() => {
    if (!isRecording) {
      appliedConfigRef.current = null;
      pendingConfigRef.current = null;
      restartingRef.current = false;
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
        debounceRef.current = null;
      }
    }
  }, [isRecording]);
}
