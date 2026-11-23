#!/usr/bin/env python3
"""
Automated upstream T3 Code feature & protocol tracker for LingXiT3-Native.
Never auto-rebases TypeScript code; generates structured Rust porting tasks instead.
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request
from pathlib import Path

UPSTREAM_COMMITS_API = "https://api.github.com/repos/pingdotgg/t3code/commits?per_page=20"
LINGXI_COMMITS_API = "https://api.github.com/repos/LingXiFox/t3code/commits?sha=lingxifox&per_page=20"
BASELINE_SHA = "5e8e265d1e4571187f2777b20c92074e9157ce3f"


def fetch_commits(url: str) -> list[dict]:
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "LingXiT3-Native-Upstream-Tracker", "Accept": "application/vnd.github+json"},
    )
    with urllib.request.urlopen(req, timeout=15) as resp:
        return json.loads(resp.read().decode("utf-8"))


def classify_commit(message: str) -> tuple[str, str]:
    first_line = message.splitlines()[0].strip()
    lower = first_line.lower()
    if lower.startswith("feat"):
        return ("Feature", "Evaluate UI/domain parity and add Rust implementation + unit test")
    if lower.startswith("fix"):
        return ("Bugfix", "Check whether equivalent Rust crate has the bug and add regression test")
    if any(k in lower for k in ("contract", "rpc", "acp", "orchestration", "provider")):
        return ("Protocol/Provider", "Review `crates/domain`, `crates/acp`, `crates/providers`, or `crates/remote` compatibility")
    return ("Maintenance", "Inspect only if it changes runtime behavior or user-facing workflows")


def render_report(upstream: list[dict], fork: list[dict], matrix_path: Path) -> str:
    matrix_lines = matrix_path.read_text(encoding="utf-8").splitlines() if matrix_path.exists() else []
    tracked_count = sum(1 for ln in matrix_lines if ln.startswith("| F-"))

    out = [
        "# T3 Code 上游功能追踪审计报告",
        "",
        f"- **基线 Commit SHA (`LingXiFox/t3code`)**: `{BASELINE_SHA}`",
        f"- **本地迁移验收矩阵条目数**: `{tracked_count}`",
        "",
        "## 1. 上游 `pingdotgg/t3code` 最新提交分析",
        "",
        "| SHA | 分类 | 提交标题 | Rust 移植建议 |",
        "| --- | --- | --- | --- |",
    ]
    for c in upstream[:15]:
        sha = c.get("sha", "")[:10]
        msg = c.get("commit", {}).get("message", "")
        title = msg.splitlines()[0].replace("|", "\\|")
        kind, advice = classify_commit(msg)
        out.append(f"| `{sha}` | {kind} | {title} | {advice} |")

    out.extend([
        "",
        "## 2. `LingXiFox/t3code` (`lingxifox`) 最新提交",
        "",
        "| SHA | 分类 | 提交标题 |",
        "| --- | --- | --- |",
    ])
    for c in fork[:10]:
        sha = c.get("sha", "")[:10]
        msg = c.get("commit", {}).get("message", "")
        title = msg.splitlines()[0].replace("|", "\\|")
        kind, _ = classify_commit(msg)
        out.append(f"| `{sha}` | {kind} | {title} |")

    return "\n".join(out) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="Audit upstream T3 Code changes for Rust porting.")
    parser.add_argument("--dry-run", action="store_true", help="Print report to stdout without writing files.")
    parser.add_argument("--out", type=Path, default=None, help="Output markdown file path.")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    matrix_path = repo_root / "docs" / "audit" / "feature-matrix.md"

    try:
        upstream = fetch_commits(UPSTREAM_COMMITS_API)
        fork = fetch_commits(LINGXI_COMMITS_API)
    except urllib.error.URLError as exc:
        print(f"Warning: could not reach GitHub API ({exc}); skipping remote fetch.", file=sys.stderr)
        upstream, fork = [], []

    report = render_report(upstream, fork, matrix_path)
    if args.dry_run or args.out is None:
        print(report)
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(report, encoding="utf-8")
        print(f"Wrote upstream audit report to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
