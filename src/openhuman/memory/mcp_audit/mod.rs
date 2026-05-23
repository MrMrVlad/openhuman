//! MCP write-tool audit log (#2536).
//!
//! Persists metadata for `memory.store`, `memory.note`, and `tree.tag` invocations
//! in the memory-tree SQLite DB (`mcp_writes` table). Queried via the internal
//! `openhuman.mcp_audit_list` RPC (not exposed as an MCP tool in v1).

pub mod rpc;
pub mod schemas;
pub mod store;
pub mod types;

pub use schemas::{
    all_controller_schemas as all_mcp_audit_controller_schemas,
    all_registered_controllers as all_mcp_audit_registered_controllers,
};
pub use store::{insert_mcp_write, summarize_write_args, NewMcpWriteRecord};
pub use types::{McpAuditListParams, McpAuditListResponse, McpWriteRecord};

#[cfg(test)]
mod store_tests;
