//! Unit tests for MCP write audit persistence (#2536).

use serde_json::{json, Map, Value};

use crate::openhuman::config::Config;
use crate::openhuman::memory::mcp_audit::store::{
    insert_mcp_write, list_mcp_writes, summarize_write_args, NewMcpWriteRecord,
};
use crate::openhuman::memory::mcp_audit::types::McpAuditListParams;
use crate::openhuman::memory::tree::store::with_connection;
use tempfile::TempDir;

fn test_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tempdir");
    let mut cfg = Config::default();
    cfg.workspace_dir = tmp.path().to_path_buf();
    (tmp, cfg)
}

fn seed_write(
    cfg: &Config,
    ts: i64,
    client: &str,
    tool: &str,
    success: bool,
    chunk_id: Option<&str>,
) {
    insert_mcp_write(
        cfg,
        &NewMcpWriteRecord {
            timestamp_ms: ts,
            client_info: client.to_string(),
            tool_name: tool.to_string(),
            args_summary: Some(r#"{"k":"v"}"#.to_string()),
            resulting_chunk_id: chunk_id.map(str::to_string),
            success,
            error_message: if success {
                None
            } else {
                Some("boom".to_string())
            },
        },
    )
    .expect("insert");
    let _id: i64 = with_connection(cfg, |conn| {
        Ok(conn.query_row(
            "SELECT id FROM mcp_writes ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )?)
    })
    .expect("last id");
}

#[test]
fn insert_success_and_failure_rows() {
    let (_tmp, cfg) = test_config();
    seed_write(
        &cfg,
        1_000,
        "mcp:cursor",
        "memory.store",
        true,
        Some("doc-1"),
    );
    seed_write(&cfg, 2_000, "mcp:cursor", "memory.note", false, None);

    let rows = list_mcp_writes(&cfg, &McpAuditListParams::default()).expect("list");
    assert_eq!(rows.len(), 2);
    assert!(rows[0].timestamp_ms >= rows[1].timestamp_ms);
    assert!(rows
        .iter()
        .any(|r| r.success && r.resulting_chunk_id.as_deref() == Some("doc-1")));
    assert!(rows.iter().any(|r| !r.success && r.error_message.is_some()));
}

#[test]
fn list_filters_by_client_tool_since_and_success() {
    let (_tmp, cfg) = test_config();
    seed_write(&cfg, 100, "mcp:a", "memory.store", true, None);
    seed_write(&cfg, 200, "mcp:b", "tree.tag", true, None);
    seed_write(&cfg, 300, "mcp:b", "memory.store", false, None);

    let client_rows = list_mcp_writes(
        &cfg,
        &McpAuditListParams {
            client_filter: Some("mcp:b".to_string()),
            ..Default::default()
        },
    )
    .expect("client filter");
    assert_eq!(client_rows.len(), 2);
    assert!(client_rows.iter().all(|r| r.client_info == "mcp:b"));

    let tool_rows = list_mcp_writes(
        &cfg,
        &McpAuditListParams {
            tool_filter: Some("memory.store".to_string()),
            ..Default::default()
        },
    )
    .expect("tool filter");
    assert_eq!(tool_rows.len(), 2);

    let since_rows = list_mcp_writes(
        &cfg,
        &McpAuditListParams {
            since_ms: Some(150),
            ..Default::default()
        },
    )
    .expect("since");
    assert_eq!(since_rows.len(), 2);

    let ok_only = list_mcp_writes(
        &cfg,
        &McpAuditListParams {
            success_only: Some(true),
            ..Default::default()
        },
    )
    .expect("success");
    assert_eq!(ok_only.len(), 2);
    assert!(ok_only.iter().all(|r| r.success));
}

#[test]
fn list_respects_limit_and_offset() {
    let (_tmp, cfg) = test_config();
    for i in 0..5 {
        seed_write(&cfg, i as i64 * 10, "mcp", "memory.store", true, None);
    }
    let page0 = list_mcp_writes(
        &cfg,
        &McpAuditListParams {
            limit: 2,
            offset: 0,
            ..Default::default()
        },
    )
    .expect("page0");
    assert_eq!(page0.len(), 2);
    let page1 = list_mcp_writes(
        &cfg,
        &McpAuditListParams {
            limit: 2,
            offset: 2,
            ..Default::default()
        },
    )
    .expect("page1");
    assert_eq!(page1.len(), 2);
    assert_ne!(page0[0].id, page1[0].id);
}

#[test]
fn summarize_write_args_omits_document_bodies() {
    let mut params = Map::new();
    params.insert("title".to_string(), Value::String("Hello".to_string()));
    params.insert(
        "content".to_string(),
        Value::String("secret body must not appear".to_string()),
    );
    params.insert("namespace".to_string(), Value::String("work".to_string()));
    params.insert("tags".to_string(), json!(["a", "b"]));
    let summary = summarize_write_args("memory.store", &params).expect("summary");
    assert!(summary.contains("Hello"));
    assert!(summary.contains("work"));
    assert!(!summary.contains("secret body"));
}

#[test]
fn summarize_memory_note_and_tree_tag_shapes() {
    let mut note = Map::new();
    note.insert(
        "metadata".to_string(),
        json!({ "annotates_chunk_id": "chunk-1" }),
    );
    note.insert(
        "content".to_string(),
        Value::String("note body".to_string()),
    );
    let note_summary = summarize_write_args("memory.note", &note).expect("note");
    assert!(note_summary.contains("chunk-1"));
    assert!(note_summary.contains("note_text_length"));

    let mut tag = Map::new();
    tag.insert(
        "metadata".to_string(),
        json!({ "tags_for_chunk_id": "chunk-2" }),
    );
    tag.insert("tags".to_string(), json!(["urgent", "work"]));
    let tag_summary = summarize_write_args("tree.tag", &tag).expect("tag");
    assert!(tag_summary.contains("chunk-2"));
    assert!(tag_summary.contains("urgent"));
}
