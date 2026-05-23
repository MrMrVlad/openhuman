//! Controller-registry schemas for `openhuman.mcp_audit_list` (#2536).

use serde_json::{Map, Value};

use crate::core::all::{ControllerFuture, RegisteredController};
use crate::core::{ControllerSchema, FieldSchema, TypeSchema};
use crate::openhuman::config::ops::load_config_with_timeout;
use crate::rpc::RpcOutcome;

use super::rpc;
use super::types::McpAuditListParams;

pub fn all_controller_schemas() -> Vec<ControllerSchema> {
    vec![schemas("audit_list")]
}

pub fn all_registered_controllers() -> Vec<RegisteredController> {
    vec![RegisteredController {
        schema: schemas("audit_list"),
        handler: handle_audit_list,
    }]
}

pub fn schemas(function: &str) -> ControllerSchema {
    match function {
        "audit_list" => ControllerSchema {
            namespace: "mcp_audit",
            function: "audit_list",
            description:
                "List MCP write-tool audit history (memory.store, memory.note, tree.tag). \
                 Metadata only — not exposed to MCP clients.",
            inputs: vec![
                FieldSchema {
                    name: "limit",
                    ty: TypeSchema::U64,
                    comment: "Max rows (default 50, max 500).",
                    required: false,
                },
                FieldSchema {
                    name: "offset",
                    ty: TypeSchema::U64,
                    comment: "Pagination offset (default 0).",
                    required: false,
                },
                FieldSchema {
                    name: "since_ms",
                    ty: TypeSchema::Option(Box::new(TypeSchema::U64)),
                    comment: "Only rows at or after this Unix timestamp (ms).",
                    required: false,
                },
                FieldSchema {
                    name: "client_filter",
                    ty: TypeSchema::Option(Box::new(TypeSchema::String)),
                    comment: "Filter by client_info (e.g. mcp:claude-desktop).",
                    required: false,
                },
                FieldSchema {
                    name: "tool_filter",
                    ty: TypeSchema::Option(Box::new(TypeSchema::String)),
                    comment: "Filter by tool_name (memory.store, memory.note, tree.tag).",
                    required: false,
                },
                FieldSchema {
                    name: "success_only",
                    ty: TypeSchema::Option(Box::new(TypeSchema::Bool)),
                    comment: "When set, filter to successes (true) or failures (false).",
                    required: false,
                },
            ],
            outputs: vec![FieldSchema {
                name: "writes",
                ty: TypeSchema::Array(Box::new(TypeSchema::Ref("McpWriteRecord"))),
                comment: "Audit rows, newest first.",
                required: true,
            }],
        },
        other => panic!("unknown mcp_audit schema function: {other}"),
    }
}

fn handle_audit_list(params: Map<String, Value>) -> ControllerFuture {
    Box::pin(async move {
        let config = load_config_with_timeout().await?;
        let parsed: McpAuditListParams = serde_json::from_value(Value::Object(params))
            .map_err(|e| format!("invalid mcp_audit_list params: {e}"))?;
        to_json(rpc::audit_list_rpc(&config, parsed).await?)
    })
}

fn to_json<T: serde::Serialize>(outcome: RpcOutcome<T>) -> Result<Value, String> {
    outcome.into_cli_compatible_json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_only_audit_list() {
        let regs = all_registered_controllers();
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].schema.function, "audit_list");
        assert_eq!(regs[0].schema.namespace, "mcp_audit");
    }
}
