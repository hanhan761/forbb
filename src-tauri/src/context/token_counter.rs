use serde::Serialize;

/// Approximate token count using chars/4 heuristic.
/// This is a simple but effective approximation for most LLM tokenizers.
pub fn count_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    // chars / 4 heuristic — roughly matches GPT-style tokenizers
    let char_count = text.chars().count();
    (char_count + 3) / 4 // round up
}

/// A single segment in the token budget breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct TokenBudgetSegment {
    pub label: String,
    pub tokens: usize,
    pub color: String,
    pub category: String,
}

/// The full token budget breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct TokenBudget {
    /// Estimated tokens sent in the active prompt (instructions + system
    /// overhead + transcript). Indexed resources are retrieved on demand and
    /// are not all sent in one request.
    pub total: usize,
    pub limit: u64,
    /// Total tokens stored in the local knowledge-base index.
    pub indexed_total: usize,
    pub segments: Vec<TokenBudgetSegment>,
}

/// Categorize a resource by its name/type for color assignment.
fn categorize_resource(name: &str, file_type: &str) -> (&'static str, &'static str, &'static str) {
    let name_lower = name.to_lowercase();

    if name_lower.contains("resume") || name_lower.contains("cv") {
        ("Resume", "#3b82f6", "resume")
    } else if name_lower.contains("jd")
        || name_lower.contains("job")
        || name_lower.contains("description")
        || name_lower.contains("posting")
    {
        ("JD", "#22c55e", "jd")
    } else {
        // Default to "Notes" category
        match file_type {
            "pdf" => ("Notes (PDF)", "#a855f7", "notes"),
            "md" => ("Notes (MD)", "#a855f7", "notes"),
            _ => ("Notes", "#a855f7", "notes"),
        }
    }
}

/// Represents a resource passed into budget computation.
pub struct BudgetResource {
    pub name: String,
    pub file_type: String,
    pub token_count: usize,
}

/// Compute the token budget breakdown.
///
/// - `resources`: the loaded context resources
/// - `custom_instructions`: the user's custom instructions text
/// - `transcript_tokens`: estimated tokens used by the current transcript
/// - `model_limit`: the model's context window size in tokens
pub fn compute_budget(
    resources: &[BudgetResource],
    custom_instructions: &str,
    transcript_tokens: usize,
    model_limit: u64,
) -> TokenBudget {
    let mut segments = Vec::new();
    let mut total: usize = 0;
    let mut indexed_total: usize = 0;

    // Resources are indexed locally and retrieved by relevance. They are not
    // all injected into a single prompt, so keep them visible as a separate
    // accounting bucket instead of making the active request look over limit.
    for res in resources {
        let (label, color, _category) = categorize_resource(&res.name, &res.file_type);
        segments.push(TokenBudgetSegment {
            label: label.to_string(),
            tokens: res.token_count,
            color: color.to_string(),
            category: "indexed".to_string(),
        });
        indexed_total += res.token_count;
    }

    // Custom instructions segment
    let instructions_tokens = count_tokens(custom_instructions);
    if instructions_tokens > 0 {
        segments.push(TokenBudgetSegment {
            label: "Custom Instructions".to_string(),
            tokens: instructions_tokens,
            color: "#6b7280".to_string(),
            category: "system".to_string(),
        });
        total += instructions_tokens;
    }

    // System prompt overhead (estimated)
    let system_prompt_tokens: usize = 200;
    segments.push(TokenBudgetSegment {
        label: "System Prompt".to_string(),
        tokens: system_prompt_tokens,
        color: "#6b7280".to_string(),
        category: "system".to_string(),
    });
    total += system_prompt_tokens;

    // Transcript segment
    if transcript_tokens > 0 {
        segments.push(TokenBudgetSegment {
            label: "Transcript".to_string(),
            tokens: transcript_tokens,
            color: "#f97316".to_string(),
            category: "transcript".to_string(),
        });
        total += transcript_tokens;
    }

    // Remaining headroom
    let limit = model_limit;
    if (limit as usize) > total {
        let headroom = (limit as usize) - total;
        segments.push(TokenBudgetSegment {
            label: "Available".to_string(),
            tokens: headroom,
            color: "#1f2937".to_string(),
            category: "headroom".to_string(),
        });
    }

    TokenBudget {
        total,
        limit,
        indexed_total,
        segments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_empty() {
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn test_count_tokens_basic() {
        // "hello" = 5 chars => (5+3)/4 = 2 tokens
        assert_eq!(count_tokens("hello"), 2);
    }

    #[test]
    fn test_count_tokens_longer() {
        // 100 chars => 25 tokens
        let text = "a".repeat(100);
        assert_eq!(count_tokens(&text), 25);
    }

    #[test]
    fn indexed_resources_do_not_consume_the_active_prompt_budget() {
        let resources = vec![BudgetResource {
            name: "large-notes.md".to_string(),
            file_type: "md".to_string(),
            token_count: 200_000,
        }];

        let budget = compute_budget(&resources, "", 0, 128_000);

        assert!(
            budget.total <= budget.limit as usize,
            "indexed library content must not make the active prompt exceed its limit"
        );
    }
}
