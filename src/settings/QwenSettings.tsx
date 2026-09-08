import { useEffect, useState } from "react";
import { CheckCircle, Eye, EyeOff, Loader2, Sparkles, XCircle } from "lucide-react";
import { useConfigStore } from "../stores/configStore";
import { useTranslationStore } from "../stores/translationStore";
import {
  deleteApiKey,
  getApiKey,
  setActiveModel,
  setLLMProvider,
  setSTTProvider,
  storeApiKey,
  testLLMConnection,
  testSTTConnection,
} from "../lib/ipc";

type ConnectionStatus = "idle" | "testing" | "success" | "error";

const QWEN_MODELS = ["qwen-plus", "qwen-turbo", "qwen-max"];

export function QwenSettings() {
  const llmModel = useConfigStore((state) => state.llmModel);
  const setConfigProvider = useConfigStore((state) => state.setLLMProvider);
  const setConfigModel = useConfigStore((state) => state.setLLMModel);
  const meetingAudioConfig = useConfigStore((state) => state.meetingAudioConfig);
  const setMeetingAudioConfig = useConfigStore((state) => state.setMeetingAudioConfig);
  const sttLanguage = useConfigStore((state) => state.sttLanguage);
  const setSTTLanguage = useConfigStore((state) => state.setSTTLanguage);

  const targetLang = useTranslationStore((state) => state.targetLang);
  const sourceLang = useTranslationStore((state) => state.sourceLang);
  const displayMode = useTranslationStore((state) => state.displayMode);
  const autoTranslateEnabled = useTranslationStore((state) => state.autoTranslateEnabled);
  const selectionToolbarEnabled = useTranslationStore((state) => state.selectionToolbarEnabled);
  const cacheEnabled = useTranslationStore((state) => state.cacheEnabled);
  const setTranslationProvider = useTranslationStore((state) => state.setProvider);
  const setTargetLang = useTranslationStore((state) => state.setTargetLang);
  const setSourceLang = useTranslationStore((state) => state.setSourceLang);
  const setDisplayMode = useTranslationStore((state) => state.setDisplayMode);
  const setAutoTranslateEnabled = useTranslationStore((state) => state.setAutoTranslateEnabled);
  const setSelectionToolbarEnabled = useTranslationStore((state) => state.setSelectionToolbarEnabled);
  const setCacheEnabled = useTranslationStore((state) => state.setCacheEnabled);

  const [apiKey, setApiKey] = useState("");
  const [showApiKey, setShowApiKey] = useState(false);
  const [model, setModel] = useState(QWEN_MODELS.includes(llmModel) ? llmModel : "qwen-plus");
  const [status, setStatus] = useState<ConnectionStatus>("idle");
  const [message, setMessage] = useState("");

  useEffect(() => {
    getApiKey("qwen")
      .then((key) => setApiKey(key || ""))
      .catch(() => setApiKey(""));
  }, []);

  function qwenConfig(key: string) {
    return JSON.stringify({ provider_type: "qwen", api_key: key });
  }

  async function syncQwen(key: string) {
    await storeApiKey("qwen", key);
    await setLLMProvider(qwenConfig(key));
    await setActiveModel("qwen", model);
    await setSTTProvider("qwen_asr");
    setConfigProvider("qwen");
    setConfigModel(model);
    setTranslationProvider("llm");

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
    if (!key) return;
    try {
      await syncQwen(key);
      setMessage("已保存到本机凭据管理器");
      setStatus("idle");
    } catch (error) {
      setStatus("error");
      setMessage(error instanceof Error ? error.message : "保存失败");
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
      if (!llmReady || !sttReady) throw new Error("文本或语音服务连接失败");
      setStatus("success");
      setMessage("连接成功，全部 AI 功能已使用 Qwen");
    } catch (error) {
      setStatus("error");
      setMessage(error instanceof Error ? error.message : "连接失败，请检查 API Key 和网络");
    }
  }

  async function handleClear() {
    await deleteApiKey("qwen").catch(() => {});
    setApiKey("");
    setStatus("idle");
    setMessage("已清除。下次启动时需要重新配置 Qwen API Key。");
  }

  return (
    <div className="space-y-6">
      <div>
        <div className="mb-2 flex items-center gap-2">
          <Sparkles className="h-5 w-5 text-primary" />
          <h2 className="text-lg font-semibold text-foreground">通义千问 API</h2>
        </div>
        <p className="text-sm leading-relaxed text-muted-foreground">
          NexQ 只连接 Qwen：文本回答、翻译、语音识别和会议总结共用同一个 API Key。
        </p>
      </div>

      <section className="rounded-xl border border-border/30 bg-card/40 p-5">
        <h3 className="mb-3 text-sm font-semibold text-foreground">API Key</h3>
        <div className="relative">
          <input
            type={showApiKey ? "text" : "password"}
            value={apiKey}
            onChange={(event) => {
              setApiKey(event.target.value);
              setStatus("idle");
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
            {showApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
          </button>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-3">
          <button
            type="button"
            onClick={handleTest}
            disabled={status === "testing" || !apiKey.trim()}
            className="inline-flex items-center gap-2 rounded-lg bg-primary px-3.5 py-2 text-xs font-semibold text-primary-foreground hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {status === "testing" ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <CheckCircle className="h-3.5 w-3.5" />}
            保存并测试
          </button>
          <button
            type="button"
            onClick={handleClear}
            disabled={!apiKey}
            className="rounded-lg border border-border/40 px-3.5 py-2 text-xs font-medium text-muted-foreground hover:bg-accent disabled:cursor-not-allowed disabled:opacity-40"
          >
            清除 Key
          </button>
          {status === "success" && <span className="flex items-center gap-1 text-xs text-success"><CheckCircle className="h-3.5 w-3.5" />{message}</span>}
          {status === "error" && <span className="flex items-center gap-1 text-xs text-destructive"><XCircle className="h-3.5 w-3.5" />{message}</span>}
          {status === "idle" && message && <span className="text-xs text-muted-foreground">{message}</span>}
        </div>
        <a
          className="mt-3 inline-block text-xs text-primary hover:underline"
          href="https://help.aliyun.com/zh/model-studio/get-api-key"
          target="_blank"
          rel="noreferrer"
        >
          如何创建 Qwen API Key →
        </a>
      </section>

      <section className="grid gap-4 rounded-xl border border-border/30 bg-card/40 p-5 sm:grid-cols-2">
        <label className="space-y-2 text-sm text-foreground">
          <span className="font-semibold">Qwen 文本模型</span>
          <select
            value={model}
            onChange={(event) => {
              setModel(event.target.value);
              setConfigModel(event.target.value);
            }}
            className="w-full rounded-lg border border-border/40 bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
          >
            {QWEN_MODELS.map((item) => <option key={item} value={item}>{item}</option>)}
          </select>
        </label>
        <label className="space-y-2 text-sm text-foreground">
          <span className="font-semibold">语音识别语言</span>
          <select
            value={sttLanguage}
            onChange={(event) => setSTTLanguage(event.target.value)}
            className="w-full rounded-lg border border-border/40 bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
          >
            <option value="zh-CN">中文</option>
            <option value="en-US">English</option>
            <option value="ja-JP">日本語</option>
            <option value="ko-KR">한국어</option>
            <option value="auto">自动识别</option>
          </select>
        </label>
      </section>

      <section className="space-y-4 rounded-xl border border-border/30 bg-card/40 p-5">
        <div>
          <h3 className="text-sm font-semibold text-foreground">翻译</h3>
          <p className="mt-1 text-xs text-muted-foreground">翻译默认通过 Qwen LLM 完成，不需要第二个服务商。</p>
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <label className="space-y-2 text-sm text-foreground">
            <span>源语言</span>
            <select value={sourceLang} onChange={(event) => setSourceLang(event.target.value)} className="w-full rounded-lg border border-border/40 bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none">
              <option value="auto">自动识别</option>
              <option value="zh">中文</option>
              <option value="en">English</option>
              <option value="ja">日本語</option>
              <option value="ko">한국어</option>
            </select>
          </label>
          <label className="space-y-2 text-sm text-foreground">
            <span>目标语言</span>
            <select value={targetLang} onChange={(event) => setTargetLang(event.target.value)} className="w-full rounded-lg border border-border/40 bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none">
              <option value="zh">中文</option>
              <option value="en">English</option>
              <option value="ja">日本語</option>
              <option value="ko">한국어</option>
            </select>
          </label>
        </div>
        <ToggleRow label="自动翻译字幕" checked={autoTranslateEnabled} onChange={setAutoTranslateEnabled} />
        <ToggleRow label="显示选中文本翻译工具栏" checked={selectionToolbarEnabled} onChange={setSelectionToolbarEnabled} />
        <ToggleRow label="缓存翻译结果" checked={cacheEnabled} onChange={setCacheEnabled} />
        <ToggleRow label="翻译显示位置：悬浮" checked={displayMode === "hover"} onChange={(enabled) => setDisplayMode(enabled ? "hover" : "inline")} />
      </section>
    </div>
  );
}

function ToggleRow({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return (
    <label className="flex cursor-pointer items-center justify-between gap-4 text-sm text-foreground">
      <span>{label}</span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative h-5 w-9 rounded-full transition-colors ${checked ? "bg-primary" : "bg-muted"}`}
      >
        <span className={`absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-white shadow-sm transition-transform ${checked ? "translate-x-4" : "translate-x-0"}`} />
      </button>
    </label>
  );
}
