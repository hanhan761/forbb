import { useCallback, useState } from "react";
import { useConfigStore } from "../../stores/configStore";
import { useMeetingStore } from "../../stores/meetingStore";
import { AudioSetupStep } from "./AudioSetupStep";
import { LLMSetupStep } from "./LLMSetupStep";
import { ReadyStep } from "./ReadyStep";
import { ChevronLeft, ChevronRight } from "lucide-react";

const STEP_COUNT = 3;
const STEP_LABELS = ["Qwen API", "音频", "完成"];

export function FirstRunWizard() {
  const setFirstRunCompleted = useConfigStore((state) => state.setFirstRunCompleted);
  const startMeetingFlow = useMeetingStore((state) => state.startMeetingFlow);
  const setSettingsOpen = useMeetingStore((state) => state.setSettingsOpen);

  const [currentStep, setCurrentStep] = useState(0);
  const [qwenReady, setQwenReady] = useState(false);
  const [isAnimating, setIsAnimating] = useState(false);
  const [slideDirection, setSlideDirection] = useState<"left" | "right">("right");

  const onQwenReadyChange = useCallback((ready: boolean) => {
    setQwenReady(ready);
  }, []);

  const animateToStep = useCallback(
    (nextStep: number) => {
      if (isAnimating) return;
      setSlideDirection(nextStep > currentStep ? "right" : "left");
      setIsAnimating(true);
      requestAnimationFrame(() => {
        setCurrentStep(nextStep);
        setTimeout(() => setIsAnimating(false), 300);
      });
    },
    [currentStep, isAnimating]
  );

  const handleNext = useCallback(() => {
    if (currentStep < STEP_COUNT - 1 && (currentStep !== 0 || qwenReady)) {
      animateToStep(currentStep + 1);
    }
  }, [animateToStep, currentStep, qwenReady]);

  const handleBack = useCallback(() => {
    if (currentStep > 0) animateToStep(currentStep - 1);
  }, [animateToStep, currentStep]);

  const handleStartMeeting = useCallback(async () => {
    setFirstRunCompleted(true);
    try {
      await startMeetingFlow();
    } catch (error) {
      console.error("[FirstRunWizard] Failed to start meeting:", error);
    }
  }, [setFirstRunCompleted, startMeetingFlow]);

  const handleGoToLauncher = useCallback(() => {
    setFirstRunCompleted(true);
  }, [setFirstRunCompleted]);

  const handleOpenContext = useCallback(() => {
    setFirstRunCompleted(true);
    setTimeout(() => setSettingsOpen(true), 100);
  }, [setFirstRunCompleted, setSettingsOpen]);

  const canNext = currentStep !== 0 || qwenReady;

  return (
    <div className="flex h-full flex-col bg-background">
      <header className="flex items-center justify-between border-b border-border/20 px-8 py-4">
        <div className="flex items-center gap-2.5">
          <span className="text-base font-bold tracking-tight text-foreground">NexQ</span>
          <span className="text-sm font-medium text-muted-foreground/60">首次设置</span>
        </div>
        <div className="flex items-center gap-2">
          {STEP_LABELS.map((label, index) => (
            <button
              key={label}
              type="button"
              onClick={() => {
                if (index <= currentStep || (index === currentStep + 1 && canNext)) {
                  animateToStep(index);
                }
              }}
              className="flex items-center gap-1.5"
              aria-label={label}
              aria-current={currentStep === index ? "step" : undefined}
            >
              <span
                className={`rounded-full transition-all duration-300 ${
                  index === currentStep
                    ? "h-2.5 w-8 bg-primary shadow-sm shadow-primary/30"
                    : index < currentStep
                      ? "h-2.5 w-2.5 bg-primary/50"
                      : "h-2.5 w-2.5 bg-border/40"
                }`}
              />
              <span className="hidden text-xs text-muted-foreground/70 sm:inline">{label}</span>
            </button>
          ))}
        </div>
        <span className="text-xs font-medium text-muted-foreground/70">
          {currentStep + 1} / {STEP_COUNT}
        </span>
      </header>

      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-2xl px-8 py-10">
          <div
            className={`transition-all duration-300 ease-out ${
              isAnimating
                ? slideDirection === "right"
                  ? "translate-x-4 opacity-0"
                  : "-translate-x-4 opacity-0"
                : "translate-x-0 opacity-100"
            }`}
          >
            {currentStep === 0 && <LLMSetupStep onReadyChange={onQwenReadyChange} />}
            {currentStep === 1 && <AudioSetupStep />}
            {currentStep === 2 && (
              <ReadyStep
                onStartMeeting={handleStartMeeting}
                onGoToLauncher={handleGoToLauncher}
                onOpenContext={handleOpenContext}
              />
            )}
          </div>
        </div>
      </div>

      <footer className="flex items-center justify-between border-t border-border/20 px-8 py-4">
        <div>
          {currentStep > 0 && currentStep < STEP_COUNT - 1 && (
            <button
              type="button"
              onClick={handleBack}
              className="inline-flex items-center gap-1.5 rounded-xl px-4 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              <ChevronLeft className="h-4 w-4" />
              上一步
            </button>
          )}
        </div>

        {currentStep < STEP_COUNT - 1 && (
          <button
            type="button"
            onClick={handleNext}
            disabled={!canNext}
            className="inline-flex items-center gap-1.5 rounded-xl bg-primary px-5 py-2 text-sm font-semibold text-primary-foreground shadow-sm transition-all hover:bg-primary/90 hover:shadow-md disabled:cursor-not-allowed disabled:opacity-50"
          >
            下一步
            <ChevronRight className="h-4 w-4" />
          </button>
        )}
      </footer>
    </div>
  );
}
