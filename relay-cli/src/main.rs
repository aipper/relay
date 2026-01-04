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

    let json = serde_json::to_vec(body).context("encode json")?;
    let req = Request::builder()
        .method("POST")
        .uri(format!("http://localhost{path}"))
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(json)))
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

fn relay_home_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(std::path::PathBuf::from(home).join(".relay"))
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
    jsonrpc: Option<String>,
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

fn tool_list_result() -> JsonValue {
    serde_json::json!({
        "tools": [
            {
                "name": "fs_read",
                "description": "Read a UTF-8 text file under the configured root (relative path only).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Relative file path" },
                        "max_bytes": { "type": "integer", "description": "Optional max bytes (default 1048576)" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "fs_search",
                "description": "Search for a pattern under root using ripgrep (rg).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "q": { "type": "string", "description": "Search query" },
                        "max_matches": { "type": "integer", "description": "Optional max matches (default 200)" }
                    },
                    "required": ["q"]
                }
            },
            {
                "name": "git_status",
                "description": "Run `git status --porcelain=v1 -b` under root.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "git_diff",
                "description": "Run `git diff` under root (optional relative path).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Optional relative file path" }
                    }
                }
            }
        ]
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

async fn run_mcp(root: std::path::PathBuf) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut lines = tokio::io::BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

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
                jsonrpc_ok(
                    id.clone(),
                    serde_json::json!({
                        "protocolVersion": "2025-06-18",
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "relay-mcp", "version": env!("CARGO_PKG_VERSION") },
                        "instructions": "Tools are restricted to the configured root directory. Paths must be relative."
                    }),
                )
            }
            "tools/list" => jsonrpc_ok(id.clone(), tool_list_result()),
            "tools/call" => {
                let params = req.params.unwrap_or(JsonValue::Null);
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(JsonValue::Null);
                match name {
                    "fs_read" => {
                        let rel = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let max_bytes =
                            args.get("max_bytes")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(1024 * 1024) as usize;
                        match safe_join(&root, rel) {
                            Err(e) => jsonrpc_ok(id.clone(), tool_error_result(e.to_string())),
                            Ok(full) => match tokio::fs::read(&full).await {
                                Err(e) => jsonrpc_ok(id.clone(), tool_error_result(e.to_string())),
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
                                            tool_error_result("file is not valid UTF-8".into()),
                                        ),
                                    }
                                }
                            },
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
                            match std::process::Command::new("rg")
                                .arg("--line-number")
                                .arg("--column")
                                .arg("--no-heading")
                                .arg("--color")
                                .arg("never")
                                .arg("--max-count")
                                .arg(max_matches.to_string())
                                .arg(&q)
                                .arg(".")
                                .current_dir(&root)
                                .output()
                            {
                                Err(e) => jsonrpc_ok(
                                    id.clone(),
                                    tool_error_result(format!("rg failed: {e}")),
                                ),
                                Ok(out) => {
                                    let stdout_s = String::from_utf8_lossy(&out.stdout).to_string();
                                    let stderr_s = String::from_utf8_lossy(&out.stderr).to_string();
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
                                            jsonrpc_ok(id.clone(), tool_text_result(String::new()))
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
                            }
                        }
                    }
                    "git_status" => {
                        let out = std::process::Command::new("git")
                            .arg("status")
                            .arg("--porcelain=v1")
                            .arg("-b")
                            .current_dir(&root)
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
                    "git_diff" => {
                        let rel = args.get("path").and_then(|v| v.as_str());
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
                                let out = cmd.current_dir(&root).output().context("git diff")?;
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
                        } else {
                            let mut cmd = std::process::Command::new("git");
                            cmd.arg("diff");
                            let out = cmd.current_dir(&root).output().context("git diff")?;
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
