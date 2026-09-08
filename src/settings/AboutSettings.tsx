import {
  Github,
  FileText,
  AlertCircle,
  HelpCircle,
} from "lucide-react";
import { NEXQ_VERSION, NEXQ_BUILD_DATE, NEXQ_DEVELOPER } from "../lib/version";
import { open } from "@tauri-apps/plugin-shell";

const GITHUB_URL = "https://github.com/hanhan761/forbb";

function formatBuildDate(dateStr: string): string {
  try {
    const d = new Date(dateStr);
    return d.toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr;
  }
}

export function AboutSettings() {
  return (
    <div className="space-y-6">
      {/* App Identity Card */}
      <div className="rounded-xl border border-border/30 bg-card/50 p-6">
        <div className="flex items-start gap-5">
          <img src="/nexq-icon.png" alt="NexQ" className="h-14 w-14 shrink-0 rounded-2xl" />
          <div>
            <h3 className="text-lg font-bold text-foreground">NexQ</h3>
            <p className="text-xs text-muted-foreground">
              v{NEXQ_VERSION}
            </p>
            <p className="mt-2 text-sm text-muted-foreground">
              AI Meeting Assistant &amp; Real-Time Interview Copilot
            </p>
            <div className="mt-3 flex flex-wrap items-center gap-2">
              <span className="inline-flex items-center rounded-full bg-secondary/50 px-3 py-1 text-meta font-medium text-muted-foreground">
                Tauri 2
              </span>
              <span className="inline-flex items-center rounded-full bg-secondary/50 px-3 py-1 text-meta font-medium text-muted-foreground">
                React + Rust
              </span>
              <span className="inline-flex items-center rounded-full bg-secondary/50 px-3 py-1 text-meta font-medium text-muted-foreground">
                Windows x64
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Meta Grid (2x2) */}
      <div className="grid grid-cols-2 gap-3">
        <div className="rounded-xl border border-border/30 bg-card/50 p-4">
          <p className="text-meta text-muted-foreground/60">Build Date</p>
          <p className="mt-1 text-sm font-medium text-foreground">
            {formatBuildDate(NEXQ_BUILD_DATE)}
          </p>
        </div>
        <div className="rounded-xl border border-border/30 bg-card/50 p-4">
          <p className="text-meta text-muted-foreground/60">Developer</p>
          <p className="mt-1 text-sm font-medium text-foreground">
            {NEXQ_DEVELOPER}
          </p>
        </div>
        <div className="rounded-xl border border-border/30 bg-card/50 p-4">
          <p className="text-meta text-muted-foreground/60">Architecture</p>
          <p className="mt-1 text-sm font-medium text-foreground">x86_64</p>
        </div>
        <div className="rounded-xl border border-border/30 bg-card/50 p-4">
          <p className="text-meta text-muted-foreground/60">License</p>
          <p className="mt-1 text-sm font-medium text-foreground">MIT</p>
        </div>
      </div>

      {/* Quick Links Row */}
      <div className="grid grid-cols-4 gap-3">
        <button
          onClick={() => open(GITHUB_URL)}
          className="flex flex-col items-center gap-2 rounded-xl border border-border/30 bg-card/50 p-4 transition-colors hover:bg-secondary/30"
        >
          <Github className="h-4 w-4 text-muted-foreground" />
          <span className="text-meta font-medium text-muted-foreground">
            GitHub
          </span>
        </button>
        <button
          onClick={() => open(`${GITHUB_URL}/blob/main/CHANGELOG.md`)}
          className="flex flex-col items-center gap-2 rounded-xl border border-border/30 bg-card/50 p-4 transition-colors hover:bg-secondary/30"
        >
          <FileText className="h-4 w-4 text-muted-foreground" />
          <span className="text-meta font-medium text-muted-foreground">
            Changelog
          </span>
        </button>
        <button
          onClick={() => open(`${GITHUB_URL}/issues/new/choose`)}
          className="flex flex-col items-center gap-2 rounded-xl border border-border/30 bg-card/50 p-4 transition-colors hover:bg-secondary/30"
        >
          <AlertCircle className="h-4 w-4 text-muted-foreground" />
          <span className="text-meta font-medium text-muted-foreground">
            Report Issue
          </span>
        </button>
        <button
          onClick={() => open(`${GITHUB_URL}/wiki`)}
          className="flex flex-col items-center gap-2 rounded-xl border border-border/30 bg-card/50 p-4 transition-colors hover:bg-secondary/30"
        >
          <HelpCircle className="h-4 w-4 text-muted-foreground" />
          <span className="text-meta font-medium text-muted-foreground">
            Documentation
          </span>
        </button>
      </div>

      {/* Footer */}
      <div className="rounded-xl border border-border/30 bg-card/50 p-5">
        <p className="text-xs text-muted-foreground/60 leading-relaxed">
          "Qwen 纯 API 中文版：音频采集和会议历史保留在本地，语音识别、翻译和 AI 推理统一使用 Qwen。更新时请从 GitHub Releases 下载新的安装包。"
        </p>
      </div>
    </div>
  );
}
