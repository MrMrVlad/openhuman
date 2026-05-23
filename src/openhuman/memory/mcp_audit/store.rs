//! SQLite persistence for MCP write audit rows (`mcp_writes` in `chunks.db`).

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};

use crate::openhuman::config::Config;
use crate::openhuman::memory::tree::store::with_connection;

use super::types::{McpAuditListParams, McpWriteRecord};

pub const MAX_LIST_LIMIT: u32 = 500;
const DEFAULT_LIST_LIMIT: u32 = 50;
const TITLE_SUMMARY_MAX_CHARS: usize = 128;

/// Row to insert — `id` is assigned by SQLite.
#[derive(Clone, Debug)]
pub struct NewMcpWriteRecord {
    pub timestamp_ms: i64,
    pub client_info: String,
    pub tool_name: String,
    pub args_summary: Option<String>,
    pub resulting_chunk_id: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

pub fn insert_mcp_write(config: &Config, record: &NewMcpWriteRecord) -> Result<()> {
    with_connection(config, |conn| {
        conn.execute(
            "INSERT INTO mcp_writes (
                timestamp_ms, client_info, tool_name, args_summary,
                resulting_chunk_id, success, error_message
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.timestamp_ms,
                record.client_info,
                record.tool_name,
                record.args_summary,
                record.resulting_chunk_id,
                record.success as i64,
                record.error_message,
            ],
        )?;
        Ok(())
    })
}

pub fn list_mcp_writes(config: &Config, query: &McpAuditListParams) -> Result<Vec<McpWriteRecord>> {
    let limit = clamp_list_limit(query.limit);
    let offset = query.offset;
    with_connection(config, |conn| {
        list_mcp_writes_conn(conn, query, limit, offset)
    })
}

fn list_mcp_writes_conn(
    conn: &Connection,
    query: &McpAuditListParams,
    limit: u32,
    offset: u32,
) -> Result<Vec<McpWriteRecord>> {
    let mut sql = String::from(
        "SELECT id, timestamp_ms, client_info, tool_name, args_summary,
                resulting_chunk_id, success, error_message
           FROM mcp_writes
          WHERE 1=1",
    );
    let mut bind: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(since) = query.since_ms {
        sql.push_str(" AND timestamp_ms >= ?");
        bind.push(Box::new(since as i64));
    }
    if let Some(client) = query.client_filter.as_deref() {
        sql.push_str(" AND client_info = ?");
        bind.push(Box::new(client.to_string()));
    }
    if let Some(tool) = query.tool_filter.as_deref() {
        sql.push_str(" AND tool_name = ?");
        bind.push(Box::new(tool.to_string()));
    }
    if let Some(success_only) = query.success_only {
        sql.push_str(" AND success = ?");
        bind.push(Box::new(if success_only { 1i64 } else { 0i64 }));
    }

    sql.push_str(" ORDER BY timestamp_ms DESC LIMIT ? OFFSET ?");
    bind.push(Box::new(limit as i64));
    bind.push(Box::new(offset as i64));

    let refs: Vec<&dyn rusqlite::types::ToSql> = bind.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(refs.as_slice(), row_to_record)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("collect mcp_writes rows")
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpWriteRecord> {
    Ok(McpWriteRecord {
        id: row.get(0)?,
        timestamp_ms: row.get(1)?,
        client_info: row.get(2)?,
        tool_name: row.get(3)?,
        args_summary: row.get(4)?,
        resulting_chunk_id: row.get(5)?,
        success: row.get::<_, i64>(6)? != 0,
        error_message: row.get(7)?,
    })
}

/// Slim JSON metadata for the audit row — never duplicates document bodies.
pub fn summarize_write_args(tool_name: &str, params: &Map<String, Value>) -> Option<String> {
    let summary = match tool_name {
        "memory.store" => {
            let title = params.get("title").and_then(Value::as_str).unwrap_or("");
            let truncated: String = title.chars().take(TITLE_SUMMARY_MAX_CHARS).collect();
            let namespace = params
                .get("namespace")
                .and_then(Value::as_str)
                .unwrap_or("mcp");
            let tag_count = params
                .get("tags")
                .and_then(Value::as_array)
                .map(|a| a.len())
                .unwrap_or(0);
            json!({
                "title": truncated,
                "namespace": namespace,
                "tag_count": tag_count,
            })
        }
        "memory.note" => {
            let chunk_id = params
                .get("metadata")
                .and_then(Value::as_object)
                .and_then(|m| m.get("annotates_chunk_id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let note_text_length = params
                .get("content")
                .and_then(Value::as_str)
                .map(|s| s.len())
                .unwrap_or(0);
            json!({
                "chunk_id": chunk_id,
                "note_text_length": note_text_length,
            })
        }
        "tree.tag" => {
            let chunk_id = params
                .get("metadata")
                .and_then(Value::as_object)
                .and_then(|m| m.get("tags_for_chunk_id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let tags = params
                .get("tags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            json!({
                "chunk_id": chunk_id,
                "tags": tags,
            })
        }
        _ => return None,
    };
    serde_json::to_string(&summary).ok()
}

pub fn clamp_list_limit(requested: u32) -> u32 {
    if requested == 0 {
        DEFAULT_LIST_LIMIT
    } else {
        requested.min(MAX_LIST_LIMIT)
    }
}
