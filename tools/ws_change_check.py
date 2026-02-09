#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


CHANGE_BRANCH_RE = re.compile(r"^(change|changes|ws|ws-change)/([a-z0-9]+(?:-[a-z0-9]+)*)$")
CHANGE_ID_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def eprint(msg: str) -> None:
    sys.stderr.write(msg + "\n")


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while True:
            b = f.read(1024 * 1024)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def git_root(cwd: Path) -> Optional[Path]:
    try:
        root = subprocess.check_output(
            ["git", "-C", str(cwd), "rev-parse", "--show-toplevel"],
            text=True,
        ).strip()
        if root:
            return Path(root).resolve()
    except Exception:
        return None
    return None


def current_branch(root: Path) -> Optional[str]:
    try:
        b = subprocess.check_output(
            ["git", "-C", str(root), "symbolic-ref", "--quiet", "--short", "HEAD"],
            text=True,
        ).strip()
        return b or None
    except Exception:
        return None


def infer_change_id_from_branch(branch: Optional[str]) -> Optional[str]:
    if not branch:
        return None
    m = CHANGE_BRANCH_RE.match(branch)
    if not m:
        return None
    return m.group(2)


def has_truth_files(root: Path) -> Tuple[bool, List[str]]:
    required = ["AI_PROJECT.md", "AI_WORKSPACE.md", "REQUIREMENTS.md"]
    missing = [f for f in required if not (root / f).exists()]
    return (len(missing) == 0, missing)


def extract_id(label: str, text: str) -> str:
    m = re.search(rf"(?m)^.*{re.escape(label)}.*?[:=]\s*(.+)$", text)
    if not m:
        return ""
    v = m.group(1).strip()
    v = re.sub(r"<!--.*?-->", "", v).strip()
    v = v.strip("`").strip()
    return v


def csv_has_id(path: Path, column: str, value: str) -> bool:
    with path.open("r", encoding="utf-8", errors="replace", newline="") as f:
        reader = csv.DictReader(f)
        if not reader.fieldnames:
            return False
        if column in reader.fieldnames:
            for row in reader:
                if (row.get(column) or "").strip() == value:
                    return True
            return False
        for row in reader:
            for cell in row.values():
                if (cell or "").strip() == value:
                    return True
        return False


def truth_snapshot(root: Path) -> Dict[str, Dict[str, Any]]:
    truth_files = ["AI_PROJECT.md", "AI_WORKSPACE.md", "REQUIREMENTS.md"]
    out: Dict[str, Dict[str, Any]] = {}
    for rel in truth_files:
        p = root / rel
        if not p.exists():
            continue
        st = p.stat()
        out[rel] = {"mtime": int(st.st_mtime), "sha256": sha256(p)}
    return out


def validate_change(
    *,
    root: Path,
    change_id: str,
    strict: bool,
    allow_truth_drift: bool,
) -> int:
    change_dir = root / "changes" / change_id
    required_files = ["proposal.md", "tasks.md"]

    errors: List[str] = []
    warnings: List[str] = []

    def file_state(rel: str) -> Optional[Path]:
        p = change_dir / rel
        if not p.exists():
            errors.append(f"missing: {rel}")
            return None
        if p.stat().st_size == 0:
            errors.append(f"empty: {rel}")
            return None
        return p

    proposal_path = file_state("proposal.md")
    tasks_path = file_state("tasks.md")

    meta_path = change_dir / ".ws-change.json"
    meta: Optional[Dict[str, Any]] = None
    if not meta_path.exists():
        (errors if strict else warnings).append("missing: .ws-change.json (created by aiws change new / aiws change sync)")
    else:
        try:
            meta = json.loads(read_text(meta_path))
        except Exception as e:
            errors.append(f"invalid .ws-change.json: {e}")

    if meta:
        created_truth = meta.get("base_truth_files") or {}
        synced_truth = meta.get("synced_truth_files") or {}
        baseline = synced_truth if synced_truth else created_truth
        baseline_label = "last sync" if synced_truth else "creation"
        baseline_at = meta.get("synced_at") if synced_truth else meta.get("created_at")

        for rel, base in (baseline or {}).items():
            p = root / rel
            if not p.exists():
                (errors if strict else warnings).append(f"truth file missing now: {rel} (baseline={baseline_label})")
                continue
            try:
                cur_sha = sha256(p)
            except Exception as e:
                warnings.append(f"failed to hash truth file {rel}: {e}")
                continue
            base_sha = (base or {}).get("sha256") if isinstance(base, dict) else None
            if base_sha and cur_sha != base_sha:
                msg = (
                    f"truth file changed since {baseline_label}: {rel} "
                    f"(baseline_at={baseline_at}, baseline_sha={base_sha}, current_sha={cur_sha})"
                )
                if strict and not allow_truth_drift:
                    errors.append(msg + f"; run `aiws change sync {change_id}` to acknowledge")
                else:
                    warnings.append(msg)

    def placeholder_scan(rel: str, text: str) -> None:
        if "{{CHANGE_ID}}" in text or "{{TITLE}}" in text or "{{CREATED_AT}}" in text:
            errors.append(f"unrendered template placeholders in {rel}")
        if "WS:TODO" in text:
            (errors if strict else warnings).append(f"WS:TODO markers remain in {rel}")

    req_id = ""
    prob_id = ""

    if proposal_path:
        t = read_text(proposal_path)
        placeholder_scan("proposal.md", t)
        if "验证" not in t:
            warnings.append("proposal.md does not mention 验证 (recommended to include reproducible verification)")
        if "AI_WORKSPACE.md" not in t:
            warnings.append("proposal.md does not reference AI_WORKSPACE.md (recommended)")

        req_id = extract_id("Req_ID", t)
        prob_id = extract_id("Problem_ID", t)
        if strict and not (req_id or prob_id):
            errors.append("proposal.md must include a non-empty Req_ID or Problem_ID (attribution)")

        req_csv = root / "requirements" / "requirements-issues.csv"
        prob_csv = root / "issues" / "problem-issues.csv"

        if req_id and req_csv.exists():
            ok = False
            try:
                ok = csv_has_id(req_csv, "Req_ID", req_id)
            except Exception as e:
                warnings.append(f"failed to read requirements/requirements-issues.csv: {e}")
            if not ok:
                (errors if strict else warnings).append(f"Req_ID not found in requirements/requirements-issues.csv: {req_id}")

        if prob_id and prob_csv.exists():
            ok = False
            try:
                ok = csv_has_id(prob_csv, "Problem_ID", prob_id)
            except Exception as e:
                warnings.append(f"failed to read issues/problem-issues.csv: {e}")
            if not ok:
                (errors if strict else warnings).append(f"Problem_ID not found in issues/problem-issues.csv: {prob_id}")

    if tasks_path:
        t = read_text(tasks_path)
        placeholder_scan("tasks.md", t)
        if not re.search(r"(?m)^- \[[ xX]\]", t):
            errors.append("tasks.md has no checkbox tasks ('- [ ]' or '- [x]')")

    design_path = change_dir / "design.md"
    if design_path.exists() and design_path.stat().st_size > 0:
        placeholder_scan("design.md", read_text(design_path))

    for e in errors:
        eprint(f"error: {e}")
    for w in warnings:
        eprint(f"warn: {w}")

    return 2 if errors else 0


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Validate ws change artifacts for hooks/CI.")
    parser.add_argument("--workspace-root", default="", help="Workspace root (defaults to git root).")
    parser.add_argument("--change-id", default="", help="Change id (defaults to infer from branch).")
    parser.add_argument(
        "--branch",
        default="",
        help="Branch name to validate against (for CI detached HEAD). When set, enforces change/<id> naming.",
    )
    parser.add_argument("--strict", action="store_true", help="Treat WS:TODO as errors and require attribution IDs.")
    parser.add_argument(
        "--allow-truth-drift",
        action="store_true",
        help="Do not fail strict validation on truth drift (use only for emergencies).",
    )
    parser.add_argument(
        "--allow-branches",
        default="main,master",
        help="Comma-separated branch names that are exempt (default: main,master).",
    )
    args = parser.parse_args(argv)

    cwd = Path(os.getcwd())
    root = Path(args.workspace_root).resolve() if args.workspace_root else (git_root(cwd) or cwd.resolve())

    ok_truth, missing = has_truth_files(root)
    if not ok_truth:
        # Not an AI Workspace; do not block.
        eprint(f"warn: skip ws-change check (missing truth files): {missing}")
        return 0

    change_id = args.change_id.strip()
    if change_id and not CHANGE_ID_RE.match(change_id):
        eprint(f"error: invalid change id (use kebab-case): {change_id}")
        return 2

    branch_arg = (args.branch or "").strip()
    branch = branch_arg or current_branch(root)
    if not change_id:
        # Detached HEAD during rebase/merge; do not block unless CI passes --branch.
        if not branch:
            eprint("warn: skip ws-change check (detached HEAD; pass --branch in CI to enforce)")
            return 0
        allow = {b.strip() for b in (args.allow_branches or "").split(",") if b.strip()}
        if branch in allow:
            return 0
        inferred = infer_change_id_from_branch(branch)
        if not inferred:
            eprint(f"error: branch must be change/<change-id> (current: {branch})")
            eprint("hint: switch/create: git switch -c change/<change-id>")
            return 2
        change_id = inferred
    else:
        # If CI provides --branch, cross-check branch naming even when --change-id is provided.
        if branch_arg:
            allow = {b.strip() for b in (args.allow_branches or "").split(",") if b.strip()}
            if branch_arg not in allow:
                inferred = infer_change_id_from_branch(branch_arg)
                if not inferred:
                    eprint(f"error: branch must be change/<change-id> (current: {branch_arg})")
                    return 2
                if inferred != change_id:
                    eprint(f"error: change-id does not match branch (branch={branch_arg}, change_id={change_id})")
                    return 2

    change_dir = root / "changes" / change_id
    if not change_dir.exists():
        eprint(f"error: missing change dir: {change_dir}")
        eprint(f"hint: create: aiws change new {change_id} --no-design")
        return 2

    return validate_change(
        root=root,
        change_id=change_id,
        strict=args.strict,
        allow_truth_drift=args.allow_truth_drift,
    )


if __name__ == "__main__":
    raise SystemExit(main())
