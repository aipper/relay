use anyhow::Context;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, StatusCode};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

fn usage() -> ! {
    eprintln!(
        r#"relay (packaged-friendly)

Usage:
  relay codex  [--sock /path/to/relay-hostd.sock] [--cmd "codex ..."] [--cwd /path/to/project]
  relay claude [--sock /path/to/relay-hostd.sock] [--cmd "claude ..."] [--cwd /path/to/project]
  relay iflow  [--sock /path/to/relay-hostd.sock] [--cmd "iflow ..."] [--cwd /path/to/project]
  relay gemini [--sock /path/to/relay-hostd.sock] [--cmd "gemini ..."] [--cwd /path/to/project]

  relay mcp [--root /path/to/project]

Notes:
  - If --cmd is omitted, it defaults to the subcommand name (e.g. `codex`).
  - If --cwd is omitted, it defaults to the current working directory.
  - If --sock is omitted, it tries RELAY_HOSTD_SOCK, ~/.relay/relay-hostd.sock, then ~/.relay/daemon.state.json.
  - `--cmd` supports simple argv forms (e.g. `codex --help`). For shell pipelines/quotes, prefer using hostd directly.
"#
    );
    std::process::exit(2);
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[derive(Serialize)]
struct StartRunRequest {
    tool: String,
    cmd: String,
    cwd: Option<String>,
}

#[derive(Deserialize)]
struct StartRunResponse {
    run_id: String,
}

async fn post_json_unix<TReq: Serialize>(
    sock_path: &str,
    path: &str,
    body: &TReq,
) -> anyhow::Result<(StatusCode, String)> {
    request_unix(
        sock_path,
        "POST",
        path,
        Some("application/json"),
        Some(body),
    )
    .await
}

async fn get_unix(sock_path: &str, path: &str) -> anyhow::Result<(StatusCode, String)> {
    request_unix::<JsonValue>(sock_path, "GET", path, None, None).await
}

async fn request_unix<TReq: Serialize>(
    sock_path: &str,
    method: &str,
    path: &str,
    content_type: Option<&str>,
    body: Option<&TReq>,
) -> anyhow::Result<(StatusCode, String)> {
    let stream = tokio::net::UnixStream::connect(sock_path)
        .await
        .with_context(|| format!("connect unix socket: {sock_path}"))?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .context("http1 handshake")?;
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let req = Request::builder()
        .method(method)
        .uri(format!("http://localhost{path}"))
        .header("content-type", content_type.unwrap_or("application/json"))
        .body(match body {
            Some(v) => {
                let json = serde_json::to_vec(v).context("encode json")?;
                Full::new(Bytes::from(json))
            }
            None => Full::new(Bytes::new()),
        })
        .context("build request")?;

    let resp = sender.send_request(req).await.context("send request")?;
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .context("read response body")?
        .to_bytes();
    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok((status, text))
}

fn percent_encode_query_value(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for &b in v.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn relay_home_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(std::path::PathBuf::from(home).join(".relay"))
}

fn xdg_config_home_dir() -> Option<std::path::PathBuf> {
    let v = std::env::var("XDG_CONFIG_HOME").ok()?;
    let v = v.trim().to_string();
    if v.is_empty() {
        return None;
    }
    Some(std::path::PathBuf::from(v))
}

fn default_abrelay_hostd_config_path() -> Option<std::path::PathBuf> {
    let base = xdg_config_home_dir().or_else(|| {
        let home = std::env::var_os("HOME")?;
        let home = home.to_string_lossy().trim().to_string();
        if home.is_empty() {
            return None;
        }
        Some(std::path::PathBuf::from(home).join(".config"))
    })?;
    Some(base.join("abrelay").join("hostd.json"))
}

fn resolve_abrelay_config_path() -> Option<std::path::PathBuf> {
    if let Ok(v) = std::env::var("ABRELAY_CONFIG") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(std::path::PathBuf::from(v));
        }
    }
    default_abrelay_hostd_config_path()
}

fn socket_from_abrelay_config() -> Option<String> {
    let path = resolve_abrelay_config_path()?;
    let raw = std::fs::read_to_string(path).ok()?;
    let v = serde_json::from_str::<JsonValue>(&raw).ok()?;
    v.get("local_unix_socket")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
}

fn daemon_state_sock() -> Option<String> {
    let root = relay_home_dir()?;
    let raw = std::fs::read_to_string(root.join("daemon.state.json")).ok()?;
    let v = serde_json::from_str::<JsonValue>(&raw).ok()?;
    v.get("sock")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

async fn pick_sock(sock_arg: Option<String>) -> anyhow::Result<String> {
    if let Some(s) = sock_arg.filter(|s| !s.trim().is_empty()) {
        return Ok(s);
    }
    if let Ok(s) = std::env::var("RELAY_HOSTD_SOCK") {
        if !s.trim().is_empty() {
            return Ok(s);
        }
    }

    let mut candidates = Vec::<String>::new();
    if let Some(s) = socket_from_abrelay_config() {
        candidates.push(s);
    }
    if let Some(root) = relay_home_dir() {
        candidates.push(root.join("relay-hostd.sock").to_string_lossy().to_string());
    }
    if let Some(s) = daemon_state_sock() {
        candidates.push(s);
    }

    // Pick the first connectable socket.
    let mut tried = Vec::<String>::new();
    for c in candidates {
        tried.push(c.clone());
        if tokio::net::UnixStream::connect(&c).await.is_ok() {
            return Ok(c);
        }
    }

    if tried.is_empty() {
        return Err(anyhow::anyhow!(
            "missing hostd unix socket; set --sock or RELAY_HOSTD_SOCK or run relay-hostd"
        ));
    }
    Err(anyhow::anyhow!(
        "hostd unix socket not connectable; tried: {}",
        tried.join(", ")
    ))
}

#[derive(Deserialize)]
struct JsonRpcReq {
    id: Option<JsonValue>,
    method: Option<String>,
    params: Option<JsonValue>,
}

fn jsonrpc_ok(id: Option<JsonValue>, result: JsonValue) -> JsonValue {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(JsonValue::Null),
        "result": result
    })
}

fn jsonrpc_err(id: Option<JsonValue>, code: i64, message: &str) -> JsonValue {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(JsonValue::Null),
        "error": { "code": code, "message": message }
    })
}

fn is_rel_path(p: &str) -> bool {
    let path = std::path::Path::new(p);
    !path.is_absolute() && !p.split('/').any(|c| c == "..")
}

fn safe_join(root: &std::path::Path, rel: &str) -> anyhow::Result<std::path::PathBuf> {
    if rel.trim().is_empty() || !is_rel_path(rel) {
        return Err(anyhow::anyhow!("path must be a relative path within root"));
    }
    Ok(root.join(rel))
}

fn tool_list_result(include_mutations: bool) -> JsonValue {
    let mut tools = vec![
        serde_json::json!({
            "name": "fs_read",
            "description": "Read a UTF-8 text file relative to the run cwd (hostd mode) or under the configured root (local mode). Path must be relative.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative file path" },
                    "max_bytes": { "type": "integer", "description": "Optional max bytes (default 1048576, best-effort)" }
                },
                "required": ["path"]
            }
        }),
        serde_json::json!({
            "name": "fs_search",
            "description": "Search for a pattern relative to the run cwd (hostd mode) or under root (local mode).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "q": { "type": "string", "description": "Search query" },
                    "max_matches": { "type": "integer", "description": "Optional max matches (default 200)" }
                },
                "required": ["q"]
            }
        }),
        serde_json::json!({
            "name": "git_status",
            "description": "Run `git status --porcelain=v1 -b` relative to the run cwd (hostd mode) or under root (local mode).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        serde_json::json!({
            "name": "git_diff",
            "description": "Run `git diff` relative to the run cwd (hostd mode) or under root (local mode).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Optional relative file path" }
                }
            }
        }),
    ];

    if include_mutations {
        tools.push(serde_json::json!({
            "name": "fs_write",
            "description": "Write a UTF-8 text file relative to the run cwd (requires approval via relay PWA).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative file path" },
                    "content": { "type": "string", "description": "Full file content" }
                },
                "required": ["path", "content"]
            }
        }));
        tools.push(serde_json::json!({
            "name": "bash",
            "description": "Run a shell command under the run cwd (requires approval via relay PWA).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cmd": { "type": "string", "description": "Shell command" }
                },
                "required": ["cmd"]
            }
        }));
    }

    serde_json::json!({
        "tools": tools
    })
}

fn tool_text_result(text: String) -> JsonValue {
    serde_json::json!({
        "content": [{ "type": "text", "text": text }],
        "isError": false
    })
}

fn tool_error_result(text: String) -> JsonValue {
    serde_json::json!({
        "content": [{ "type": "text", "text": text }],
        "isError": true
    })
}

fn normalize_mcp_tool_name(raw_name: &str) -> &str {
    let raw = raw_name.trim();
    if raw.is_empty() {
        return raw;
    }

    // Some clients namespace MCP tool names as `<server>.<tool>`.
    let dot = raw.rsplit('.').next().unwrap_or(raw);

    // Claude Code commonly exposes MCP tools to the model as `mcp__<server>__<tool>`.
    // We accept that form too in case the client forwards it unchanged.
    dot.rsplit("__").next().unwrap_or(dot)
}

#[cfg(test)]
mod tests {
    use super::normalize_mcp_tool_name;

    #[test]
    fn normalizes_common_mcp_tool_name_prefixes() {
        assert_eq!(normalize_mcp_tool_name("fs_read"), "fs_read");
        assert_eq!(normalize_mcp_tool_name("relay.fs_read"), "fs_read");
        assert_eq!(normalize_mcp_tool_name("mcp__relay__fs_read"), "fs_read");
        assert_eq!(normalize_mcp_tool_name("mcp__relay__bash"), "bash");
        assert_eq!(normalize_mcp_tool_name("   mcp__relay__git_status  "), "git_status");
    }
}

#[derive(Clone)]
enum McpMode {
    Hostd {
        sock_path: String,
        run_id: String,
        actor: String,
    },
    Local {
        root: std::path::PathBuf,
    },
}

#[derive(Deserialize)]
struct HostdReadFileResponse {
    path: String,
    content: String,
    truncated: bool,
}

#[derive(Serialize, Deserialize)]
struct HostdSearchMatch {
    path: String,
    line: i64,
    column: i64,
    text: String,
}

#[derive(Serialize, Deserialize)]
struct HostdSearchResponse {
    matches: Vec<HostdSearchMatch>,
    truncated: bool,
}

#[derive(Deserialize)]
struct HostdGitTextResponse {
    stdout: String,
    truncated: bool,
}

#[derive(Serialize)]
struct HostdWriteFileRequest {
    path: String,
    content: String,
    actor: String,
}

#[derive(Deserialize)]
struct HostdWriteFileResponse {
    path: String,
    bytes_written: i64,
    truncated: bool,
}

#[derive(Serialize)]
struct HostdBashRequest {
    cmd: String,
    actor: String,
}

#[derive(Deserialize)]
struct HostdBashResponse {
    stdout: String,
    stderr: String,
    exit_code: i64,
    truncated: bool,
}

fn truncate_utf8_bytes(s: &str, max_bytes: usize) -> (String, bool) {
    if max_bytes == 0 {
        return (String::new(), !s.is_empty());
    }
    let b = s.as_bytes();
    if b.len() <= max_bytes {
        return (s.to_string(), false);
    }
    let mut end = max_bytes;
    while end > 0 && std::str::from_utf8(&b[..end]).is_err() {
        end -= 1;
    }
    let truncated = end < b.len();
    (String::from_utf8_lossy(&b[..end]).to_string(), truncated)
}

async fn run_mcp(root: std::path::PathBuf) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut lines = tokio::io::BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    let mode = match (
        std::env::var("RELAY_HOSTD_SOCK").ok(),
        std::env::var("RELAY_RUN_ID").ok(),
    ) {
        (Some(sock), Some(run_id)) if !sock.trim().is_empty() && !run_id.trim().is_empty() => {
            let actor = std::env::var("RELAY_TOOL")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .map(|tool| format!("{tool}-mcp"))
                .unwrap_or_else(|| "relay-mcp".to_string());
            McpMode::Hostd {
                sock_path: sock,
                run_id,
                actor,
            }
        }
        _ => McpMode::Local { root },
    };

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let req: JsonRpcReq = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                let msg = jsonrpc_err(None, -32700, "parse error");
                stdout.write_all(msg.to_string().as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                continue;
            }
        };

        let Some(method) = req.method.clone() else {
            continue;
        };
        let id = req.id.clone();

        let resp = match method.as_str() {
            "initialize" => {
                // Minimal capabilities (tools only).
                let instructions = match &mode {
                    McpMode::Hostd { .. } => {
                        "Tools are scoped to the run working directory. Some tools require explicit approval in the relay PWA."
                    }
                    McpMode::Local { .. } => {
                        "Tools are restricted to the configured root directory. Paths must be relative."
                    }
                };
                jsonrpc_ok(
                    id.clone(),
                    serde_json::json!({
                        "protocolVersion": "2025-06-18",
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "relay-mcp", "version": env!("CARGO_PKG_VERSION") },
                        "instructions": instructions
                    }),
                )
            }
            "tools/list" => {
                let include_mutations = matches!(&mode, McpMode::Hostd { .. });
                jsonrpc_ok(id.clone(), tool_list_result(include_mutations))
            }
            "tools/call" => {
                let params = req.params.unwrap_or(JsonValue::Null);
                let raw_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let name = normalize_mcp_tool_name(raw_name);
                let args = params.get("arguments").cloned().unwrap_or(JsonValue::Null);
                match name {
                    "fs_read" => {
                        let rel = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        if rel.trim().is_empty() {
                            jsonrpc_ok(id.clone(), tool_error_result("missing path".into()))
                        } else {
                            let max_bytes = args
                                .get("max_bytes")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(1024 * 1024)
                                as usize;
                            match &mode {
                                McpMode::Hostd {
                                    sock_path,
                                    run_id,
                                    actor,
                                } => {
                                    let path = format!(
                                        "/runs/{}/fs/read?path={}&actor={}",
                                        percent_encode_query_value(run_id),
                                        percent_encode_query_value(rel),
                                        percent_encode_query_value(actor)
                                    );
                                    match get_unix(sock_path, &path).await {
                                        Err(e) => {
                                            jsonrpc_ok(id.clone(), tool_error_result(e.to_string()))
                                        }
                                        Ok((status, body)) => {
                                            if status != StatusCode::OK {
                                                jsonrpc_ok(
                                                    id.clone(),
                                                    tool_error_result(format!(
                                                        "hostd returned {status}: {body}"
                                                    )),
                                                )
                                            } else {
                                                match serde_json::from_str::<HostdReadFileResponse>(
                                                    &body,
                                                ) {
                                                    Err(e) => jsonrpc_ok(
                                                        id.clone(),
                                                        tool_error_result(format!(
                                                            "decode response: {e}"
                                                        )),
                                                    ),
                                                    Ok(mut r) => {
                                                        let mut truncated = r.truncated;
                                                        let (text, extra_trunc) =
                                                            truncate_utf8_bytes(
                                                                &r.content, max_bytes,
                                                            );
                                                        if extra_trunc {
                                                            truncated = true;
                                                        }
                                                        r.content = text;
                                                        let out = serde_json::json!({
                                                            "content": [{ "type": "text", "text": r.content }],
                                                            "structuredContent": { "path": r.path, "truncated": truncated },
                                                            "isError": false
                                                        });
                                                        jsonrpc_ok(id.clone(), out)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                McpMode::Local { root } => match safe_join(root, rel) {
                                    Err(e) => {
                                        jsonrpc_ok(id.clone(), tool_error_result(e.to_string()))
                                    }
                                    Ok(full) => match tokio::fs::read(&full).await {
                                        Err(e) => {
                                            jsonrpc_ok(id.clone(), tool_error_result(e.to_string()))
                                        }
                                        Ok(data) => {
                                            let truncated = data.len() > max_bytes;
                                            let slice = if truncated {
                                                &data[..max_bytes]
                                            } else {
                                                &data[..]
                                            };
                                            match std::str::from_utf8(slice) {
                                                Ok(text) => {
                                                    let out = serde_json::json!({
                                                        "content": [{ "type": "text", "text": text }],
                                                        "structuredContent": { "path": rel, "truncated": truncated },
                                                        "isError": false
                                                    });
                                                    jsonrpc_ok(id.clone(), out)
                                                }
                                                Err(_) => jsonrpc_ok(
                                                    id.clone(),
                                                    tool_error_result(
                                                        "file is not valid UTF-8".into(),
                                                    ),
                                                ),
                                            }
                                        }
                                    },
                                },
                            }
                        }
                    }
                    "fs_search" => {
                        let q = args
                            .get("q")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if q.trim().is_empty() {
                            jsonrpc_ok(id.clone(), tool_error_result("missing q".into()))
                        } else {
                            let max_matches =
                                args.get("max_matches")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(200) as usize;
                            match &mode {
                                McpMode::Hostd {
                                    sock_path,
                                    run_id,
                                    actor,
                                } => {
                                    let path = format!(
                                        "/runs/{}/fs/search?q={}&actor={}",
                                        percent_encode_query_value(run_id),
                                        percent_encode_query_value(&q),
                                        percent_encode_query_value(actor)
                                    );
                                    match get_unix(sock_path, &path).await {
                                        Err(e) => jsonrpc_ok(
                                            id.clone(),
                                            tool_error_result(format!("hostd request failed: {e}")),
                                        ),
                                        Ok((status, body)) => {
                                            if status != StatusCode::OK {
                                                jsonrpc_ok(
                                                    id.clone(),
                                                    tool_error_result(format!(
                                                        "hostd returned {status}: {body}"
                                                    )),
                                                )
                                            } else {
                                                match serde_json::from_str::<HostdSearchResponse>(
                                                    &body,
                                                ) {
                                                    Err(e) => jsonrpc_ok(
                                                        id.clone(),
                                                        tool_error_result(format!(
                                                            "decode response: {e}"
                                                        )),
                                                    ),
                                                    Ok(mut r) => {
                                                        if r.matches.len() > max_matches {
                                                            r.matches.truncate(max_matches);
                                                            r.truncated = true;
                                                        }
                                                        let text = r
                                                            .matches
                                                            .iter()
                                                            .map(|m| {
                                                                format!(
                                                                    "{}:{}:{}:{}",
                                                                    m.path,
                                                                    m.line,
                                                                    m.column,
                                                                    m.text
                                                                )
                                                            })
                                                            .collect::<Vec<_>>()
                                                            .join("\n");
                                                        jsonrpc_ok(
                                                            id.clone(),
                                                            serde_json::json!({
                                                                "content": [{ "type": "text", "text": text }],
                                                                "structuredContent": { "q": q, "truncated": r.truncated, "matches": r.matches },
                                                                "isError": false
                                                            }),
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                McpMode::Local { root } => match std::process::Command::new("rg")
                                    .arg("--line-number")
                                    .arg("--column")
                                    .arg("--no-heading")
                                    .arg("--color")
                                    .arg("never")
                                    .arg("--max-count")
                                    .arg(max_matches.to_string())
                                    .arg(&q)
                                    .arg(".")
                                    .current_dir(root)
                                    .output()
                                {
                                    Err(e) => jsonrpc_ok(
                                        id.clone(),
                                        tool_error_result(format!("rg failed: {e}")),
                                    ),
                                    Ok(out) => {
                                        let stdout_s =
                                            String::from_utf8_lossy(&out.stdout).to_string();
                                        let stderr_s =
                                            String::from_utf8_lossy(&out.stderr).to_string();
                                        if !out.status.success() && stdout_s.trim().is_empty() {
                                            // rg exits 1 when no matches; treat as ok.
                                            if out.status.code() != Some(1) {
                                                jsonrpc_ok(
                                                    id.clone(),
                                                    tool_error_result(format!(
                                                        "rg error: {}",
                                                        stderr_s.trim()
                                                    )),
                                                )
                                            } else {
                                                jsonrpc_ok(
                                                    id.clone(),
                                                    tool_text_result(String::new()),
                                                )
                                            }
                                        } else {
                                            jsonrpc_ok(
                                                id.clone(),
                                                serde_json::json!({
                                                    "content": [{ "type": "text", "text": stdout_s.clone() }],
                                                    "structuredContent": { "q": q, "truncated": false },
                                                    "isError": false
                                                }),
                                            )
                                        }
                                    }
                                },
                            }
                        }
                    }
                    "git_status" => match &mode {
                        McpMode::Hostd {
                            sock_path,
                            run_id,
                            actor,
                        } => {
                            let path = format!(
                                "/runs/{}/git/status?actor={}",
                                percent_encode_query_value(run_id),
                                percent_encode_query_value(actor)
                            );
                            match get_unix(sock_path, &path).await {
                                Err(e) => jsonrpc_ok(id.clone(), tool_error_result(e.to_string())),
                                Ok((status, body)) => {
                                    if status != StatusCode::OK {
                                        jsonrpc_ok(
                                            id.clone(),
                                            tool_error_result(format!(
                                                "hostd returned {status}: {body}"
                                            )),
                                        )
                                    } else {
                                        match serde_json::from_str::<HostdGitTextResponse>(&body) {
                                            Err(e) => jsonrpc_ok(
                                                id.clone(),
                                                tool_error_result(format!("decode response: {e}")),
                                            ),
                                            Ok(r) => jsonrpc_ok(
                                                id.clone(),
                                                serde_json::json!({
                                                    "content": [{ "type": "text", "text": r.stdout }],
                                                    "structuredContent": { "truncated": r.truncated },
                                                    "isError": false
                                                }),
                                            ),
                                        }
                                    }
                                }
                            }
                        }
                        McpMode::Local { root } => {
                            let out = std::process::Command::new("git")
                                .arg("status")
                                .arg("--porcelain=v1")
                                .arg("-b")
                                .current_dir(root)
                                .output()
                                .context("git status")?;
                            let stdout_s = String::from_utf8_lossy(&out.stdout).to_string();
                            let stderr_s = String::from_utf8_lossy(&out.stderr).to_string();
                            if !out.status.success() {
                                jsonrpc_ok(
                                    id.clone(),
                                    tool_error_result(format!(
                                        "git status failed: {}",
                                        stderr_s.trim()
                                    )),
                                )
                            } else {
                                jsonrpc_ok(id.clone(), tool_text_result(stdout_s))
                            }
                        }
                    },
                    "git_diff" => {
                        let rel = args.get("path").and_then(|v| v.as_str());
                        match &mode {
                            McpMode::Hostd {
                                sock_path,
                                run_id,
                                actor,
                            } => {
                                let path = match rel {
                                    Some(p) if !p.trim().is_empty() => format!(
                                        "/runs/{}/git/diff?path={}&actor={}",
                                        percent_encode_query_value(run_id),
                                        percent_encode_query_value(p),
                                        percent_encode_query_value(actor)
                                    ),
                                    _ => format!(
                                        "/runs/{}/git/diff?actor={}",
                                        percent_encode_query_value(run_id),
                                        percent_encode_query_value(actor)
                                    ),
                                };
                                match get_unix(sock_path, &path).await {
                                    Err(e) => {
                                        jsonrpc_ok(id.clone(), tool_error_result(e.to_string()))
                                    }
                                    Ok((status, body)) => {
                                        if status != StatusCode::OK {
                                            jsonrpc_ok(
                                                id.clone(),
                                                tool_error_result(format!(
                                                    "hostd returned {status}: {body}"
                                                )),
                                            )
                                        } else {
                                            match serde_json::from_str::<HostdGitTextResponse>(
                                                &body,
                                            ) {
                                                Err(e) => jsonrpc_ok(
                                                    id.clone(),
                                                    tool_error_result(format!(
                                                        "decode response: {e}"
                                                    )),
                                                ),
                                                Ok(r) => jsonrpc_ok(
                                                    id.clone(),
                                                    serde_json::json!({
                                                        "content": [{ "type": "text", "text": r.stdout }],
                                                        "structuredContent": { "truncated": r.truncated },
                                                        "isError": false
                                                    }),
                                                ),
                                            }
                                        }
                                    }
                                }
                            }
                            McpMode::Local { root } => {
                                if let Some(p) = rel {
                                    if !is_rel_path(p) {
                                        jsonrpc_ok(
                                            id.clone(),
                                            tool_error_result("path must be relative".into()),
                                        )
                                    } else {
                                        let mut cmd = std::process::Command::new("git");
                                        cmd.arg("diff");
                                        cmd.arg("--").arg(p);
                                        let out =
                                            cmd.current_dir(root).output().context("git diff")?;
                                        let stdout_s =
                                            String::from_utf8_lossy(&out.stdout).to_string();
                                        let stderr_s =
                                            String::from_utf8_lossy(&out.stderr).to_string();
                                        if !out.status.success() {
                                            jsonrpc_ok(
                                                id.clone(),
                                                tool_error_result(format!(
                                                    "git diff failed: {}",
                                                    stderr_s.trim()
                                                )),
                                            )
                                        } else {
                                            jsonrpc_ok(id.clone(), tool_text_result(stdout_s))
                                        }
                                    }
                                } else {
                                    let mut cmd = std::process::Command::new("git");
                                    cmd.arg("diff");
                                    let out = cmd.current_dir(root).output().context("git diff")?;
                                    let stdout_s = String::from_utf8_lossy(&out.stdout).to_string();
                                    let stderr_s = String::from_utf8_lossy(&out.stderr).to_string();
                                    if !out.status.success() {
                                        jsonrpc_ok(
                                            id.clone(),
                                            tool_error_result(format!(
                                                "git diff failed: {}",
                                                stderr_s.trim()
                                            )),
                                        )
                                    } else {
                                        jsonrpc_ok(id.clone(), tool_text_result(stdout_s))
                                    }
                                }
                            }
                        }
                    }
                    "fs_write" => match &mode {
                        McpMode::Hostd {
                            sock_path,
                            run_id,
                            actor,
                        } => {
                            let rel = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                            let content =
                                args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                            if rel.trim().is_empty() {
                                jsonrpc_ok(id.clone(), tool_error_result("missing path".into()))
                            } else {
                                let req = HostdWriteFileRequest {
                                    path: rel.to_string(),
                                    content: content.to_string(),
                                    actor: actor.clone(),
                                };
                                let path = format!(
                                    "/runs/{}/fs/write",
                                    percent_encode_query_value(run_id)
                                );
                                match post_json_unix(sock_path, &path, &req).await {
                                    Err(e) => {
                                        jsonrpc_ok(id.clone(), tool_error_result(e.to_string()))
                                    }
                                    Ok((status, body)) => {
                                        if status != StatusCode::OK {
                                            jsonrpc_ok(
                                                id.clone(),
                                                tool_error_result(format!(
                                                    "hostd returned {status}: {body}"
                                                )),
                                            )
                                        } else {
                                            match serde_json::from_str::<HostdWriteFileResponse>(
                                                &body,
                                            ) {
                                                Err(e) => jsonrpc_ok(
                                                    id.clone(),
                                                    tool_error_result(format!(
                                                        "decode response: {e}"
                                                    )),
                                                ),
                                                Ok(r) => {
                                                    let text = format!(
                                                        "wrote {} bytes to {} (truncated={})",
                                                        r.bytes_written, r.path, r.truncated
                                                    );
                                                    jsonrpc_ok(id.clone(), tool_text_result(text))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        McpMode::Local { .. } => jsonrpc_ok(
                            id.clone(),
                            tool_error_result("fs_write is only available in hostd mode".into()),
                        ),
                    },
                    "bash" => match &mode {
                        McpMode::Hostd {
                            sock_path,
                            run_id,
                            actor,
                        } => {
                            let cmd = args.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
                            if cmd.trim().is_empty() {
                                jsonrpc_ok(id.clone(), tool_error_result("missing cmd".into()))
                            } else {
                                let req = HostdBashRequest {
                                    cmd: cmd.to_string(),
                                    actor: actor.clone(),
                                };
                                let path =
                                    format!("/runs/{}/bash", percent_encode_query_value(run_id));
                                match post_json_unix(sock_path, &path, &req).await {
                                    Err(e) => {
                                        jsonrpc_ok(id.clone(), tool_error_result(e.to_string()))
                                    }
                                    Ok((status, body)) => {
                                        if status != StatusCode::OK {
                                            jsonrpc_ok(
                                                id.clone(),
                                                tool_error_result(format!(
                                                    "hostd returned {status}: {body}"
                                                )),
                                            )
                                        } else {
                                            match serde_json::from_str::<HostdBashResponse>(&body) {
                                                Err(e) => jsonrpc_ok(
                                                    id.clone(),
                                                    tool_error_result(format!(
                                                        "decode response: {e}"
                                                    )),
                                                ),
                                                Ok(r) => {
                                                    let text = format!(
                                                        "exit_code: {}\n--- stdout ---\n{}\n--- stderr ---\n{}\n(truncated={})",
                                                        r.exit_code,
                                                        r.stdout,
                                                        r.stderr,
                                                        r.truncated
                                                    );
                                                    jsonrpc_ok(id.clone(), tool_text_result(text))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        McpMode::Local { .. } => jsonrpc_ok(
                            id.clone(),
                            tool_error_result("bash is only available in hostd mode".into()),
                        ),
                    },
                    _ => jsonrpc_err(id, -32601, "unknown tool"),
                }
            }
            _ => jsonrpc_err(id, -32601, "method not found"),
        };

        stdout.write_all(resp.to_string().as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
    if cmd.is_empty() || cmd == "-h" || cmd == "--help" {
        usage();
    }

    if cmd == "mcp" {
        let root = get_arg(&args, "--root").unwrap_or_else(|| ".".to_string());
        let root = std::path::PathBuf::from(root);
        return run_mcp(root).await;
    }

    let tool = match cmd {
        "codex" | "claude" | "iflow" | "gemini" => cmd,
        _ => usage(),
    };

    let sock = pick_sock(get_arg(&args, "--sock")).await?;
    let cwd = get_arg(&args, "--cwd")
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        });
    let cmdline = get_arg(&args, "--cmd").unwrap_or_else(|| tool.to_string());

    let req = StartRunRequest {
        tool: tool.to_string(),
        cmd: cmdline.trim().to_string(),
        cwd,
    };

    let (status, body) = post_json_unix(&sock, "/runs", &req).await?;
    if status != StatusCode::OK {
        return Err(anyhow::anyhow!("hostd returned {status}: {body}"));
    }

    let parsed: StartRunResponse = serde_json::from_str(&body).context("decode response json")?;
    println!("{}", parsed.run_id);
    Ok(())
}
