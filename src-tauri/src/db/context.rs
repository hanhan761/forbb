use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::DatabaseError;

// ── Data structures ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResource {
    pub id: String,
    pub name: String,
    pub file_type: String,
    pub file_path: String,
    pub size_bytes: i64,
    pub token_count: i64,
    pub preview: String,
    pub loaded_at: String,
    pub source_folder: String,
}

// ── CRUD operations ──────────────────────────────────────────────────────────

/// Insert a new context resource record.
pub fn add_context_resource(
    conn: &Connection,
    resource: &ContextResource,
) -> Result<(), DatabaseError> {
    conn.execute(
        "INSERT INTO context_resources (id, name, file_type, file_path, size_bytes, token_count, preview, loaded_at, source_folder)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            resource.id,
            resource.name,
            resource.file_type,
            resource.file_path,
            resource.size_bytes,
            resource.token_count,
            resource.preview,
            resource.loaded_at,
            resource.source_folder,
        ],
    )?;

    Ok(())
}

/// Get a single context resource by ID.
pub fn get_context_resource(
    conn: &Connection,
    id: &str,
) -> Result<ContextResource, DatabaseError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, file_type, file_path, size_bytes, token_count, preview, loaded_at, source_folder
         FROM context_resources WHERE id = ?1",
    )?;

    stmt.query_row(params![id], |row| {
        Ok(ContextResource {
            id: row.get(0)?,
            name: row.get(1)?,
            file_type: row.get(2)?,
            file_path: row.get(3)?,
            size_bytes: row.get(4)?,
            token_count: row.get(5)?,
            preview: row.get(6)?,
            loaded_at: row.get(7)?,
            source_folder: row.get(8)?,
        })
    })
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            DatabaseError::NotFound(format!("Context resource {} not found", id))
        }
        other => DatabaseError::Query(other.to_string()),
    })
}

/// List all context resources, ordered by most recently loaded.
pub fn list_context_resources(conn: &Connection) -> Result<Vec<ContextResource>, DatabaseError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, file_type, file_path, size_bytes, token_count, preview, loaded_at, source_folder
         FROM context_resources
         ORDER BY loaded_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ContextResource {
            id: row.get(0)?,
            name: row.get(1)?,
            file_type: row.get(2)?,
            file_path: row.get(3)?,
            size_bytes: row.get(4)?,
            token_count: row.get(5)?,
            preview: row.get(6)?,
            loaded_at: row.get(7)?,
            source_folder: row.get(8)?,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Delete a context resource by ID.
pub fn delete_context_resource(conn: &Connection, id: &str) -> Result<(), DatabaseError> {
    let rows = conn.execute("DELETE FROM context_resources WHERE id = ?1", params![id])?;
    if rows == 0 {
        return Err(DatabaseError::NotFound(format!(
            "Context resource {} not found",
            id
        )));
    }
    Ok(())
}

/// Delete a set of resources and their RAG chunks in one transaction.
pub fn delete_context_resources(
    conn: &Connection,
    ids: &[String],
) -> Result<(), DatabaseError> {
    if ids.is_empty() {
        return Ok(());
    }

    let tx = conn.unchecked_transaction()?;
    for id in ids {
        tx.execute(
            "DELETE FROM rag_embeddings WHERE chunk_id IN (SELECT chunk_id FROM rag_chunks WHERE file_id = ?1)",
            params![id],
        )?;
        tx.execute("DELETE FROM rag_chunks WHERE file_id = ?1", params![id])?;
    }

    let placeholders = std::iter::repeat("?")
        .take(ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let delete_sql = format!(
        "DELETE FROM context_resources WHERE id IN ({})",
        placeholders
    );
    tx.execute(&delete_sql, rusqlite::params_from_iter(ids.iter()))?;
    tx.commit()?;

    Ok(())
}

/// Delete all context resources and file-backed RAG rows, preserving any
/// transcript-only rows that belong to the active meeting session.
pub fn clear_context_resources(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute(
        "DELETE FROM rag_embeddings WHERE chunk_id IN (
            SELECT chunk_id FROM rag_chunks WHERE source_type = 'file'
        )",
        [],
    )?;
    conn.execute("DELETE FROM rag_chunks WHERE source_type = 'file'", [])?;
    conn.execute("DELETE FROM context_resources", [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        add_context_resource, clear_context_resources, delete_context_resources,
        list_context_resources, ContextResource,
    };
    use crate::db::migrations;
    use crate::db::rag::{insert_chunks_batch, store_embedding, ChunkRecord};
    use rusqlite::Connection;

    #[test]
    fn persists_source_folder_and_cleans_rag_rows_in_batch() {
        let conn = Connection::open_in_memory().expect("open test database");
        migrations::run(&conn).expect("run migrations");

        let resource = ContextResource {
            id: "resource-1".to_string(),
            name: "notes.txt".to_string(),
            file_type: "txt".to_string(),
            file_path: "C:/managed/resource-1_notes.txt".to_string(),
            size_bytes: 10,
            token_count: 3,
            preview: "notes".to_string(),
            loaded_at: "2026-09-16T00:00:00Z".to_string(),
            source_folder: "C:/knowledge-base".to_string(),
        };
        add_context_resource(&conn, &resource).expect("insert resource");
        insert_chunks_batch(
            &conn,
            &[ChunkRecord {
                chunk_id: "chunk-1".to_string(),
                file_id: resource.id.clone(),
                chunk_index: 0,
                text: "notes".to_string(),
                token_count: 1,
                source_type: "file".to_string(),
            }],
        )
        .expect("insert chunk");
        store_embedding(&conn, "chunk-1", &[1, 2, 3]).expect("insert embedding");

        let listed = list_context_resources(&conn).expect("list resources");
        assert_eq!(listed[0].source_folder, "C:/knowledge-base");

        insert_chunks_batch(
            &conn,
            &[ChunkRecord {
                chunk_id: "transcript-chunk-1".to_string(),
                file_id: "transcript-session-1".to_string(),
                chunk_index: 0,
                text: "transcript row should survive context cleanup".to_string(),
                token_count: 1,
                source_type: "transcript".to_string(),
            }],
        )
        .expect("insert transcript chunk");

        delete_context_resources(&conn, &[resource.id]).expect("delete resource batch");
        assert!(list_context_resources(&conn).unwrap().is_empty());
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM rag_chunks WHERE source_type = 'file'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM rag_chunks WHERE source_type = 'transcript'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM rag_embeddings", [], |row| row.get::<_, i64>(0)).unwrap(), 0);

        clear_context_resources(&conn).expect("clear context resources");
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM rag_chunks WHERE source_type = 'transcript'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }
}
