use axum::http::StatusCode;

fn reject_unsafe_rel_path(rel: &str) -> Result<(), (StatusCode, String)> {
    if rel.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "missing path".into()));
    }
    if std::path::Path::new(rel).is_absolute() {
        return Err((StatusCode::BAD_REQUEST, "path must be relative".into()));
    }
    let p = std::path::Path::new(rel);
    for c in p.components() {
        match c {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err((StatusCode::FORBIDDEN, "path contains ..".into()));
            }
            _ => return Err((StatusCode::BAD_REQUEST, "invalid path".into())),
        }
    }
    Ok(())
}

pub fn safe_join_run_path(
    run_cwd: &str,
    rel: &str,
) -> Result<std::path::PathBuf, (StatusCode, String)> {
    let base = std::path::Path::new(run_cwd);
    reject_unsafe_rel_path(rel)?;

    let joined = base.join(rel);
    let base_can = std::fs::canonicalize(base)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("bad run cwd: {e}")))?;
    let joined_can = std::fs::canonicalize(&joined)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("bad path: {e}")))?;
    if !joined_can.starts_with(&base_can) {
        return Err((StatusCode::FORBIDDEN, "path escapes run cwd".into()));
    }
    Ok(joined_can)
}

pub fn safe_join_run_path_allow_create(
    run_cwd: &str,
    rel: &str,
) -> Result<std::path::PathBuf, (StatusCode, String)> {
    let base = std::path::Path::new(run_cwd);
    reject_unsafe_rel_path(rel)?;

    let joined = base.join(rel);
    let base_can = std::fs::canonicalize(base)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("bad run cwd: {e}")))?;

    if joined.exists() {
        let joined_can = std::fs::canonicalize(&joined)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("bad path: {e}")))?;
        if !joined_can.starts_with(&base_can) {
            return Err((StatusCode::FORBIDDEN, "path escapes run cwd".into()));
        }
        return Ok(joined_can);
    }

    let parent = joined.parent().unwrap_or(base);
    let parent_can = std::fs::canonicalize(parent)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("bad path: {e}")))?;
    if !parent_can.starts_with(&base_can) {
        return Err((StatusCode::FORBIDDEN, "path escapes run cwd".into()));
    }

    let file_name = joined
        .file_name()
        .ok_or((StatusCode::BAD_REQUEST, "missing file name".into()))?;
    Ok(parent_can.join(file_name))
}

pub fn read_utf8_file(
    run_cwd: &str,
    rel_path: &str,
    max_bytes: usize,
) -> Result<(String, bool), (StatusCode, String)> {
    let path = safe_join_run_path(run_cwd, rel_path)?;
    let bytes = std::fs::read(&path).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let truncated = bytes.len() > max_bytes;
    let slice = if truncated {
        &bytes[..max_bytes]
    } else {
        &bytes[..]
    };
    let content = String::from_utf8(slice.to_vec())
        .map_err(|_| (StatusCode::BAD_REQUEST, "file is not valid utf-8".into()))?;
    Ok((content, truncated))
}

pub fn write_utf8_file(
    run_cwd: &str,
    rel_path: &str,
    content: &str,
    max_bytes: usize,
) -> Result<(i64, bool), (StatusCode, String)> {
    let bytes = content.as_bytes();
    let truncated = bytes.len() > max_bytes;
    let bytes_to_write = if truncated {
        &bytes[..max_bytes]
    } else {
        bytes
    };

    let path = safe_join_run_path_allow_create(run_cwd, rel_path)?;
    if path.is_dir() {
        return Err((StatusCode::BAD_REQUEST, "path is a directory".into()));
    }
    std::fs::write(&path, bytes_to_write).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok((bytes_to_write.len() as i64, truncated))
}

pub fn has_cmd(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .output()
        .is_ok()
}

pub fn rg_search(
    run_cwd: &str,
    q: &str,
    max_matches: usize,
) -> Result<(Vec<(String, i64, i64, String)>, bool), (StatusCode, String)> {
    if q.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "missing q".into()));
    }
    if !has_cmd("rg") {
        return Err((StatusCode::NOT_IMPLEMENTED, "missing dependency: rg".into()));
    }

    let out = std::process::Command::new("rg")
        .current_dir(run_cwd)
        .args([
            "--line-number",
            "--column",
            "--no-heading",
            "--color",
            "never",
            "--max-count",
            &max_matches.to_string(),
        ])
        .arg(q)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !out.status.success() && out.status.code() != Some(1) {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        return Err((StatusCode::BAD_REQUEST, err));
    }

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let mut matches = Vec::new();
    for line in stdout.lines() {
        let Some((path, rest)) = line.split_once(':') else {
            continue;
        };
        let Some((line_no, rest)) = rest.split_once(':') else {
            continue;
        };
        let Some((col_no, text)) = rest.split_once(':') else {
            continue;
        };
        let line = line_no.parse::<i64>().unwrap_or(0);
        let column = col_no.parse::<i64>().unwrap_or(0);
        matches.push((path.to_string(), line, column, text.to_string()));
        if matches.len() >= max_matches {
            break;
        }
    }
    let truncated = matches.len() >= max_matches;
    Ok((matches, truncated))
}

pub fn git_status(run_cwd: &str, max_chars: usize) -> Result<(String, bool), (StatusCode, String)> {
    let out = std::process::Command::new("git")
        .current_dir(run_cwd)
        .args(["status", "--porcelain=v1", "-b"])
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        return Err((StatusCode::BAD_REQUEST, err));
    }

    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let truncated = s.len() > max_chars;
    let stdout = if truncated {
        s[s.len() - max_chars..].to_string()
    } else {
        s
    };
    Ok((stdout, truncated))
}

pub fn git_diff(
    run_cwd: &str,
    rel_path: Option<&str>,
    max_chars: usize,
) -> Result<(String, bool), (StatusCode, String)> {
    if let Some(p) = rel_path {
        let _ = safe_join_run_path(run_cwd, p)?;
    }

    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(run_cwd).args(["diff", "--no-color"]);
    if let Some(p) = rel_path {
        cmd.arg("--").arg(p);
    }
    let out = cmd
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        return Err((StatusCode::BAD_REQUEST, err));
    }

    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let truncated = s.len() > max_chars;
    let stdout = if truncated {
        s[s.len() - max_chars..].to_string()
    } else {
        s
    };
    Ok((stdout, truncated))
}

pub fn list_dir(
    run_cwd: &str,
    rel_path: &str,
    max_entries: usize,
) -> Result<(Vec<(String, bool, Option<i64>)>, bool), (StatusCode, String)> {
    let rel_path = if rel_path.trim().is_empty() {
        "."
    } else {
        rel_path
    };
    let path = safe_join_run_path(run_cwd, rel_path)?;
    let md = std::fs::metadata(&path).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if !md.is_dir() {
        return Err((StatusCode::BAD_REQUEST, "path is not a directory".into()));
    }

    let mut out = Vec::<(String, bool, Option<i64>)>::new();
    let mut truncated = false;
    let entries = std::fs::read_dir(&path).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    for e in entries {
        let e = e.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        let name = e.file_name().to_string_lossy().to_string();
        let md = e.metadata().ok();
        let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = md.as_ref().and_then(|m| {
            if m.is_file() {
                Some(m.len() as i64)
            } else {
                None
            }
        });
        out.push((name, is_dir, size));
        if out.len() >= max_entries {
            truncated = true;
            break;
        }
    }
    Ok((out, truncated))
}

pub fn bash_exec(
    run_cwd: &str,
    cmd: &str,
    max_stdout_chars: usize,
    max_stderr_chars: usize,
) -> Result<(String, String, i64, bool), (StatusCode, String)> {
    if cmd.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "missing cmd".into()));
    }

    let out = std::process::Command::new("bash")
        .current_dir(run_cwd)
        .arg("-lc")
        .arg(cmd)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let exit_code = out.status.code().unwrap_or(-1) as i64;

    let stdout_raw = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr_raw = String::from_utf8_lossy(&out.stderr).to_string();
    let stdout_truncated = stdout_raw.len() > max_stdout_chars;
    let stderr_truncated = stderr_raw.len() > max_stderr_chars;
    let stdout = if stdout_truncated {
        stdout_raw[stdout_raw.len() - max_stdout_chars..].to_string()
    } else {
        stdout_raw
    };
    let stderr = if stderr_truncated {
        stderr_raw[stderr_raw.len() - max_stderr_chars..].to_string()
    } else {
        stderr_raw
    };
    let truncated = stdout_truncated || stderr_truncated;

    if !out.status.success() {
        let mut msg = format!("bash exited with code {exit_code}");
        if !stderr.trim().is_empty() {
            msg.push_str(": ");
            msg.push_str(&stderr);
        }
        return Err((StatusCode::BAD_REQUEST, msg));
    }

    Ok((stdout, stderr, exit_code, truncated))
}
