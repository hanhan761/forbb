import { useState, useCallback } from "react";
import { CloudUpload, FolderOpen, Loader2 } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { useContextStore } from "../stores/contextStore";
import { useConfigStore } from "../stores/configStore";
import { useRagStore } from "../stores/ragStore";
import { showToast } from "../stores/toastStore";

export function FileUpload() {
  const loadFile = useContextStore((s) => s.loadFile);
  const importVault = useContextStore((s) => s.importVault);
  const importFolder = useContextStore((s) => s.importFolder);
  const removeFiles = useContextStore((s) => s.removeFiles);
  const resources = useContextStore((s) => s.resources);
  const contextStrategy = useConfigStore((s) => s.contextStrategy);
  const autoIndexFile = useRagStore((s) => s.autoIndexFile);
  const rebuildIndex = useRagStore((s) => s.rebuildIndex);
  const [isDragOver, setIsDragOver] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [isImportingVault, setIsImportingVault] = useState(false);
  const [isImportingFolder, setIsImportingFolder] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const processFile = useCallback(
    async (filePath: string) => {
      setIsProcessing(true);
      setErrorMessage(null);
      try {
        const resource = await loadFile(filePath);
        // Auto-index if RAG is active — fire and forget, doesn't block UI
        if (contextStrategy === "local_rag") {
          autoIndexFile(resource.id);
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setErrorMessage(msg);
        setTimeout(() => setErrorMessage(null), 5000);
      } finally {
        setIsProcessing(false);
      }
    },
    [loadFile, contextStrategy, autoIndexFile]
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragOver(false);

      const files = e.dataTransfer?.files;
      if (!files || files.length === 0) return;

      for (let i = 0; i < files.length; i++) {
        const file = files[i];
        const filePath = (file as File & { path?: string }).path;
        if (filePath) {
          await processFile(filePath);
        }
      }
    },
    [processFile]
  );

  const handleBrowse = useCallback(async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: "Context Files",
            extensions: ["pdf", "txt", "md", "docx"],
          },
        ],
      });

      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        for (const filePath of paths) {
          if (filePath) {
            await processFile(filePath);
          }
        }
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setErrorMessage(msg);
      setTimeout(() => setErrorMessage(null), 5000);
    }
  }, [processFile]);

  const handleImportVault = useCallback(async () => {
    try {
      setIsImportingVault(true);
      setErrorMessage(null);
      const selected = await open({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) return;

      const result = await importVault(selected);
      if (result.imported.length === 0) {
        showToast("No usable Markdown notes were imported", "error");
        return;
      }

      let indexError: string | null = null;
      // Rebuild once for the whole batch instead of starting one indexing job
      // per note. This keeps the import responsive and avoids concurrent RAG jobs.
      if (contextStrategy === "local_rag") {
        await rebuildIndex();
        indexError = useRagStore.getState().error;
      }

      const skipped = result.skipped.length;
      if (indexError) {
        showToast(
          `Imported ${result.imported.length} note${result.imported.length === 1 ? "" : "s"}, but indexing failed: ${indexError}`,
          "info"
        );
        return;
      }

      showToast(
        `Imported ${result.imported.length} Obsidian note${result.imported.length === 1 ? "" : "s"}${skipped > 0 ? ` (${skipped} skipped)` : ""}`,
        skipped > 0 ? "info" : "success"
      );
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setErrorMessage(msg);
      showToast(msg, "error");
      setTimeout(() => setErrorMessage(null), 5000);
    } finally {
      setIsImportingVault(false);
    }
  }, [importVault, contextStrategy, rebuildIndex]);

  const handleImportFolder = useCallback(async (replaceExisting: boolean) => {
    try {
      setIsImportingFolder(true);
      setErrorMessage(null);
      const selected = await open({
        directory: true,
        multiple: false,
        title: replaceExisting ? "Replace Knowledge Base Folder" : "Choose Knowledge Base Folder",
      });
      if (!selected || Array.isArray(selected)) return;

      const previousResourceIds = resources.map((resource) => resource.id);
      if (replaceExisting && previousResourceIds.length > 0) {
        const confirmed = window.confirm(
          `用新文件夹替换当前知识库中的 ${previousResourceIds.length} 个文件？\n\n会先验证并导入新文件夹；原始文件不会被删除，只会清理旧的应用内副本和索引。`,
        );
        if (!confirmed) return;
      }

      const result = await importFolder(selected);
      if (result.imported.length === 0) {
        showToast("Selected folder contains no usable files", "error");
        return;
      }

      // Only remove the old set after the new folder has produced at least
      // one usable resource. This keeps a failed/empty replacement reversible.
      if (replaceExisting && previousResourceIds.length > 0) {
        await removeFiles(previousResourceIds);
      }

      let indexError: string | null = null;
      if (contextStrategy === "local_rag") {
        await rebuildIndex();
        indexError = useRagStore.getState().error;
      }

      if (indexError) {
        showToast(
          `Imported ${result.imported.length} file${result.imported.length === 1 ? "" : "s"}, but indexing failed: ${indexError}`,
          "info",
        );
      } else {
        const skipped = result.skipped.length;
        showToast(
          `${replaceExisting ? "Replaced" : "Imported"} ${result.imported.length} knowledge-base file${result.imported.length === 1 ? "" : "s"}${skipped > 0 ? ` (${skipped} skipped)` : ""}`,
          skipped > 0 ? "info" : "success",
        );
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setErrorMessage(msg);
      showToast(msg, "error");
      setTimeout(() => setErrorMessage(null), 5000);
    } finally {
      setIsImportingFolder(false);
    }
  }, [contextStrategy, importFolder, rebuildIndex, removeFiles, resources]);

  return (
    <div className="w-full">
      {/* Drop zone */}
      <div
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        onClick={handleBrowse}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            handleBrowse();
          }
        }}
        role="button"
        tabIndex={0}
        aria-label="Drop files here or click to browse"
        className={`relative flex flex-col items-center justify-center rounded-xl border-2 border-dashed transition-all duration-200 ${
          isDragOver
            ? "border-primary/60 bg-primary/5 scale-[1.01]"
            : "border-muted-foreground/20 bg-secondary/20 hover:border-muted-foreground/30 hover:bg-secondary/30"
        } px-6 py-6`}
      >
        {isProcessing ? (
          <>
            <Loader2 className="mb-2 h-8 w-8 animate-spin text-primary/60" />
            <p className="text-sm font-medium text-muted-foreground">
              Processing file...
            </p>
          </>
        ) : (
          <>
            <CloudUpload
              className={`mb-2 h-8 w-8 transition-colors duration-200 ${
                isDragOver ? "text-primary" : "text-primary/30"
              }`}
            />
            <p className="text-sm font-medium text-muted-foreground">
              {isDragOver ? "Drop to upload" : "Drag files here"}
            </p>
            <p className="mt-1 text-xs text-muted-foreground">
              PDF, TXT, Markdown, or DOCX
            </p>
            <button
              onClick={handleBrowse}
              className="mt-3 rounded-lg border border-primary/20 bg-primary/5 px-4 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/10 hover:text-primary"
            >
              Browse Files
            </button>
          </>
        )}
      </div>

      {/* Obsidian Vault import — reads local Markdown directly, no plugin required. */}
      <div className="mt-3 flex items-center justify-between gap-3 rounded-xl border border-border/30 bg-secondary/10 px-3 py-2.5">
        <div className="flex min-w-0 items-center gap-2">
          <FolderOpen className="h-4 w-4 shrink-0 text-primary/60" />
          <div className="min-w-0">
            <p className="text-xs font-medium text-foreground">Obsidian Vault</p>
            <p className="truncate text-meta text-muted-foreground">
              Import local Markdown notes into the knowledge base
            </p>
          </div>
        </div>
        <button
          type="button"
          onClick={handleImportVault}
          disabled={isProcessing || isImportingVault || isImportingFolder}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-lg border border-primary/20 bg-primary/5 px-3 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/10 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {isImportingVault && <Loader2 className="h-3 w-3 animate-spin" />}
          {isImportingVault ? "Importing..." : "Choose Vault"}
        </button>
      </div>

      {/* Generic knowledge-base folder management. Files are copied into the
          app-managed context store; the selected source folder is metadata. */}
      <div className="mt-3 rounded-xl border border-primary/20 bg-primary/5 px-3 py-2.5">
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 items-center gap-2">
            <FolderOpen className="h-4 w-4 shrink-0 text-primary/70" />
            <div className="min-w-0">
              <p className="text-xs font-medium text-foreground">Knowledge Base Folder</p>
              <p className="truncate text-meta text-muted-foreground">
                Import PDF, TXT, Markdown, and DOCX files recursively
              </p>
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-1.5">
            <button
              type="button"
              onClick={() => handleImportFolder(false)}
              disabled={isProcessing || isImportingVault || isImportingFolder}
              className="inline-flex items-center gap-1.5 rounded-lg border border-primary/20 bg-background/40 px-3 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/10 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {isImportingFolder && <Loader2 className="h-3 w-3 animate-spin" />}
              Choose Folder
            </button>
            <button
              type="button"
              onClick={() => handleImportFolder(true)}
              disabled={isProcessing || isImportingVault || isImportingFolder}
              className="rounded-lg border border-destructive/20 bg-destructive/5 px-3 py-1.5 text-xs font-medium text-destructive transition-colors hover:bg-destructive/10 disabled:cursor-not-allowed disabled:opacity-50"
            >
              Replace
            </button>
          </div>
        </div>
      </div>

      {/* Error toast */}
      {errorMessage && (
        <div className="mt-2 rounded-xl bg-destructive/10 px-3 py-2 text-xs text-destructive">
          {errorMessage}
        </div>
      )}
    </div>
  );
}
