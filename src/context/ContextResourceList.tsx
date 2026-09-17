import { useCallback, useEffect, useMemo, useState } from "react";
import { FileText, FolderOpen, ListChecks, Trash2 } from "lucide-react";
import { useContextStore } from "../stores/contextStore";
import { showToast } from "../stores/toastStore";
import { ResourceCard } from "./ResourceCard";

/**
 * Shared knowledge-base resource management surface.
 *
 * Both the dashboard and the settings context panel use this component so
 * batch/folder deletion cannot be hidden behind an unused secondary view.
 */
export function ContextResourceList() {
  const resources = useContextStore((state) => state.resources);
  const removeFile = useContextStore((state) => state.removeFile);
  const removeFiles = useContextStore((state) => state.removeFiles);
  const removeFolder = useContextStore((state) => state.removeFolder);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  const resourceGroups = useMemo(() => {
    const groups = new Map<string, typeof resources>();
    for (const resource of resources) {
      const folder = resource.source_folder?.trim() || "__individual__";
      const group = groups.get(folder) ?? [];
      group.push(resource);
      groups.set(folder, group);
    }
    return [...groups.entries()];
  }, [resources]);

  // Drop selections for resources removed by another action/window.
  useEffect(() => {
    setSelectedIds((current) => {
      const validIds = new Set(resources.map((resource) => resource.id));
      const next = new Set([...current].filter((id) => validIds.has(id)));
      return next.size === current.size ? current : next;
    });
  }, [resources]);

  const handleRemoveFile = useCallback(async (id: string) => {
    try {
      await removeFile(id);
      setSelectedIds((current) => {
        const next = new Set(current);
        next.delete(id);
        return next;
      });
    } catch (error) {
      showToast(error instanceof Error ? error.message : "删除文件失败", "error");
    }
  }, [removeFile]);

  const handleRemoveSelected = useCallback(async () => {
    const ids = [...selectedIds];
    if (ids.length === 0) return;
    if (!window.confirm(`删除选中的 ${ids.length} 个知识库文件？\n\n将同时删除应用副本和索引，原始文件不会被删除。`)) return;

    try {
      const removed = await removeFiles(ids);
      setSelectedIds(new Set());
      showToast(`已删除 ${removed} 个文件`, "success");
    } catch (error) {
      showToast(error instanceof Error ? error.message : "批量删除失败", "error");
    }
  }, [removeFiles, selectedIds]);

  const handleRemoveFolder = useCallback(async (sourceFolder: string, count: number) => {
    if (!window.confirm(`删除文件夹中的 ${count} 个知识库文件？\n\n仅删除应用副本和索引，不会删除原始文件夹。`)) return;

    try {
      const removed = await removeFolder(sourceFolder);
      setSelectedIds((current) => {
        const next = new Set(current);
        for (const resource of resources) {
          if (resource.source_folder === sourceFolder) next.delete(resource.id);
        }
        return next;
      });
      showToast(`已删除文件夹中的 ${removed} 个文件`, "success");
    } catch (error) {
      showToast(error instanceof Error ? error.message : "删除文件夹失败", "error");
    }
  }, [removeFolder, resources]);

  if (resources.length === 0) return null;

  const allSelected =
    selectedIds.size === resources.length &&
    resources.every((resource) => selectedIds.has(resource.id));

  return (
    <section aria-label="知识库文件管理">
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <FileText className="h-3 w-3 text-muted-foreground/60" />
        <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
          知识库文件 ({resources.length})
        </span>
        <div className="ml-auto flex flex-wrap items-center gap-1.5">
          <button
            type="button"
            onClick={() => setSelectedIds(allSelected ? new Set() : new Set(resources.map((resource) => resource.id)))}
            className="inline-flex items-center gap-1 rounded-md border border-border/40 bg-secondary/20 px-2 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-secondary/50 hover:text-foreground"
            aria-label={allSelected ? "取消全选知识库文件" : "全选知识库文件"}
          >
            {allSelected ? "取消全选" : "全选"}
          </button>
          <button
            type="button"
            onClick={handleRemoveSelected}
            disabled={selectedIds.size === 0}
            className="inline-flex items-center gap-1 rounded-md border border-destructive/25 bg-destructive/10 px-2 py-1 text-xs font-semibold text-destructive transition-colors hover:bg-destructive/20 disabled:cursor-not-allowed disabled:border-border/30 disabled:bg-secondary/20 disabled:text-muted-foreground/50"
            aria-label="批量删除知识库文件"
            title={selectedIds.size === 0 ? "请先勾选要删除的文件" : `删除已选的 ${selectedIds.size} 个文件`}
          >
            <ListChecks className="h-3.5 w-3.5" />
            批量删除{selectedIds.size > 0 ? ` (${selectedIds.size})` : ""}
          </button>
        </div>
      </div>

      <div className="mb-2 flex items-center gap-2 rounded-lg border border-primary/15 bg-primary/5 px-2.5 py-1.5 text-xs text-muted-foreground">
        <ListChecks className="h-3.5 w-3.5 shrink-0 text-primary/70" />
        <span>
          {selectedIds.size > 0
            ? `已选择 ${selectedIds.size} 个文件，点击右上角“批量删除”`
            : "勾选文件左侧复选框后，可在右上角批量删除"}
        </span>
      </div>

      <div className="space-y-2">
        {resourceGroups.map(([folder, group]) => (
          <div key={folder} className="flex flex-col gap-2">
            <div className="flex items-center gap-2 rounded-lg border border-border/20 bg-secondary/10 px-2.5 py-1.5">
              <FolderOpen className="h-3.5 w-3.5 shrink-0 text-primary/60" />
              <span
                className="min-w-0 flex-1 truncate text-meta font-medium text-muted-foreground"
                title={folder === "__individual__" ? "Individual file imports" : folder}
              >
                {folder === "__individual__" ? "单独导入的文件" : folderName(folder)}
              </span>
              {folder !== "__individual__" && (
                <button
                  type="button"
                  onClick={() => handleRemoveFolder(folder, group.length)}
                  className="inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-1 text-meta text-muted-foreground/60 transition-colors hover:bg-destructive/10 hover:text-destructive"
                  title="删除此文件夹导入的全部文件"
                >
                  <Trash2 className="h-3 w-3" />
                  删除此文件夹
                </button>
              )}
            </div>
            {group.map((resource) => (
              <ResourceCard
                key={resource.id}
                resource={resource}
                selected={selectedIds.has(resource.id)}
                onToggleSelect={() => setSelectedIds((current) => {
                  const next = new Set(current);
                  if (next.has(resource.id)) next.delete(resource.id);
                  else next.add(resource.id);
                  return next;
                })}
                onRemove={handleRemoveFile}
              />
            ))}
          </div>
        ))}
      </div>
    </section>
  );
}

function folderName(path: string): string {
  const normalized = path.replace(/[\\/]+$/g, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || path;
}
