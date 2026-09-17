import { useContextStore } from "../stores/contextStore";
import { AlertTriangle, Info } from "lucide-react";

export function TokenBudget() {
  const tokenBudget = useContextStore((s) => s.tokenBudget);

  if (!tokenBudget || tokenBudget.limit === 0) {
    return null;
  }

  const usedTokens = tokenBudget.total;
  const limit = tokenBudget.limit;
  const usagePercent = Math.min((usedTokens / limit) * 100, 100);
  const isWarning = usagePercent > 80;
  const isCritical = usagePercent > 95;

  // Indexed files are searchable library content, not content sent wholesale
  // in the current request. Keep them out of the active prompt meter so a
  // large RAG library does not produce a false 100% warning.
  const indexedSegments = tokenBudget.segments.filter(
    (s) =>
      s.category === "indexed" ||
      s.category === "resume" ||
      s.category === "jd" ||
      s.category === "notes"
  );
  const visibleSegments = tokenBudget.segments.filter(
    (s) => s.category !== "headroom" && !indexedSegments.includes(s) && s.tokens > 0
  );
  const indexedTokens =
    tokenBudget.indexed_total ??
    indexedSegments.reduce((sum, segment) => sum + segment.tokens, 0);
  const overBy = Math.max(usedTokens - limit, 0);

  const borderClass = isCritical
    ? "border-destructive/60"
    : isWarning
      ? "border-warning/60"
      : "border-border/50";

  return (
    <div
      className={`w-full rounded-xl border ${borderClass} bg-secondary/30 p-4`}
    >
      {/* Header row */}
      <div className="mb-2.5 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xs font-semibold text-muted-foreground">
            Prompt Token Budget
          </span>
          {(isWarning || isCritical) && (
            <AlertTriangle
              className={`h-3.5 w-3.5 ${
                isCritical ? "text-destructive" : "text-warning"
              }`}
            />
          )}
        </div>
        <span className="text-xs tabular-nums text-muted-foreground">
          {formatNumber(usedTokens)} / {formatNumber(limit)} tokens used (
          {usagePercent.toFixed(0)}%
          {overBy > 0 ? ` · over by ${formatNumber(overBy)}` : ""})
        </span>
      </div>

      {/* Stacked bar */}
      <div
        className="h-2.5 w-full overflow-hidden rounded-full bg-muted/40"
        role="meter"
        aria-label="Active prompt token budget usage"
        aria-valuenow={usedTokens}
        aria-valuemin={0}
        aria-valuemax={limit}
      >
        <div className="flex h-full">
          {visibleSegments.map((segment, i) => {
            const widthPercent = (segment.tokens / limit) * 100;
            if (widthPercent < 0.1) return null;
            return (
              <div
                key={`${segment.category}-${i}`}
                className="h-full transition-all duration-300"
                style={{
                  width: `${widthPercent}%`,
                  backgroundColor: segment.color,
                  minWidth: widthPercent > 0 ? "2px" : "0",
                }}
                title={`${segment.label}: ~${formatNumber(segment.tokens)} tokens`}
              />
            );
          })}
        </div>
      </div>

      {/* Legend */}
      {visibleSegments.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-3">
          {visibleSegments.map((segment, i) => (
            <div
              key={`legend-${segment.category}-${i}`}
              className="flex items-center gap-1.5"
            >
              <div
                className="h-2 w-2 rounded-full"
                style={{ backgroundColor: segment.color }}
              />
              <span className="text-meta tabular-nums text-muted-foreground">
                {segment.label}: ~{formatNumber(segment.tokens)}
              </span>
            </div>
          ))}
        </div>
      )}

      {indexedSegments.length > 0 && (
        <div className="mt-3 rounded-lg border border-info/20 bg-info/5 px-3 py-2.5">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-1.5">
              <Info className="h-3.5 w-3.5 text-info" />
              <span className="text-xs font-medium text-info/90">
                Indexed knowledge base
              </span>
            </div>
            <span className="text-meta tabular-nums text-muted-foreground">
              ~{formatNumber(indexedTokens)} tokens
            </span>
          </div>
          <p className="mt-1 text-meta leading-relaxed text-muted-foreground">
            RAG retrieves relevant excerpts for each question; the entire library is not sent at once.
          </p>
          <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1">
            {indexedSegments.map((segment, i) => (
              <span
                key={`indexed-${segment.category}-${i}`}
                className="text-meta tabular-nums text-muted-foreground"
              >
                {segment.label}: ~{formatNumber(segment.tokens)}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) {
    return `${(n / 1_000_000).toFixed(1)}M`;
  }
  if (n >= 1_000) {
    return `${(n / 1_000).toFixed(1)}k`;
  }
  return n.toString();
}
