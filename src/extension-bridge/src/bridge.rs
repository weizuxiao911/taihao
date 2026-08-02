//! JSON-RPC 2.0 消息模型与桥接核心
//!
//! 权限边界：工具必须注册（白名单）；参数只允许注入白名单字段；
//! 命令经 sh -c 执行，参数做单引号安全转义。

use serde_json::{json, Value};

/// JSON-RPC 请求
#[derive(Debug, serde::Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 响应（成功）
#[derive(Debug, serde::Serialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Value,
}

/// JSON-RPC 错误响应
#[derive(Debug, serde::Serialize)]
pub struct RpcError {
    pub jsonrpc: String,
    pub id: Value,
    pub error: Value,
}

pub fn error(id: Value, code: i64, message: &str) -> RpcError {
    RpcError { jsonrpc: "2.0".into(), id, error: json!({"code": code, "message": message}) }
}

pub fn ok(id: Value, result: Value) -> RpcResponse {
    RpcResponse { jsonrpc: "2.0".into(), id, result }
}

/// 校验请求：JSON-RPC 2.0 形状
pub fn validate(req: &RpcRequest) -> Result<(), String> {
    if req.jsonrpc != "2.0" {
        return Err("jsonrpc 必须为 2.0".into());
    }
    if req.method.is_empty() {
        return Err("method 不能为空".into());
    }
    Ok(())
}

/// 参数白名单提取：只取工具允许的参数
pub fn filter_params(tool_args: &[String], params: &Value) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    if let Some(obj) = params.as_object() {
        for key in tool_args {
            if let Some(v) = obj.get(key) {
                out.insert(key.clone(), v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string()));
            }
        }
    }
    out
}

/// 单引号安全转义（注入 sh -c 的参数；envs 注入方式下保留供脚本参数场景）
#[allow(dead_code)]
pub fn shell_escape(s: &str) -> String {
    s.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_request() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"ping","args":{}}}"#;
        let req: RpcRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "tools/call");
    }

    #[test]
    fn validate_rejects_bad_version() {
        let req: RpcRequest = serde_json::from_str(r#"{"jsonrpc":"1.0","id":1,"method":"x"}"#).unwrap();
        assert!(validate(&req).is_err());
    }

    #[test]
    fn filter_params_only_whitelist() {
        let params = json!({"allowed": "yes", "sneaky": "no"});
        let filtered = filter_params(&["allowed".to_string()], &params);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.get("allowed").unwrap(), "yes");
    }

    #[test]
    fn shell_escape_quotes() {
        assert_eq!(shell_escape("a'b"), "a'\\''b");
    }
}
