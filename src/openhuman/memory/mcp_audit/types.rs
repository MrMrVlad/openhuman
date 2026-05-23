//! Wire types for `openhuman.mcp_audit_list` (#2536).

use serde::{Deserialize, Serialize};

/// One persisted MCP write-tool audit row.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpWriteRecord {
    pub id: i64,
    pub timestamp_ms: i64,
    pub client_info: String,
    pub tool_name: String,
    pub args_summary: Option<String>,
    pub resulting_chunk_id: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Query parameters for `openhuman.mcp_audit_list`.
#[derive(Clone, Debug, Deserialize)]
pub struct McpAuditListParams {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub since_ms: Option<u64>,
    #[serde(default)]
    pub client_filter: Option<String>,
    #[serde(default)]
    pub tool_filter: Option<String>,
    #[serde(default)]
    pub success_only: Option<bool>,
}

impl Default for McpAuditListParams {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            offset: 0,
            since_ms: None,
            client_filter: None,
            tool_filter: None,
            success_only: None,
        }
    }
}

fn default_limit() -> u32 {
    50
}

/// Response envelope for `openhuman.mcp_audit_list`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpAuditListResponse {
    pub writes: Vec<McpWriteRecord>,
}
