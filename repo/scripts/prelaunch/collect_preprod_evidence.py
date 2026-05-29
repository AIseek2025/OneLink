#!/usr/bin/env python3
"""Collect real pre-prod evidence inputs and probe results for OneLink."""

from __future__ import annotations

import argparse
import json
import os
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib import error, request
from urllib.parse import urlparse


DEFAULT_ENV_MAP = {
    "preprod_url": "ONELINK_PREPROD_URL",
    "web_url": "ONELINK_PREPROD_WEB_URL",
    "bff_base_url": "ONELINK_PREPROD_BFF_BASE_URL",
    "api_base_url": "ONELINK_PREPROD_API_BASE_URL",
    "candidate_version": "ONELINK_PREPROD_VERSION",
    "image_digest": "ONELINK_PREPROD_IMAGE_DIGEST",
    "asset_manifest": "ONELINK_PREPROD_ASSET_MANIFEST",
    "dashboard_url": "ONELINK_PREPROD_DASHBOARD_URL",
    "alert_evidence_url": "ONELINK_PREPROD_ALERT_URL",
    "approval_url": "ONELINK_PREPROD_APPROVAL_URL",
    "release_owner": "ONELINK_PREPROD_RELEASE_OWNER",
    "rollback_owner": "ONELINK_PREPROD_ROLLBACK_OWNER",
    "smoke_user": "ONELINK_PREPROD_SMOKE_USER",
    "smoke_password": "ONELINK_PREPROD_SMOKE_PASSWORD",
}


@dataclass
class ProbeResult:
    name: str
    url: str
    status: str
    http_status: int | None
    detail: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Collect real pre-prod evidence for OneLink pre-launch repair."
    )
    parser.add_argument("--workspace-root", required=True)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--output-md", required=True)
    parser.add_argument("--preprod-url")
    parser.add_argument("--web-url")
    parser.add_argument("--bff-base-url")
    parser.add_argument("--api-base-url")
    parser.add_argument("--candidate-version")
    parser.add_argument("--image-digest")
    parser.add_argument("--asset-manifest")
    parser.add_argument("--dashboard-url")
    parser.add_argument("--alert-evidence-url")
    parser.add_argument("--approval-url")
    parser.add_argument("--release-owner")
    parser.add_argument("--rollback-owner")
    parser.add_argument("--smoke-user")
    parser.add_argument("--smoke-password")
    parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=5.0,
        help="Timeout for HTTP probes. Default: 5 seconds.",
    )
    parser.add_argument(
        "--require-real-preprod",
        action="store_true",
        help=(
            "Reject loopback/file/local placeholder inputs instead of treating any "
            "filled field as real pre-prod evidence."
        ),
    )
    return parser.parse_args()


def pick_value(args: argparse.Namespace, field_name: str) -> str:
    cli_value = getattr(args, field_name)
    if cli_value:
        return cli_value
    env_name = DEFAULT_ENV_MAP[field_name]
    return os.environ.get(env_name, "").strip()


def load_asset_manifest(raw: str) -> dict[str, Any] | None:
    if not raw:
        return None
    path = Path(raw)
    if path.exists():
        try:
            return json.loads(path.read_text())
        except json.JSONDecodeError:
            return {"raw_path": str(path), "raw_text": path.read_text().strip()}
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        return {"raw_value": raw}


def contains_local_placeholder(value: str) -> bool:
    lowered = value.strip().lower()
    if not lowered:
        return False
    tokens = (
        "127.0.0.1",
        "localhost",
        "0.0.0.0",
        "file://",
        "placeholder",
        "temporary_local_shared_env",
        "codemaster-local-owner",
        "local-worktree-no-digest",
        "local-smoke-",
    )
    return any(token in lowered for token in tokens) or lowered.startswith("local-")


def validate_real_url(value: str) -> dict[str, str]:
    if not value:
        return {"status": "missing", "reason": "missing"}
    if value.startswith("file://"):
        return {"status": "invalid_local_placeholder", "reason": "file_url"}
    parsed = urlparse(value)
    hostname = (parsed.hostname or "").lower()
    if hostname in {"127.0.0.1", "localhost", "0.0.0.0"} or hostname.endswith(".local"):
        return {"status": "invalid_local_placeholder", "reason": "loopback_or_local_host"}
    if parsed.scheme and parsed.scheme not in {"http", "https"}:
        return {"status": "invalid_non_http_url", "reason": parsed.scheme}
    if not parsed.scheme:
        return {"status": "invalid_url", "reason": "missing_scheme"}
    return {"status": "pass", "reason": "ok"}


def validate_real_string(value: str) -> dict[str, str]:
    if not value:
        return {"status": "missing", "reason": "missing"}
    if contains_local_placeholder(value):
        return {"status": "invalid_local_placeholder", "reason": "local_placeholder_marker"}
    return {"status": "pass", "reason": "ok"}


def validate_real_asset_manifest(asset_manifest: Any) -> dict[str, str]:
    if asset_manifest is None:
        return {"status": "missing", "reason": "missing"}
    serialized = json.dumps(asset_manifest, ensure_ascii=False, sort_keys=True)
    if contains_local_placeholder(serialized):
        return {"status": "invalid_local_placeholder", "reason": "local_placeholder_marker"}
    return {"status": "pass", "reason": "ok"}


def build_real_preprod_validation(payload: dict[str, Any]) -> list[dict[str, str]]:
    validators = [
        ("preprod_url", "真实 pre-prod URL", validate_real_url(payload["preprod_url"])),
        ("web_url", "真实 Web URL", validate_real_url(payload["web_url"])),
        ("bff_base_url", "真实 BFF/API base URL", validate_real_url(payload["bff_base_url"])),
        ("api_base_url", "真实 API base URL", validate_real_url(payload["api_base_url"])),
        ("candidate_version", "候选版本号", validate_real_string(payload["candidate_version"])),
        ("image_digest", "镜像 digest", validate_real_string(payload["image_digest"])),
        ("asset_manifest", "静态资源版本映射", validate_real_asset_manifest(payload["asset_manifest"])),
        ("dashboard_url", "监控看板链接", validate_real_url(payload["dashboard_url"])),
        ("alert_evidence_url", "告警触发记录", validate_real_url(payload["alert_evidence_url"])),
        ("approval_url", "审批链证据", validate_real_url(payload["approval_url"])),
        ("release_owner", "发布责任人", validate_real_string(payload["release_owner"])),
        ("rollback_owner", "回滚责任人", validate_real_string(payload["rollback_owner"])),
        ("smoke_user", "真实 smoke 账号", validate_real_string(payload["smoke_user"])),
        ("smoke_password", "真实 smoke 密码", validate_real_string(payload["smoke_password_raw"])),
    ]
    return [
        {"field": field, "label": label, "status": result["status"], "reason": result["reason"]}
        for field, label, result in validators
    ]


def http_probe(url: str, timeout_seconds: float) -> ProbeResult:
    req = request.Request(url, method="GET")
    try:
        with request.urlopen(req, timeout=timeout_seconds) as resp:
            body = resp.read(400).decode("utf-8", errors="replace").strip()
            detail = body if body else "empty body"
            return ProbeResult(
                name="",
                url=url,
                status="pass",
                http_status=resp.status,
                detail=detail,
            )
    except error.HTTPError as exc:
        body = exc.read(400).decode("utf-8", errors="replace").strip()
        return ProbeResult(
            name="",
            url=url,
            status="http_error",
            http_status=exc.code,
            detail=body or str(exc),
        )
    except error.URLError as exc:
        return ProbeResult(
            name="",
            url=url,
            status="network_error",
            http_status=None,
            detail=str(exc.reason),
        )
    except OSError as exc:
        return ProbeResult(
            name="",
            url=url,
            status="network_error",
            http_status=None,
            detail=str(exc),
        )


def build_probe_matrix(preprod_url: str, timeout_seconds: float) -> list[ProbeResult]:
    if not preprod_url:
        return []

    base = preprod_url.rstrip("/")
    targets = [
        ("health", f"{base}/health"),
        ("ready", f"{base}/ready"),
        ("metrics", f"{base}/metrics"),
    ]
    results: list[ProbeResult] = []
    for name, url in targets:
        result = http_probe(url, timeout_seconds)
        result.name = name
        results.append(result)
    return results


def summarize_smoke_prerequisites(payload: dict[str, Any]) -> dict[str, Any]:
    missing: list[str] = []
    if not payload["bff_base_url"]:
        missing.append("BFF/API base URL")
    if not payload["smoke_user_present"]:
        missing.append("smoke 账号")
    if not payload["smoke_password_present"]:
        missing.append("smoke 密码")

    if missing:
        return {
            "status": "blocked",
            "missing_prerequisites": missing,
            "detail": "未满足登录态业务 smoke 的真实环境前提。",
        }

    return {
        "status": "ready_for_execution",
        "missing_prerequisites": [],
        "detail": "已具备最小业务 smoke 前提，可继续接入登录/聊天/找人/推荐/DM/安全/后台审核脚本。",
    }


def build_required_field_status(payload: dict[str, Any]) -> list[dict[str, str]]:
    fields = [
        ("preprod_url", "真实 pre-prod URL"),
        ("bff_base_url", "BFF/API base URL"),
        ("candidate_version", "候选版本号"),
        ("image_digest", "镜像 digest"),
        ("dashboard_url", "监控看板链接"),
        ("alert_evidence_url", "告警触发记录"),
        ("approval_url", "审批链证据"),
        ("release_owner", "发布责任人"),
        ("rollback_owner", "回滚责任人"),
    ]
    validation_map = {
        item["field"]: item["status"] for item in payload.get("real_preprod_validation", [])
    }
    statuses: list[dict[str, str]] = []
    for key, label in fields:
        if payload.get("require_real_preprod_validation"):
            status = validation_map.get(key, "missing")
        else:
            status = "present" if payload.get(key) else "missing"
        statuses.append({"field": key, "label": label, "status": status})
    return statuses


def format_markdown(payload: dict[str, Any]) -> str:
    lines = [
        "# Real Pre-Prod Evidence Collection",
        "",
        f"- 采集时间: {payload['written_at']}",
        f"- 采集结论: `{payload['collection_status']}`",
        f"- 作用域: `{payload['scope']}`",
        f"- 严格真实 pre-prod 校验: `{str(payload['require_real_preprod_validation']).lower()}`",
        "",
        "## 资产采集",
        "",
        f"- pre-prod URL: {payload['preprod_url'] or 'missing'}",
        f"- Web URL: {payload['web_url'] or 'missing'}",
        f"- BFF/API base: {payload['bff_base_url'] or 'missing'}",
        f"- API base: {payload['api_base_url'] or 'missing'}",
        f"- 候选版本号: {payload['candidate_version'] or 'missing'}",
        f"- 镜像 digest: {payload['image_digest'] or 'missing'}",
        f"- 监控看板: {payload['dashboard_url'] or 'missing'}",
        f"- 告警触发记录: {payload['alert_evidence_url'] or 'missing'}",
        f"- 审批链 evidence: {payload['approval_url'] or 'missing'}",
        f"- 发布责任人: {payload['release_owner'] or 'missing'}",
        f"- 回滚责任人: {payload['rollback_owner'] or 'missing'}",
        "",
        "## 必要字段状态",
        "",
        "| 字段 | 状态 |",
        "| --- | --- |",
    ]
    for field in payload["required_fields"]:
        lines.append(f"| {field['label']} | `{field['status']}` |")

    if payload["require_real_preprod_validation"]:
        lines.extend(["", "## 真实性校验", "", "| 字段 | 结果 | 原因 |", "| --- | --- | --- |"])
        for field in payload["real_preprod_validation"]:
            lines.append(
                f"| {field['label']} | `{field['status']}` | `{field['reason']}` |"
            )

    lines.extend(["", "## 基础 Probe"])
    if payload["probes"]:
        lines.extend(["", "| Probe | 结果 | HTTP | URL | 详情 |", "| --- | --- | --- | --- | --- |"])
        for probe in payload["probes"]:
            http_status = str(probe["http_status"]) if probe["http_status"] is not None else "-"
            lines.append(
                f"| {probe['name']} | `{probe['status']}` | {http_status} | `{probe['url']}` | {probe['detail']} |"
            )
    else:
        lines.extend(
            [
                "",
                f"- 未执行。原因：{payload['probe_skip_reason']}",
            ]
        )

    lines.extend(
        [
            "",
            "## 业务 Smoke 前提",
            "",
            f"- 状态: `{payload['smoke_prerequisites']['status']}`",
            f"- 说明: {payload['smoke_prerequisites']['detail']}",
        ]
    )
    for missing in payload["smoke_prerequisites"]["missing_prerequisites"]:
        lines.append(f"- 缺失: {missing}")

    lines.extend(["", "## 静态资源版本映射"])
    if payload["asset_manifest"] is None:
        lines.extend(["", "- missing"])
    else:
        lines.extend(["", "```json", json.dumps(payload["asset_manifest"], ensure_ascii=False, indent=2), "```"])

    lines.extend(["", "## 阻塞原因"])
    if payload["blocking_reasons"]:
        lines.extend([""] + [f"- {reason}" for reason in payload["blocking_reasons"]])
    else:
        lines.extend(["", "- none"])
    return "\n".join(lines) + "\n"


def main() -> int:
    args = parse_args()
    workspace_root = Path(args.workspace_root).resolve()
    output_json = Path(args.output_json).resolve()
    output_md = Path(args.output_md).resolve()

    payload: dict[str, Any] = {
        "kind": "onelink_preprod_evidence_collection",
        "scope": "preprod_evidence_repair",
        "workspace_root": str(workspace_root),
        "preprod_url": pick_value(args, "preprod_url"),
        "web_url": pick_value(args, "web_url"),
        "bff_base_url": pick_value(args, "bff_base_url"),
        "api_base_url": pick_value(args, "api_base_url"),
        "candidate_version": pick_value(args, "candidate_version"),
        "image_digest": pick_value(args, "image_digest"),
        "asset_manifest": load_asset_manifest(pick_value(args, "asset_manifest")),
        "dashboard_url": pick_value(args, "dashboard_url"),
        "alert_evidence_url": pick_value(args, "alert_evidence_url"),
        "approval_url": pick_value(args, "approval_url"),
        "release_owner": pick_value(args, "release_owner"),
        "rollback_owner": pick_value(args, "rollback_owner"),
        "smoke_user": pick_value(args, "smoke_user"),
        "smoke_password_raw": pick_value(args, "smoke_password"),
        "smoke_password_present": bool(pick_value(args, "smoke_password")),
        "smoke_user_present": bool(pick_value(args, "smoke_user")),
        "require_real_preprod_validation": args.require_real_preprod,
        "written_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat(),
    }

    payload["real_preprod_validation"] = build_real_preprod_validation(payload)
    validation_map = {
        item["field"]: item for item in payload["real_preprod_validation"]
    }
    probe_skip_reason = (
        "缺少 `ONELINK_PREPROD_URL` 或对应 CLI 参数，无法对真实 pre-prod 地址抓取 `health / ready / metrics`。"
    )
    if args.require_real_preprod:
        preprod_validation = validation_map["preprod_url"]
        if preprod_validation["status"] != "pass":
            probe_input = ""
            probe_skip_reason = (
                "`ONELINK_PREPROD_URL` 未通过真实性校验，当前值不能作为真实 pre-prod probe 地址。"
            )
        else:
            probe_input = payload["preprod_url"]
    else:
        probe_input = payload["preprod_url"]
    probes = [asdict(probe) for probe in build_probe_matrix(probe_input, args.timeout_seconds)]
    payload["probes"] = probes
    payload["probe_skip_reason"] = probe_skip_reason
    payload["required_fields"] = build_required_field_status(payload)
    payload["smoke_prerequisites"] = summarize_smoke_prerequisites(payload)

    blocking_reasons = [
        field["label"] for field in payload["required_fields"] if field["status"] == "missing"
    ]
    if args.require_real_preprod:
        for field in payload["real_preprod_validation"]:
            if field["status"] != "pass":
                blocking_reasons.append(
                    f"{field['label']} 未通过真实性校验: {field['status']}"
                )
    if not probes and probe_input == "":
        blocking_reasons.append("真实 pre-prod probe 地址缺失，未能抓取 health / ready / metrics")
    for probe in probes:
        if probe["status"] != "pass":
            blocking_reasons.append(
                f"Probe {probe['name']} 未通过: {probe['status']}"
                + (
                    f" (HTTP {probe['http_status']})"
                    if probe.get("http_status") is not None
                    else ""
                )
            )
    if payload["smoke_prerequisites"]["status"] != "ready_for_execution":
        blocking_reasons.append("真实业务 smoke 前提未满足")

    payload.pop("smoke_password_raw", None)
    payload["blocking_reasons"] = blocking_reasons
    payload["collection_status"] = "blocked" if blocking_reasons else "pass"

    output_json.parent.mkdir(parents=True, exist_ok=True)
    output_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
    output_md.write_text(format_markdown(payload))

    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
