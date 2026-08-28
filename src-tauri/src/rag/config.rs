use serde::{Deserialize, Serialize};

/// RAG pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Whether the RAG pipeline is enabled.
    pub enabled: bool,
    /// Ollama embedding model name.
    pub embedding_model: String,
    /// Ollama server base URL.
    pub ollama_url: String,
    /// Number of texts to embed per batch request.
    pub batch_size: usize,
    /// Target chunk size in tokens.
    pub chunk_size: usize,
    /// Overlap between consecutive chunks in tokens.
    pub chunk_overlap: usize,
    /// Text splitting strategy ("recursive", etc.).
    pub splitting_strategy: String,
    /// Number of top results to return from search.
    pub top_k: usize,
    /// Search mode: "hybrid", "semantic", or "keyword".
    pub search_mode: String,
    /// Minimum similarity score to include in results.
    pub similarity_threshold: f32,
    /// Weight for semantic search in hybrid mode (0.0–1.0).
    pub semantic_weight: f32,
    /// Whether to index live transcript segments.
    pub include_transcript: bool,
    /// Dimensionality of the embedding vectors.
    pub embedding_dimensions: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            embedding_model: "nomic-embed-text".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            batch_size: 32,
            chunk_size: 512,
            chunk_overlap: 64,
            splitting_strategy: "recursive".to_string(),
            top_k: 5,
            // Keyword-only search keeps the knowledge base usable without
            // requiring Ollama or another local embedding service.
            search_mode: "keyword".to_string(),
            similarity_threshold: 0.3,
            semantic_weight: 0.7,
            include_transcript: false,
            embedding_dimensions: 768,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RagConfig;

    #[test]
    fn default_search_is_available_without_ollama() {
        let config = RagConfig::default();

        assert!(config.enabled);
        assert_eq!(config.search_mode, "keyword");
        assert!(!config.include_transcript);
    }
}
