#!/usr/bin/env python3
"""Collect OneLink integration readiness evidence for pre-launch closure."""

from __future__ import annotations

import argparse
import json
import os
import socket
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import urlparse


@dataclass
class RedisProbeResult:
    reachable: bool
    status: str


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError:
        return {}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Collect integration readiness evidence for OneLink pre-launch."
    )
    parser.add_argument("--workspace-root", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--output-md", required=True)
    parser.add_argument("--codemaster-json")
    parser.add_argument("--codemaster-md")
    parser.add_argument("--scope", default="prelaunch_platform_closeout")
    return parser.parse_args()


def probe_redis(redis_url: str) -> RedisProbeResult:
    parsed = urlparse(redis_url)
    host = parsed.hostname or "127.0.0.1"
    port = parsed.port or 6379
    try:
        with socket.create_connection((host, port), timeout=1.5) as conn:
            conn.sendall(b"*1\r\n$4\r\nPING\r\n")
            response = conn.recv(128)
        if response.startswith(b"+PONG"):
            return RedisProbeResult(True, f"{host}:{port} reachable (PING/PONG)")
        return RedisProbeResult(False, f"{host}:{port} unexpected response {response!r}")
    except OSError as exc:
        return RedisProbeResult(False, f"{host}:{port} unreachable ({exc})")


def format_markdown(payload: dict[str, Any]) -> str:
    missing_modules = payload.get("missing_modules") or []
    reasons = payload.get("reasons") or []
    lines = [
        "# Integration Readiness",
        "",
        f"- 当前周期: {payload['current_cycle']}",
        f"- 当前主目标: {payload['current_objective']}",
        f"- scope: {payload['scope']}",
        f"- integration enabled: {'yes' if payload['integration_enabled'] else 'no'}",
        f"- backend python: {payload['backend_python']}",
        f"- redis url: {payload['redis_url']}",
        f"- redis reachable: {'yes' if payload['redis_reachable'] else 'no'}",
        f"- redis status: {payload['redis_status']}",
        f"- ready: {'yes' if payload['ready'] else 'no'}",
        f"- 写入时间: {payload['written_at']}",
        "",
        "## Missing Modules",
    ]
    if missing_modules:
        lines.extend(f"- {module}" for module in missing_modules)
    else:
        lines.append("- none")
    lines.extend(["", "## Reasons"])
    if reasons:
        lines.extend(f"- {reason}" for reason in reasons)
    else:
        lines.append("- none")
    return "\n".join(lines) + "\n"


def main() -> int:
    args = parse_args()
    workspace_root = Path(args.workspace_root).resolve()
    output_json = Path(args.output_json).resolve()
    output_md = Path(args.output_md).resolve()
    codemaster_json = Path(args.codemaster_json).resolve() if args.codemaster_json else None
    codemaster_md = Path(args.codemaster_md).resolve() if args.codemaster_md else None

    previous = load_json(
        workspace_root / "reports" / "codemaster" / "integration_readiness.json"
    )
    owner_brief = load_json(workspace_root / "reports" / "codemaster" / "project_owner_brief.json")

    integration_enabled = os.environ.get("SKYLINE_RUN_INTEGRATION") == "1"
    redis_url = os.environ.get("REDIS_URL", "redis://127.0.0.1:6379/15")
    redis_probe = probe_redis(redis_url)

    reasons: list[str] = []
    if not integration_enabled:
        reasons.append("Set SKYLINE_RUN_INTEGRATION=1")
    if not redis_probe.reachable:
        reasons.append(f"redis not ready: {redis_probe.status}")

    payload = {
        "kind": "codemaster_project_integration_readiness",
        "workspace_root": str(workspace_root),
        "current_cycle": owner_brief.get(
            "current_cycle", previous.get("current_cycle", "unknown_cycle")
        ),
        "current_objective": owner_brief.get(
            "current_objective", previous.get("current_objective", "unknown_objective")
        ),
        "scope": args.scope,
        "integration_enabled": integration_enabled,
        "backend_python": previous.get("backend_python") or os.environ.get("PYTHON", "python3"),
        "redis_url": redis_url,
        "missing_modules": [],
        "redis_reachable": redis_probe.reachable,
        "redis_status": redis_probe.status,
        "ready": integration_enabled and redis_probe.reachable,
        "reasons": reasons,
        "written_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat(),
    }

    output_json.parent.mkdir(parents=True, exist_ok=True)
    output_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
    output_md.write_text(format_markdown(payload))

    if codemaster_json and codemaster_md:
        codemaster_json.parent.mkdir(parents=True, exist_ok=True)
        codemaster_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
        codemaster_md.write_text(format_markdown(payload))

    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
