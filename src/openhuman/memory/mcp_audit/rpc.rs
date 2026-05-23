//! JSON-RPC handler for `openhuman.mcp_audit_list` (#2536).

use crate::openhuman::config::Config;
use crate::rpc::RpcOutcome;

use super::store::{clamp_list_limit, list_mcp_writes};
use super::types::{McpAuditListParams, McpAuditListResponse};

/// `openhuman.mcp_audit_list` — query MCP write audit history (read-only).
pub async fn audit_list_rpc(
    config: &Config,
    params: McpAuditListParams,
) -> Result<RpcOutcome<McpAuditListResponse>, String> {
    let mut params = params;
    params.limit = clamp_list_limit(params.limit);

    tracing::debug!(
        limit = params.limit,
        offset = params.offset,
        "[mcp_audit][rpc] audit_list"
    );

    let config = config.clone();
    let writes = tokio::task::spawn_blocking(move || {
        list_mcp_writes(&config, &params).map_err(|e| format!("mcp_audit list: {e}"))
    })
    .await
    .map_err(|e| format!("mcp_audit list task: {e}"))??;

    Ok(RpcOutcome::single_log(
        McpAuditListResponse { writes },
        "mcp_audit: listed write history".to_string(),
    ))
}
