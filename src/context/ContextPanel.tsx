import { useEffect } from "react";
import { useContextStore } from "../stores/contextStore";
import { useConfigStore } from "../stores/configStore";
import { FileUpload } from "./FileUpload";
import { ContextResourceList } from "./ContextResourceList";
import { TokenBudget } from "./TokenBudget";
import { RagIndexBar } from "./RagIndexBar";
import { FileText } from "lucide-react";

export function ContextPanel() {
  const loadResources = useContextStore((state) => state.loadResources);
  const refreshTokenBudget = useContextStore((state) => state.refreshTokenBudget);
  const contextStrategy = useConfigStore((state) => state.contextStrategy);

  useEffect(() => {
    loadResources();
    refreshTokenBudget();
  }, [loadResources, refreshTokenBudget]);

  return (
    <div className="flex h-full flex-col gap-5 overflow-y-auto p-5">
      <div className="flex items-center gap-2">
        <FileText className="h-4 w-4 text-primary" />
        <h2 className="text-sm font-semibold text-foreground">Meeting Context</h2>
        {contextStrategy === "local_rag" && (
          <div className="flex items-center gap-1.5 rounded-full bg-primary/10 px-2.5 py-1 text-meta font-medium text-primary">
            <span>Smart Search Active</span>
          </div>
        )}
      </div>

      <TokenBudget />
      {contextStrategy === "local_rag" && <RagIndexBar />}
      <FileUpload />
      <ContextResourceList />
    </div>
  );
}
