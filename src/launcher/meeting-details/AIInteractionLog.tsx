import type { AIInteraction } from "../../lib/types";
import { formatRelativeTime, getModeLabel } from "../../lib/utils";
import { MessageSquare, ChevronDown, ChevronUp } from "lucide-react";

interface AIInteractionLogProps {
  interactions: AIInteraction[];
  expandedId: string | null;
  onToggle: (id: string) => void;
}

export function AIInteractionLog({ interactions, expandedId, onToggle }: AIInteractionLogProps) {
  if (interactions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16">
        <MessageSquare className="mb-3 h-8 w-8 text-primary/20" />
        <p className="text-sm font-medium text-muted-foreground/40">No AI interactions</p>
      </div>
    );
  }

  return (
    <div className="space-y-1.5 p-4">
      {interactions.map((interaction) => {
        const isExpanded = expandedId === interaction.id;
        return (
          <div key={interaction.id} className="rounded-xl border border-border/20 bg-card/30 border-l-2 border-l-primary/20">
            <button
              onClick={() => onToggle(interaction.id)}
              className="flex w-full items-center justify-between px-4 py-3 text-left cursor-pointer"
              aria-expanded={isExpanded}
            >
              <div className="flex items-center gap-2.5">
                <span className="rounded-lg bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
                  {getModeLabel(interaction.mode)}
                </span>
                <span className="truncate max-w-[250px] text-sm text-foreground/60">
                  {interaction.question_context}
                </span>
              </div>
              <div className="flex items-center gap-2.5">
                <span className="text-xs tabular-nums text-muted-foreground/50">{interaction.latency_ms}ms</span>
                {isExpanded
                  ? <ChevronUp className="h-4 w-4 text-muted-foreground/40" />
                  : <ChevronDown className="h-4 w-4 text-muted-foreground/40" />}
              </div>
            </button>
            {isExpanded && (
              <div className="border-t border-border/20 px-4 py-3">
                <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground/50">
                  <span>{interaction.provider}/{interaction.model}</span>
                  <span>&middot;</span>
                  <span>{formatRelativeTime(interaction.timestamp)}</span>
                  {interaction.ttft_ms != null && (
                    <>
                      <span>&middot;</span>
                      <span className="tabular-nums">TTFT {interaction.ttft_ms}ms</span>
                    </>
                  )}
                  {interaction.answer_source && (
                    <>
                      <span>&middot;</span>
                      <span className="text-info/70">{sourceLabel(interaction.answer_source)}</span>
                    </>
                  )}
                  {interaction.confidence && (
                    <span className="rounded bg-secondary/40 px-1.5 py-0.5 text-[10px]">
                      {interaction.confidence}
                    </span>
                  )}
                </div>
                <p className="whitespace-pre-wrap text-sm leading-relaxed text-foreground/80">
                  {interaction.response}
                </p>
                {interaction.sources && interaction.sources.length > 0 && (
                  <div className="mt-3 border-t border-border/15 pt-2 text-xs text-muted-foreground/60">
                    <span className="mr-1.5">Sources:</span>
                    {interaction.sources.map((source, index) => (
                      <a
                        key={`${source.url}-${index}`}
                        href={source.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="mr-2 inline-block max-w-[18rem] truncate align-bottom text-info/70 hover:text-info"
                        title={source.url}
                      >
                        {source.title}
                      </a>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

function sourceLabel(source: string): string {
  switch (source) {
    case "local_rag": return "My files";
    case "hot_context": return "Hot context";
    case "web_search": return "Web";
    case "not_found": return "No matching files";
    default: return "Codex";
  }
}
