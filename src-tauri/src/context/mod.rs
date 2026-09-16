pub mod file_loader;
pub mod pdf_extractor;
pub mod resource_cache;
pub mod token_counter;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use uuid::Uuid;

use resource_cache::{CachedResource, ResourceCache};
use token_counter::{count_tokens, BudgetResource, TokenBudget};

/// A loaded context resource (mirrors the TS ContextResource type).
#[derive(Debug, Clone, Serialize)]
pub struct ContextResource {
    pub id: String,
    pub name: String,
    pub file_type: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub token_count: usize,
    pub preview: String,
    pub loaded_at: String,
    /// Canonical folder selected by the user when this resource was imported.
    /// The folder is metadata only; deletion never removes this source path.
    pub source_folder: String,
}

/// Loads, caches, and serves context text from files.
/// Files stored in %APPDATA%/com.nexq.app/context/
pub struct ContextManager {
    context_dir: PathBuf,
    resources: Vec<ContextResource>,
    cache: ResourceCache,
    custom_instructions: String,
}

impl ContextManager {
    pub fn new() -> Self {
        // Determine the context directory: %APPDATA%/com.nexq.app/context/
        let context_dir = Self::get_context_dir();

        // Ensure the directory exists
        if let Err(e) = fs::create_dir_all(&context_dir) {
            log::warn!("Failed to create context directory: {}", e);
        }

        Self {
            context_dir,
            resources: Vec::new(),
            cache: ResourceCache::new(),
            custom_instructions: String::new(),
        }
    }

    /// Get the context storage directory path.
    fn get_context_dir() -> PathBuf {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            PathBuf::from(appdata)
                .join("com.nexq.app")
                .join("context")
        } else {
            // Fallback for non-Windows or if APPDATA is not set
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join(".nexq")
                .join("context")
        }
    }

    /// Load a file into the context manager.
    /// Copies the file to the context directory, extracts text, and caches it.
    pub fn load_file(&mut self, file_path: &str) -> Result<ContextResource, String> {
        self.load_file_with_source_folder(file_path, None)
    }

    /// Load a file and associate it with an import folder. When no folder is
    /// supplied, the file's parent directory is used for single-file imports.
    pub fn load_file_with_source_folder(
        &mut self,
        file_path: &str,
        source_folder: Option<&str>,
    ) -> Result<ContextResource, String> {
        let source_path = Path::new(file_path);

        if !source_path.exists() {
            return Err(format!("File not found: {}", file_path));
        }

        // Determine file type from extension
        let extension = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let file_type = match extension.as_str() {
            "pdf" => "pdf",
            "txt" => "txt",
            "md" => "md",
            "docx" => "docx",
            _ => {
                return Err(format!(
                    "Unsupported file type: .{}. Supported: .pdf, .txt, .md, .docx",
                    extension
                ))
            }
        };

        // Generate a unique resource ID
        let resource_id = Uuid::new_v4().to_string();

        // Get original file name
        let file_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Copy file to context directory with unique name to avoid collisions
        fs::create_dir_all(&self.context_dir)
            .map_err(|e| format!("Failed to create context directory: {}", e))?;
        let dest_filename = format!("{}_{}", resource_id, file_name);
        let dest_path = self.context_dir.join(&dest_filename);

        fs::copy(source_path, &dest_path)
            .map_err(|e| format!("Failed to copy file to context directory: {}", e))?;

        // Get file size
        let size_bytes = fs::metadata(&dest_path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Extract text based on file type — clean up copied file on failure
        let text = match file_type {
            "pdf" => pdf_extractor::extract_text_from_pdf(
                dest_path.to_str().unwrap_or(file_path),
            ),
            "txt" | "md" => file_loader::load_text_file(
                dest_path.to_str().unwrap_or(file_path),
            ),
            "docx" => crate::rag::file_processor::extract_docx_text(
                dest_path.to_str().unwrap_or(file_path),
            ),
            _ => Ok(String::new()),
        }
        .map_err(|e| {
            let _ = fs::remove_file(&dest_path);
            e
        })?;

        // Count tokens
        let token_count = count_tokens(&text);

        // Create preview (first 100 chars)
        let preview = if text.len() > 100 {
            let boundary = text
                .char_indices()
                .nth(100)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
            format!("{}...", &text[..boundary])
        } else {
            text.clone()
        };

        // Timestamp
        let loaded_at = chrono::Utc::now().to_rfc3339();
        let source_folder = source_folder
            .map(PathBuf::from)
            .or_else(|| source_path.parent().map(Path::to_path_buf))
            .map(|path| Self::normalize_source_folder(&path))
            .unwrap_or_default();

        // Create resource
        let resource = ContextResource {
            id: resource_id.clone(),
            name: file_name,
            file_type: file_type.to_string(),
            file_path: dest_path
                .to_str()
                .unwrap_or("")
                .to_string(),
            size_bytes,
            token_count,
            preview,
            loaded_at: loaded_at.clone(),
            source_folder,
        };

        // Cache the extracted text
        self.cache.insert(
            resource_id,
            CachedResource {
                text,
                token_count,
                loaded_at,
            },
        );

        // Store resource metadata
        self.resources.push(resource.clone());

        log::info!(
            "Loaded context file: {} ({} tokens)",
            resource.name,
            resource.token_count
        );

        Ok(resource)
    }

    fn normalize_source_folder(path: &Path) -> String {
        fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .into_owned()
    }

    /// Restore a previously-persisted resource from stored metadata.
    ///
    /// Re-extracts the text from the copied file in the context directory so
    /// the in-memory cache is rebuilt after a restart. Skips silently if the
    /// file is no longer on disk (returns Err so the caller can clean up DB).
    pub fn restore_resource(&mut self, resource: ContextResource) -> Result<(), String> {
        let path = Path::new(&resource.file_path);
        if !path.exists() {
            return Err(format!("Context file missing from disk: {}", resource.file_path));
        }

        // Re-extract text (needed for context-stuffing mode and token budget)
        let text = match resource.file_type.as_str() {
            "pdf" => pdf_extractor::extract_text_from_pdf(&resource.file_path)?,
            "txt" | "md" => file_loader::load_text_file(&resource.file_path)?,
            "docx" => crate::rag::file_processor::extract_docx_text(&resource.file_path)?,
            _ => String::new(),
        };

        let token_count = count_tokens(&text);

        self.cache.insert(
            resource.id.clone(),
            CachedResource {
                text,
                token_count,
                loaded_at: resource.loaded_at.clone(),
            },
        );

        // Guard against duplicate restore (idempotent)
        if !self.resources.iter().any(|r| r.id == resource.id) {
            self.resources.push(resource);
        }

        Ok(())
    }

    /// Remove a context resource by its ID.
    pub fn remove_file(&mut self, resource_id: &str) -> Result<(), String> {
        // Find the resource
        let idx = self
            .resources
            .iter()
            .position(|r| r.id == resource_id)
            .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

        let resource = self.resources.remove(idx);

        // Remove the cached file from context directory
        let file_path = Path::new(&resource.file_path);
        if file_path.exists() {
            if let Err(e) = fs::remove_file(file_path) {
                log::warn!("Failed to remove context file: {}", e);
            }
        }

        // Remove from cache
        self.cache.remove(resource_id);

        log::info!("Removed context resource: {}", resource.name);

        Ok(())
    }

    /// Remove several resources atomically from the in-memory list after
    /// validating that every requested id exists. Files are app-managed
    /// copies, so this never touches `source_folder`.
    pub fn remove_files(&mut self, resource_ids: &[String]) -> Result<Vec<ContextResource>, String> {
        let mut seen = HashSet::new();
        let unique_ids: Vec<String> = resource_ids
            .iter()
            .filter_map(|resource_id| {
                if seen.insert(resource_id.clone()) {
                    Some(resource_id.clone())
                } else {
                    None
                }
            })
            .collect();

        let mut resources = Vec::with_capacity(unique_ids.len());
        for resource_id in &unique_ids {
            let resource = self
                .resources
                .iter()
                .find(|resource| resource.id == *resource_id)
                .cloned()
                .ok_or_else(|| format!("Resource not found: {}", resource_id))?;
            resources.push(resource);
        }

        for resource_id in &unique_ids {
            self.remove_file(resource_id)?;
        }

        Ok(resources)
    }

    /// Return the IDs belonging to a source folder without mutating state.
    /// Commands use this snapshot before deleting database rows.
    pub fn resource_ids_for_folder(&self, source_folder: &str) -> Vec<String> {
        let normalized = Self::normalize_source_folder(Path::new(source_folder));
        self.resources
            .iter()
            .filter(|resource| resource.source_folder == normalized)
            .map(|resource| resource.id.clone())
            .collect()
    }

    /// Remove all resources imported from a source folder.
    pub fn remove_folder(&mut self, source_folder: &str) -> Result<Vec<ContextResource>, String> {
        let ids = self.resource_ids_for_folder(source_folder);
        self.remove_files(&ids)
    }

    /// Remove the app-managed storage directory when no copied resources
    /// remain. This is intentionally separate from source-folder metadata so
    /// a source directory selected by the user is never removed.
    pub fn cleanup_empty_storage_dir(&self) {
        if !self.resources.is_empty() {
            return;
        }

        if let Err(error) = fs::remove_dir(&self.context_dir) {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::debug!("Context storage directory was not removed: {}", error);
            }
        }
    }

    /// Remove every loaded resource and delete the now-empty app-managed
    /// storage directory. It is recreated on the next import.
    pub fn remove_all_files(&mut self) -> Result<Vec<ContextResource>, String> {
        let ids: Vec<String> = self.resources.iter().map(|resource| resource.id.clone()).collect();
        let removed = self.remove_files(&ids)?;
        self.cleanup_empty_storage_dir();
        Ok(removed)
    }

    /// List all loaded context resources.
    pub fn list_resources(&self) -> Vec<ContextResource> {
        self.resources.clone()
    }

    /// Get the assembled context text (concatenation of all resources' text).
    pub fn get_assembled_context(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Add custom instructions first if present
        if !self.custom_instructions.is_empty() {
            parts.push(format!(
                "## Custom Instructions\n{}\n",
                self.custom_instructions
            ));
        }

        // Add each resource's text
        for resource in &self.resources {
            if let Some(cached) = self.cache.get(&resource.id) {
                if !cached.text.is_empty() {
                    parts.push(format!(
                        "## {} ({})\n{}\n",
                        resource.name, resource.file_type, cached.text
                    ));
                }
            }
        }

        parts.join("\n")
    }

    /// Return a bounded, always-available context slice for live interview
    /// turns. Files whose names suggest a CV/profile/project/research summary
    /// are preferred; if none are named that way, the first loaded resource is
    /// used as a conservative fallback. Full papers and notes remain in the
    /// cold RAG index instead of bloating every prompt.
    pub fn get_hot_context(&self, max_chars: usize, include_custom_instructions: bool) -> String {
        if max_chars == 0 {
            return String::new();
        }

        const HOT_TERMS: &[&str] = &[
            "cv", "resume", "profile", "bio", "background", "motivation", "education",
            "experience", "project", "research", "introduction", "professor", "advisor", "lab", "laboratory", "教授", "导师", "简历",
            "背景", "经历", "动机", "教育", "项目", "研究", "个人介绍",
        ];

        let is_hot = |resource: &&ContextResource| {
            let name = resource.name.to_ascii_lowercase();
            HOT_TERMS.iter().any(|term| name.contains(term))
        };

        let mut selected: Vec<&ContextResource> = self.resources.iter().filter(is_hot).collect();
        if selected.is_empty() {
            if let Some(first) = self.resources.first() {
                selected.push(first);
            }
        }

        let mut sections = Vec::new();
        if include_custom_instructions && !self.custom_instructions.is_empty() {
            sections.push(format!(
                "## Interview Instructions\n{}",
                self.custom_instructions
            ));
        }

        let mut used_chars = sections.iter().map(|section| section.chars().count()).sum::<usize>();
        for resource in selected {
            let Some(cached) = self.cache.get(&resource.id) else {
                continue;
            };
            if cached.text.is_empty() || used_chars >= max_chars {
                continue;
            }

            let header = format!("## {} ({})\n", resource.name, resource.file_type);
            let remaining = max_chars.saturating_sub(used_chars + header.chars().count());
            if remaining == 0 {
                break;
            }
            let body: String = cached.text.chars().take(remaining).collect();
            sections.push(format!("{}{}", header, body));
            used_chars += header.chars().count() + body.chars().count();
        }

        sections.join("\n\n")
    }

    /// Set custom instructions text.
    pub fn set_custom_instructions(&mut self, text: &str) {
        self.custom_instructions = text.to_string();
    }

    /// Get the custom instructions text.
    pub fn get_custom_instructions(&self) -> &str {
        &self.custom_instructions
    }

    /// Compute the token budget breakdown.
    pub fn get_token_budget(
        &self,
        model_context_window: u64,
        transcript_tokens: usize,
    ) -> TokenBudget {
        let budget_resources: Vec<BudgetResource> = self
            .resources
            .iter()
            .map(|r| BudgetResource {
                name: r.name.clone(),
                file_type: r.file_type.clone(),
                token_count: r.token_count,
            })
            .collect();

        token_counter::compute_budget(
            &budget_resources,
            &self.custom_instructions,
            transcript_tokens,
            model_context_window,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ContextManager, ResourceCache};
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn imports_and_removes_resources_by_source_folder_without_deleting_source() {
        let root = std::env::temp_dir().join(format!("nexq-context-test-{}", Uuid::new_v4()));
        let source = root.join("knowledge-base");
        let managed = root.join("managed-context");
        fs::create_dir_all(&source).expect("create source folder");
        fs::create_dir_all(&managed).expect("create managed folder");
        let source_file = source.join("notes.txt");
        fs::write(&source_file, "A question about the project.").expect("write source file");

        let mut manager = ContextManager {
            context_dir: managed.clone(),
            resources: Vec::new(),
            cache: ResourceCache::new(),
            custom_instructions: String::new(),
        };

        let resource = manager
            .load_file_with_source_folder(
                source_file.to_str().expect("source path"),
                Some(source.to_str().expect("folder path")),
            )
            .expect("import source file");

        assert_eq!(
            resource.source_folder,
            fs::canonicalize(&source)
                .expect("canonical source folder")
                .to_string_lossy()
                .to_string()
        );
        assert!(source_file.exists(), "import must not delete source file");
        assert_eq!(manager.remove_folder(source.to_str().expect("folder path")).unwrap().len(), 1);
        assert!(manager.list_resources().is_empty());
        assert!(source.exists(), "folder removal must not delete source folder");

        manager.cleanup_empty_storage_dir();
        let _ = fs::remove_dir_all(root);
    }
}
