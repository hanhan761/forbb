//! Lightweight query routing for interview-time assistance.
//!
//! The router deliberately uses deterministic rules instead of another model
//! call.  That keeps the hot path fast and makes the reason for each source
//! choice visible in the UI and the call log.

use serde::{Deserialize, Serialize};

/// Where a question should be answered from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryRoute {
    Auto,
    QuickAnswer,
    SearchFiles,
    SearchWeb,
    AskCodex,
}

impl QueryRoute {
    pub fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
            "quick_answer" | "quick" | "direct" => Self::QuickAnswer,
            "search_files" | "files" | "local_rag" | "rag" => Self::SearchFiles,
            "search_web" | "web" | "latest" => Self::SearchWeb,
            "ask_codex" | "codex" | "ask" => Self::AskCodex,
            _ => Self::Auto,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::QuickAnswer => "quick_answer",
            Self::SearchFiles => "search_files",
            Self::SearchWeb => "search_web",
            Self::AskCodex => "ask_codex",
        }
    }
}

/// A coarse question category used to explain routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    Personal,
    Project,
    Research,
    Course,
    Algorithm,
    Math,
    Professor,
    Latest,
    UnknownTerm,
    FollowUp,
    Unknown,
}

impl QuestionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Project => "project",
            Self::Research => "research",
            Self::Course => "course",
            Self::Algorithm => "algorithm",
            Self::Math => "math",
            Self::Professor => "professor",
            Self::Latest => "latest",
            Self::UnknownTerm => "unknown_term",
            Self::FollowUp => "follow_up",
            Self::Unknown => "unknown",
        }
    }
}

/// User-visible explanation attached to one generated answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub route: QueryRoute,
    pub question_type: QuestionType,
    pub answer_source: String,
    pub confidence: String,
}

impl RouteDecision {
    fn for_route(route: QueryRoute, question_type: QuestionType) -> Self {
        let (answer_source, confidence) = match route {
            QueryRoute::SearchFiles => ("local_rag", "high"),
            QueryRoute::SearchWeb => ("web_search", "web"),
            QueryRoute::QuickAnswer | QueryRoute::AskCodex => ("codex", "medium"),
            QueryRoute::Auto => ("codex", "medium"),
        };

        Self {
            route,
            question_type,
            answer_source: answer_source.to_string(),
            confidence: confidence.to_string(),
        }
    }
}

const PERSONAL_TERMS: &[&str] = &[
    "my resume", "my cv", "my experience", "my background", "my project",
    "my research", "my thesis", "my work", "my skills", "my internship",
    "我的简历", "我的经历", "我的背景", "我的项目", "我的研究", "我的论文",
    "项目经历", "个人经历", "实习经历", "工作经历", "个人背景",
];

const PROFESSOR_TERMS: &[&str] = &[
    "professor", "advisor", "supervisor", "pi ", "lab ", "导师", "教授", "课题组",
];

const LATEST_TERMS: &[&str] = &[
    "latest", "newest", "current", "recent", "today", "this year", "news", "price",
    "deadline", "最新", "最近", "当前", "现在", "新闻", "价格", "截止时间", "政策",
];

const RESEARCH_TERMS: &[&str] = &[
    "research", "paper", "thesis", "experiment", "dataset", "publication", "methodology",
    "研究", "论文", "实验", "数据集", "发表", "方法论", "科研",
];

const COURSE_TERMS: &[&str] = &[
    "course", "class", "exam", "operating system", "computer network", "database", "machine learning",
    "课程", "专业课", "考试", "操作系统", "计算机网络", "数据库", "机器学习", "深度学习",
];

const PROJECT_TERMS: &[&str] = &[
    "project", "repository", "repo", "codebase", "architecture", "implementation", "resume bullet",
    "项目", "仓库", "代码库", "架构", "实现", "方案", "简历项目",
];

const ALGORITHM_TERMS: &[&str] = &[
    "algorithm", "complexity", "big o", "dynamic programming", "binary search", "graph", "算法",
    "复杂度", "动态规划", "二分", "图论",
];

const MATH_TERMS: &[&str] = &[
    "equation", "derivative", "integral", "probability", "statistics", "matrix", "equation",
    "方程", "导数", "积分", "概率", "统计", "矩阵",
];

fn contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

/// Detect a likely unknown proper noun without sending another classifier
/// request. This intentionally stays conservative: a quoted phrase, acronym,
/// or capitalized multi-word name in a definition-style question is enough to
/// justify web grounding; ordinary lowercase concepts remain on the fast path.
fn looks_like_unknown_term(question: &str) -> bool {
    let text = question.trim();
    let lower = text.to_ascii_lowercase();
    let definition_style = [
        "what is ",
        "what are ",
        "what does ",
        "what do ",
        "who is ",
        "meaning of ",
        "define ",
        "是什么",
        "什么意思",
        "谁是",
    ]
    .iter()
    .any(|hint| lower.contains(hint));

    if !definition_style {
        return false;
    }

    if text.contains('"') || text.contains('\'') || text.contains('“') || text.contains('”') {
        return true;
    }

    let acronym_like = text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| word.len() >= 2)
        .any(|word| word.chars().all(|character| !character.is_ascii_lowercase()));
    if acronym_like {
        return true;
    }

    // Look for a capitalized word after the question lead-in. This catches
    // names such as “What is Amdahl's law?” while avoiding the initial “What”.
    let mut saw_definition_word = false;
    for word in text.split_whitespace() {
        let normalized = word.trim_matches(|character: char| !character.is_ascii_alphabetic());
        if !saw_definition_word {
            if ["is", "are", "does", "do", "of", "define", "who"]
                .iter()
                .any(|candidate| normalized.eq_ignore_ascii_case(candidate))
            {
                saw_definition_word = true;
            }
            continue;
        }
        if normalized.len() >= 3
            && normalized.chars().next().is_some_and(|character| character.is_ascii_uppercase())
        {
            return true;
        }
    }

    false
}

/// Classify the question using stable, low-latency lexical rules.
pub fn classify_question(mode: &str, question: &str) -> QuestionType {
    let text = question.trim().to_ascii_lowercase();
    if mode == "FollowUp" {
        return QuestionType::FollowUp;
    }
    if contains_any(&text, PERSONAL_TERMS) {
        return QuestionType::Personal;
    }
    if contains_any(&text, PROFESSOR_TERMS) {
        return QuestionType::Professor;
    }
    if contains_any(&text, LATEST_TERMS) {
        return QuestionType::Latest;
    }
    if contains_any(&text, RESEARCH_TERMS) {
        return QuestionType::Research;
    }
    if contains_any(&text, COURSE_TERMS) {
        return QuestionType::Course;
    }
    if contains_any(&text, PROJECT_TERMS) {
        return QuestionType::Project;
    }
    if contains_any(&text, ALGORITHM_TERMS) {
        return QuestionType::Algorithm;
    }
    if contains_any(&text, MATH_TERMS) {
        return QuestionType::Math;
    }
    if looks_like_unknown_term(question) {
        return QuestionType::UnknownTerm;
    }
    QuestionType::Unknown
}

/// Resolve the route. An explicit route always wins; `auto` uses the
/// question category and the current action mode.
pub fn resolve_route(override_route: Option<&str>, mode: &str, question: &str) -> RouteDecision {
    let question_type = classify_question(mode, question);
    let requested = QueryRoute::parse(override_route);

    if requested != QueryRoute::Auto {
        return RouteDecision::for_route(requested, question_type);
    }

    let route = match question_type {
        QuestionType::Personal | QuestionType::Project | QuestionType::Research | QuestionType::Course => {
            QueryRoute::SearchFiles
        }
        QuestionType::Professor | QuestionType::Latest | QuestionType::UnknownTerm => QueryRoute::SearchWeb,
        QuestionType::FollowUp => QueryRoute::QuickAnswer,
        QuestionType::Algorithm | QuestionType::Math => QueryRoute::QuickAnswer,
        QuestionType::Unknown if mode == "AskQuestion" => QueryRoute::AskCodex,
        QuestionType::Unknown => QueryRoute::QuickAnswer,
    };

    RouteDecision::for_route(route, question_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_personal_questions_to_files() {
        let decision = resolve_route(None, "Assist", "Tell me about my research project");
        assert_eq!(decision.route, QueryRoute::SearchFiles);
        assert_eq!(decision.question_type, QuestionType::Personal);
    }

    #[test]
    fn routes_latest_questions_to_web() {
        let decision = resolve_route(None, "AskQuestion", "What is the latest policy?");
        assert_eq!(decision.route, QueryRoute::SearchWeb);
        assert_eq!(decision.question_type, QuestionType::Latest);
    }

    #[test]
    fn routes_generic_questions_directly() {
        let decision = resolve_route(None, "Assist", "What is a hash map?");
        assert_eq!(decision.route, QueryRoute::QuickAnswer);
        assert_eq!(decision.question_type, QuestionType::Unknown);
    }

    #[test]
    fn explicit_route_wins() {
        let decision = resolve_route(Some("search_web"), "Assist", "Tell me about my resume");
        assert_eq!(decision.route, QueryRoute::SearchWeb);
        assert_eq!(decision.question_type, QuestionType::Personal);
    }

    #[test]
    fn routes_likely_unknown_proper_nouns_to_web() {
        let decision = resolve_route(None, "AskQuestion", "What is Amdahl's law?");
        assert_eq!(decision.route, QueryRoute::SearchWeb);
        assert_eq!(decision.question_type, QuestionType::UnknownTerm);
    }

    #[test]
    fn keeps_generic_lowercase_concepts_on_fast_path() {
        let decision = resolve_route(None, "Assist", "What is a hash map?");
        assert_eq!(decision.route, QueryRoute::QuickAnswer);
        assert_eq!(decision.question_type, QuestionType::Unknown);
    }
}
