use std::fs;
use std::io::Write;
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

fn is_supported_context_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "pdf" | "txt" | "md" | "docx"
            )
        })
}

fn collect_context_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_context_files_recursive(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_context_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read context folder '{}': {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to inspect context folder entry: {}", e))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect '{}': {}", path.display(), e))?;

        if file_type.is_dir() {
            if !entry.file_name().to_string_lossy().starts_with('.') {
                collect_context_files_recursive(&path, files)?;
            }
        } else if file_type.is_file() && is_supported_context_file(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn persist_context_resource(
    state: &AppState,
    resource: &ContextResource,
) -> Result<(), String> {
    let Some(db_arc) = state.database.as_ref() else {
        return Ok(());
    };
    let db_guard = db_arc
        .lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;
    let db_resource = crate::db::context::ContextResource {
        id: resource.id.clone(),
        name: resource.name.clone(),
        file_type: resource.file_type.clone(),
        file_path: resource.file_path.clone(),
        size_bytes: resource.size_bytes as i64,
        token_count: resource.token_count as i64,
        preview: resource.preview.clone(),
        loaded_at: resource.loaded_at.clone(),
        source_folder: resource.source_folder.clone(),
    };
    crate::db::context::add_context_resource(db_guard.connection(), &db_resource)
        .map_err(|e| format!("Failed to persist context resource: {}", e))
}

fn persist_context_resources(
    state: &AppState,
    resources: &[ContextResource],
) -> Result<(), String> {
    for resource in resources {
        persist_context_resource(state, resource)?;
    }
    Ok(())
}

fn delete_context_resources_from_database(
    state: &AppState,
    resource_ids: &[String],
    operation: &str,
) -> Result<(), String> {
    if let Some(db_arc) = state.database.as_ref() {
        let db_guard = db_arc
            .lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;
        crate::db::context::delete_context_resources(db_guard.connection(), resource_ids)
            .map_err(|e| format!("Failed to {} context resources in database: {}", operation, e))?;
    }
    Ok(())
}

fn clear_context_database(state: &AppState) -> Result<(), String> {
    if let Some(db_arc) = state.database.as_ref() {
        let db_guard = db_arc
            .lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;
        crate::db::context::clear_context_resources(db_guard.connection())
            .map_err(|e| format!("Failed to clear knowledge base database: {}", e))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct ContextFolderImportResult {
    folder_path: String,
    imported: Vec<ContextResource>,
    skipped: Vec<ObsidianSkippedFile>,
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
            match ctx.load_file_with_source_folder(&path_string, Some(&vault_path)) {
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
    persist_context_resources(&state, &imported)?;

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

/// Import every supported context file under a selected folder. The folder is
/// stored as group metadata so it can later be replaced or deleted in one
/// operation; only the app-managed copies are removed by those operations.
#[command]
pub async fn import_context_folder(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let folder = PathBuf::from(&folder_path);
    if !folder.is_dir() {
        return Err(format!("Knowledge base folder not found: {}", folder_path));
    }

    let files = collect_context_files(&folder)?;
    if files.is_empty() {
        return Err("No supported files found in the selected knowledge-base folder".to_string());
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
            match ctx.load_file_with_source_folder(&path_string, Some(&folder_path)) {
                Ok(resource) => imported.push(resource),
                Err(reason) => skipped.push(ObsidianSkippedFile {
                    path: path_string,
                    reason,
                }),
            }
        }
    }

    persist_context_resources(&state, &imported)?;
    serde_json::to_string(&ContextFolderImportResult {
        folder_path,
        imported,
        skipped,
    })
    .map_err(|e| format!("Failed to serialize folder import result: {}", e))
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

/// Append the structured Mistake Bank section from one review into a durable
/// per-profile Obsidian note. Existing entries are never overwritten.
#[command]
pub async fn append_obsidian_mistake_bank(
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
    if content.trim().is_empty() {
        return Err("Mistake Bank content is empty".to_string());
    }

    let profile = profile_folder_name(professor_profile.as_deref());
    let output_dir = base.join("Interview Assistant").join(&profile);
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("Failed to create Obsidian mistake-bank folder: {}", error))?;

    let output_path = output_dir.join("Mistake Bank.md");
    let is_new = !output_path.exists();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&output_path)
        .map_err(|error| format!("Failed to open Obsidian mistake bank: {}", error))?;

    if is_new {
        writeln!(file, "# Interview Mistake Bank")
            .map_err(|error| format!("Failed to write Obsidian mistake bank: {}", error))?;
        writeln!(file, "\nProfile: {}", profile)
            .map_err(|error| format!("Failed to write Obsidian mistake bank: {}", error))?;
    }

    let date = safe_path_component(&meeting_date, "undated");
    let entry_title = safe_path_component(&title, "Interview");
    writeln!(file, "\n## {} — {}\n", date, entry_title)
        .map_err(|error| format!("Failed to write Obsidian mistake bank: {}", error))?;
    writeln!(file, "{}", content.trim())
        .map_err(|error| format!("Failed to write Obsidian mistake bank: {}", error))?;

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

    // Persist to DB so the resource survives app restarts.
    persist_context_resource(&state, &resource)?;

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

    // Validate before touching the database. The database deletion happens
    // first so a database failure cannot leave an in-memory resource invisible
    // from the persisted knowledge base.
    {
        let ctx = ctx_mgr
            .lock()
            .map_err(|e| format!("Failed to lock context manager: {}", e))?;
        if !ctx
            .list_resources()
            .iter()
            .any(|resource| resource.id == resource_id)
        {
            return Err(format!("Resource not found: {}", resource_id));
        }
    }

    let ids = vec![resource_id.clone()];
    delete_context_resources_from_database(&state, &ids, "remove")?;

    let mut ctx = ctx_mgr
        .lock()
        .map_err(|e| format!("Failed to lock context manager: {}", e))?;
    ctx.remove_file(&resource_id)?;
    ctx.cleanup_empty_storage_dir();

    Ok(())
}

#[command]
pub async fn remove_context_files(
    resource_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    if resource_ids.is_empty() {
        return Ok(0);
    }

    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    let mut resource_ids = resource_ids;
    resource_ids.sort();
    resource_ids.dedup();

    // Validate every ID before touching the database. The database deletion
    // happens before removing in-memory resources to avoid split state when
    // the database is locked or otherwise fails.
    {
        let ctx = ctx_mgr
            .lock()
            .map_err(|e| format!("Failed to lock context manager: {}", e))?;
        let resources = ctx.list_resources();
        for resource_id in &resource_ids {
            if !resources.iter().any(|resource| resource.id == *resource_id) {
                return Err(format!("Resource not found: {}", resource_id));
            }
        }
    }

    delete_context_resources_from_database(&state, &resource_ids, "remove")?;

    let mut ctx = ctx_mgr
        .lock()
        .map_err(|e| format!("Failed to lock context manager: {}", e))?;
    let removed = ctx.remove_files(&resource_ids)?;
    ctx.cleanup_empty_storage_dir();

    Ok(removed.len())
}

#[command]
pub async fn remove_context_folder(
    source_folder: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    let ids = {
        let ctx = ctx_mgr
            .lock()
            .map_err(|e| format!("Failed to lock context manager: {}", e))?;
        ctx.resource_ids_for_folder(&source_folder)
    };
    if ids.is_empty() {
        return Ok(0);
    }

    delete_context_resources_from_database(&state, &ids, "remove folder")?;

    let mut ctx = ctx_mgr
        .lock()
        .map_err(|e| format!("Failed to lock context manager: {}", e))?;
    let removed = ctx.remove_files(&ids)?;
    ctx.cleanup_empty_storage_dir();

    Ok(removed.len())
}

#[command]
pub async fn clear_context_resources(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let ctx_mgr = state
        .context
        .as_ref()
        .ok_or_else(|| "Context manager not initialized".to_string())?;

    let resource_count = {
        let ctx = ctx_mgr
            .lock()
            .map_err(|e| format!("Failed to lock context manager: {}", e))?;
        ctx.list_resources().len()
    };

    clear_context_database(&state)?;

    let mut ctx = ctx_mgr
        .lock()
        .map_err(|e| format!("Failed to lock context manager: {}", e))?;
    let removed_count = ctx.remove_all_files()?.len();

    Ok(removed_count.max(resource_count))
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
