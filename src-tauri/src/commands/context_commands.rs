use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{command, State};

use crate::context::ContextResource;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
struct ObsidianSkippedFile {
    path: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct ObsidianVaultImportResult {
    vault_path: String,
    imported: Vec<ContextResource>,
    skipped: Vec<ObsidianSkippedFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ObsidianReviewWriteResult {
    path: String,
}

fn safe_path_component(value: &str, fallback: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.chars().take(80).collect()
    }
}

fn profile_folder_name(profile: Option<&str>) -> String {
    let first_line = profile
        .unwrap_or("")
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    safe_path_component(first_line, "Unassigned Profile")
}

/// Collect Markdown notes from an Obsidian Vault without traversing hidden
/// configuration directories such as `.obsidian` or `.trash`.
fn collect_markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_markdown_files_recursive(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_markdown_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read Obsidian Vault '{}': {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to inspect Vault entry: {}", e))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect '{}': {}", path.display(), e))?;

        if file_type.is_dir() {
            let is_hidden = entry.file_name().to_string_lossy().starts_with('.');
            if !is_hidden {
                collect_markdown_files_recursive(&path, files)?;
            }
        } else if file_type.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            files.push(path);
        }
    }

    Ok(())
}

#[command]
pub async fn import_obsidian_vault(
    vault_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let vault = PathBuf::from(&vault_path);
    if !vault.is_dir() {
        return Err(format!("Obsidian Vault folder not found: {}", vault_path));
    }

    let files = collect_markdown_files(&vault)?;
    if files.is_empty() {
        return Err("No Markdown notes found in the selected Obsidian Vault".to_string());
    }

    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    {
        let mut ctx = ctx_mgr
            .lock()
            .map_err(|e| format!("Failed to lock context manager: {}", e))?;

        for path in files {
            let path_string = path.to_string_lossy().into_owned();
            match ctx.load_file(&path_string) {
                Ok(resource) => imported.push(resource),
                Err(reason) => skipped.push(ObsidianSkippedFile {
                    path: path_string,
                    reason,
                }),
            }
        }
    }

    // Persist the imported copies using the same database shape as single-file
    // imports, so they survive restart and remain searchable by the existing RAG.
    if let Some(db_arc) = state.database.as_ref() {
        if let Ok(db_guard) = db_arc.lock() {
            for resource in &imported {
                let db_resource = crate::db::context::ContextResource {
                    id: resource.id.clone(),
                    name: resource.name.clone(),
                    file_type: resource.file_type.clone(),
                    file_path: resource.file_path.clone(),
                    size_bytes: resource.size_bytes as i64,
                    token_count: resource.token_count as i64,
                    preview: resource.preview.clone(),
                    loaded_at: resource.loaded_at.clone(),
                };
                if let Err(e) =
                    crate::db::context::add_context_resource(db_guard.connection(), &db_resource)
                {
                    log::warn!(
                        "Failed to persist imported Obsidian note '{}': {}",
                        resource.name,
                        e
                    );
                }
            }
        }
    }

    log::info!(
        "Imported {} Markdown note(s) from Obsidian Vault '{}' ({} skipped)",
        imported.len(),
        vault_path,
        skipped.len()
    );

    serde_json::to_string(&ObsidianVaultImportResult {
        vault_path,
        imported,
        skipped,
    })
    .map_err(|e| format!("Failed to serialize Obsidian import result: {}", e))
}

/// Write a completed interview review into a user-selected Obsidian folder.
/// The file is written locally by Rust so the selected Vault path does not
/// need to be inside the app data directory or exposed to the webview.
#[command]
pub async fn write_obsidian_review(
    directory: String,
    title: String,
    meeting_date: String,
    professor_profile: Option<String>,
    content: String,
) -> Result<String, String> {
    let base = PathBuf::from(&directory);
    if !base.is_dir() {
        return Err(format!("Obsidian output folder not found: {}", directory));
    }

    let date = safe_path_component(&meeting_date, "undated");
    let profile = profile_folder_name(professor_profile.as_deref());
    let output_dir = base.join("Interview Assistant").join(profile).join(date);
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("Failed to create Obsidian review folder: {}", error))?;

    let base_filename = safe_path_component(&title, "interview");
    let mut output_path = output_dir.join(format!("{}.md", base_filename));
    if output_path.exists() {
        let suffix = chrono::Utc::now().format("%H%M%S");
        output_path = output_dir.join(format!("{}-{}.md", base_filename, suffix));
    }
    fs::write(&output_path, content)
        .map_err(|error| format!("Failed to write Obsidian review: {}", error))?;

    Ok(serde_json::to_string(&ObsidianReviewWriteResult {
        path: output_path.to_string_lossy().into_owned(),
    })
    .unwrap_or_else(|_| output_path.to_string_lossy().into_owned()))
}

#[command]
pub async fn load_context_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    let resource = {
        let mut ctx = ctx_mgr
            .lock()
            .map_err(|e| format!("Failed to lock context manager: {}", e))?;
        ctx.load_file(&file_path)?
    };

    // Persist to DB so the resource survives app restarts
    if let Some(db_arc) = state.database.as_ref() {
        if let Ok(db_guard) = db_arc.lock() {
            let db_res = crate::db::context::ContextResource {
                id: resource.id.clone(),
                name: resource.name.clone(),
                file_type: resource.file_type.clone(),
                file_path: resource.file_path.clone(),
                size_bytes: resource.size_bytes as i64,
                token_count: resource.token_count as i64,
                preview: resource.preview.clone(),
                loaded_at: resource.loaded_at.clone(),
            };
            if let Err(e) = crate::db::context::add_context_resource(db_guard.connection(), &db_res) {
                log::warn!("Failed to persist context resource to DB: {}", e);
            }
        }
    }

    serde_json::to_string(&resource)
        .map_err(|e| format!("Failed to serialize resource: {}", e))
}

#[command]
pub async fn remove_context_file(
    resource_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    {
        let mut ctx = ctx_mgr
            .lock()
            .map_err(|e| format!("Failed to lock context manager: {}", e))?;
        ctx.remove_file(&resource_id)?;
    }

    // Remove from DB (context record + RAG chunks)
    if let Some(db_arc) = state.database.as_ref() {
        if let Ok(db_guard) = db_arc.lock() {
            let _ = crate::db::context::delete_context_resource(db_guard.connection(), &resource_id);
            let _ = crate::db::rag::delete_chunks_by_file(db_guard.connection(), &resource_id);
        }
    }

    Ok(())
}

#[command]
pub async fn list_context_resources(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    let ctx = ctx_mgr
        .lock()
        .map_err(|e| format!("Failed to lock context manager: {}", e))?;

    let resources = ctx.list_resources();

    serde_json::to_string(&resources)
        .map_err(|e| format!("Failed to serialize resources: {}", e))
}

#[command]
pub async fn set_custom_instructions(
    instructions: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    let mut ctx = ctx_mgr
        .lock()
        .map_err(|e| format!("Failed to lock context manager: {}", e))?;

    ctx.set_custom_instructions(&instructions);

    Ok(())
}

#[command]
pub async fn get_assembled_context(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    let ctx = ctx_mgr
        .lock()
        .map_err(|e| format!("Failed to lock context manager: {}", e))?;

    Ok(ctx.get_assembled_context())
}

#[command]
pub async fn get_token_budget(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    let ctx = ctx_mgr
        .lock()
        .map_err(|e| format!("Failed to lock context manager: {}", e))?;

    // Default model context window: 128k tokens (can be overridden later)
    let model_context_window: u64 = 128_000;
    // Transcript tokens default to 0 when not in a meeting
    let transcript_tokens: usize = 0;

    let budget = ctx.get_token_budget(model_context_window, transcript_tokens);

    serde_json::to_string(&budget)
        .map_err(|e| format!("Failed to serialize token budget: {}", e))
}

#[cfg(test)]
mod tests {
    use super::collect_markdown_files;
    use std::fs;

    #[test]
    fn collect_markdown_files_skips_hidden_obsidian_directories() {
        let root =
            std::env::temp_dir().join(format!("nexq-obsidian-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("interview")).unwrap();
        fs::create_dir_all(root.join(".obsidian")).unwrap();
        fs::write(root.join("README.md"), "read me").unwrap();
        fs::write(root.join("interview/questions.md"), "questions").unwrap();
        fs::write(root.join(".obsidian/app.json"), "{}").unwrap();
        fs::write(root.join("notes.txt"), "not a note for this importer").unwrap();

        let files = collect_markdown_files(&root).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files
            .iter()
            .all(|path| !path.to_string_lossy().contains(".obsidian")));
        let _ = fs::remove_dir_all(root);
    }
}
