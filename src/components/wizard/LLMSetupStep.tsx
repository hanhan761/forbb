import { useEffect, useState } from "react";
import { useConfigStore } from "../../stores/configStore";
import {
  getApiKey,
  setActiveModel,
  setLLMProvider,
  setSTTProvider,
  storeApiKey,
  testLLMConnection,
  testSTTConnection,
} from "../../lib/ipc";
import { CheckCircle, Eye, EyeOff, Loader2, Sparkles, XCircle } from "lucide-react";

interface LLMSetupStepProps {
  onReadyChange?: (ready: boolean) => void;
}

type ConnectionStatus = "idle" | "testing" | "success" | "error";

const QWEN_MODEL = "qwen-plus";

/** First-run setup for the only supported AI service: Qwen. */
export function LLMSetupStep({ onReadyChange }: LLMSetupStepProps) {
  const setConfigProvider = useConfigStore((state) => state.setLLMProvider);
  const setConfigModel = useConfigStore((state) => state.setLLMModel);
  const meetingAudioConfig = useConfigStore((state) => state.meetingAudioConfig);
  const setMeetingAudioConfig = useConfigStore((state) => state.setMeetingAudioConfig);

  const [apiKey, setApiKey] = useState("");
  const [showApiKey, setShowApiKey] = useState(false);
  const [status, setStatus] = useState<ConnectionStatus>("idle");
  const [message, setMessage] = useState("");

  useEffect(() => {
    getApiKey("qwen")
      .then((key) => {
        const value = key || "";
        setApiKey(value);
        onReadyChange?.(Boolean(value.trim()));
      })
      .catch(() => onReadyChange?.(false));
  }, [onReadyChange]);

  function qwenConfig(key: string) {
    return JSON.stringify({
      provider_type: "qwen",
      api_key: key,
    });
  }

  async function syncQwen(key: string) {
    await storeApiKey("qwen", key);
    await setLLMProvider(qwenConfig(key));
    await setActiveModel("qwen", QWEN_MODEL);
    setConfigProvider("qwen");
    setConfigModel(QWEN_MODEL);
    await setSTTProvider("qwen_asr");

    if (meetingAudioConfig) {
      setMeetingAudioConfig({
        ...meetingAudioConfig,
        you: { ...meetingAudioConfig.you, stt_provider: "qwen_asr", local_model_id: undefined },
        them: { ...meetingAudioConfig.them, stt_provider: "qwen_asr", local_model_id: undefined },
        preset_name: null,
      });
    }
  }

  async function handleSave() {
    const key = apiKey.trim();
    if (!key) {
      onReadyChange?.(false);
      return;
    }
    try {
      await syncQwen(key);
      onReadyChange?.(true);
      setMessage("已保存。可以直接进入下一步。");
    } catch (error) {
      setStatus("error");
      setMessage(error instanceof Error ? error.message : "保存失败，请检查 API Key");
    }
  }

  async function handleTest() {
    const key = apiKey.trim();
    if (!key) return;

    setStatus("testing");
    setMessage("");
    try {
      await syncQwen(key);
      const [llmReady, sttReady] = await Promise.all([
        testLLMConnection(qwenConfig(key)),
        testSTTConnection("qwen_asr"),
      ]);
      if (!llmReady || !sttReady) {
        throw new Error("Qwen 文本或语音服务连接失败");
      }
      setStatus("success");
      setMessage("连接成功：文本、翻译、语音识别共用这个 Qwen API Key");
      onReadyChange?.(true);
    } catch (error) {
      setStatus("error");
      setMessage(error instanceof Error ? error.message : "连接失败，请检查 API Key 和网络");
      onReadyChange?.(Boolean(key));
    }
  }

  return (
    <div className="flex flex-col items-center">
      <div className="mb-8 text-center">
        <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-primary/10 shadow-md shadow-primary/10">
          <Sparkles className="h-7 w-7 text-primary" />
        </div>
        <h2 className="text-2xl font-bold text-foreground">先配置通义千问</h2>
        <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
          NexQ 只需要一个 Qwen API Key。文本、翻译和语音识别都会使用它。
        </p>
      </div>

      <div className="w-full max-w-lg space-y-5">
        <div className="rounded-xl border border-primary/20 bg-primary/5 px-5 py-4 text-sm leading-relaxed text-foreground/80">
          API Key 只保存在本机系统凭据管理器中，不会写入项目文件。
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium text-foreground" htmlFor="qwen-api-key">
            Qwen API Key
          </label>
          <div className="relative">
            <input
              id="qwen-api-key"
              type={showApiKey ? "text" : "password"}
              value={apiKey}
              onChange={(event) => {
                const value = event.target.value;
                setApiKey(value);
                setStatus("idle");
                onReadyChange?.(false);
              }}
              onBlur={handleSave}
              placeholder="sk-..."
              className="w-full rounded-xl border border-border/40 bg-background px-4 py-3 pr-11 text-sm text-foreground placeholder:text-muted-foreground/50 focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary/20"
            />
            <button
              type="button"
              onClick={() => setShowApiKey((visible) => !visible)}
              className="absolute right-2.5 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground"
              aria-label={showApiKey ? "隐藏 API Key" : "显示 API Key"}
            >
              {showApiKey ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
            </button>
          </div>
          <p className="text-xs text-muted-foreground">
            在阿里云百炼控制台创建 API Key，然后粘贴到这里。
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            onClick={handleTest}
            disabled={status === "testing" || !apiKey.trim()}
            className="inline-flex items-center gap-2 rounded-xl bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground transition-colors hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {status === "testing" ? <Loader2 className="h-4 w-4 animate-spin" /> : <CheckCircle className="h-4 w-4" />}
            测试 Qwen 连接
          </button>
          {status === "success" && (
            <span className="flex items-center gap-1 text-xs text-success">
              <CheckCircle className="h-3.5 w-3.5" />
              {message}
            </span>
          )}
          {status === "error" && (
            <span className="flex items-center gap-1 text-xs text-destructive">
              <XCircle className="h-3.5 w-3.5" />
              {message}
            </span>
          )}
        </div>

        {status === "idle" && message && (
          <p className="text-xs text-success">{message}</p>
        )}
      </div>
    </div>
  );
}
