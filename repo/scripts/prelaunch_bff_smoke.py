#!/usr/bin/env python3
import argparse
import json
import os
import signal
import socket
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


INTERNAL_TOKEN = "onelink-dev-internal-token"
AUTH_TOKEN = "Bearer smoke-token"
USER_ID = "11111111-1111-1111-1111-111111111111"
RECIPIENT_USER_ID = "22222222-2222-2222-2222-222222222222"
THREAD_ID = "33333333-3333-3333-3333-333333333333"
MESSAGE_ID = "44444444-4444-4444-4444-444444444444"
REPORT_ID = "55555555-5555-5555-5555-555555555555"
APPEAL_ID = "66666666-6666-6666-6666-666666666666"


def json_request(method, url, payload=None, headers=None, timeout=10):
    data = None
    request_headers = dict(headers or {})
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        request_headers.setdefault("Content-Type", "application/json")
    req = urllib.request.Request(url, data=data, headers=request_headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8")
            return resp.status, json.loads(body) if body else {}
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8")
        parsed = body
        try:
            parsed = json.loads(body) if body else {}
        except json.JSONDecodeError:
            pass
        return exc.code, parsed


class MockHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        self._dispatch("GET")

    def do_POST(self):
        self._dispatch("POST")

    def do_PATCH(self):
        self._dispatch("PATCH")

    def log_message(self, fmt, *args):
        sys.stdout.write("[mock:%s] " % self.server.server_port + (fmt % args) + "\n")

    def _read_json(self):
        length = int(self.headers.get("Content-Length", "0"))
        if length <= 0:
            return None
        raw = self.rfile.read(length).decode("utf-8")
        return json.loads(raw)

    def _json(self, status, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _require_internal(self):
        got = self.headers.get("x-internal-token")
        if got != INTERNAL_TOKEN:
            self._json(401, {"error": "missing or invalid internal token"})
            return False
        return True

    def _dispatch(self, method):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        query = urllib.parse.parse_qs(parsed.query)
        body = self._read_json()
        self.server.requests.append(
            {
                "method": method,
                "path": path,
                "query": query,
                "body": body,
                "authorization": self.headers.get("Authorization"),
                "internal_token": self.headers.get("x-internal-token"),
            }
        )

        if self.server.service_name == "identity":
            self._handle_identity(method, path, body)
            return
        if not self._require_internal():
            return
        if self.server.service_name == "profile":
            self._handle_profile(method, path, body)
        elif self.server.service_name == "match":
            self._handle_match(method, path, query, body)
        elif self.server.service_name == "dm":
            self._handle_dm(method, path, query, body)
        elif self.server.service_name == "safety":
            self._handle_safety(method, path, body)
        elif self.server.service_name == "admin":
            self._handle_admin(method, path, body)
        else:
            self._json(404, {"error": "unknown service"})

    def _handle_identity(self, method, path, body):
        if method == "POST" and path == "/api/v1/identity/login":
            self._json(
                200,
                {
                    "user_id": USER_ID,
                    "session": {
                        "token": AUTH_TOKEN.split(" ", 1)[1],
                        "expires_at": "2026-06-28T00:00:00Z",
                    },
                },
            )
            return
        if method == "POST" and path == "/api/v1/identity/register":
            self._json(
                200,
                {
                    "user_id": USER_ID,
                    "session": {
                        "token": AUTH_TOKEN.split(" ", 1)[1],
                        "expires_at": "2026-06-28T00:00:00Z",
                    },
                },
            )
            return
        if method == "GET" and path == "/api/v1/identity/me":
            if self.headers.get("Authorization") != AUTH_TOKEN:
                self._json(401, {"error": "unauthorized"})
                return
            self._json(
                200,
                {
                    "user_id": USER_ID,
                    "status": "active",
                    "primary_region": "CN",
                    "primary_language": "zh-CN",
                    "content_language": "zh-CN",
                    "notification_language": "zh-CN",
                    "timezone": "Asia/Shanghai",
                },
            )
            return
        self._json(404, {"error": "identity route not mocked", "path": path})

    def _handle_profile(self, method, path, body):
        if method == "GET" and path == "/api/v1/profile/me/compliance":
            self._json(
                200,
                {
                    "data_export_available": True,
                    "data_delete_available": True,
                    "data_correction_available": True,
                    "pending_requests": [],
                },
            )
            return
        if method == "POST" and path == "/api/v1/profile/me/compliance/export":
            self._json(200, {"request_id": "export-1", "status": "submitted"})
            return
        if method == "POST" and path == "/api/v1/profile/me/compliance/delete":
            self._json(200, {"request_id": "delete-1", "status": "submitted"})
            return
        if method == "POST" and path == "/api/v1/profile/me/compliance/correction":
            self._json(200, {"request_id": "correction-1", "status": "submitted"})
            return
        if method == "PATCH" and path == "/api/v1/profile/me":
            self._json(200, {"user_id": USER_ID, "display_name": body.get("display_name", "Smoke User")})
            return
        self._json(404, {"error": "profile route not mocked", "path": path})

    def _handle_match(self, method, path, query, body):
        if method == "GET" and path == "/api/v1/match/recommendations":
            self._json(
                200,
                {
                    "request_id": "rec-req-1",
                    "state": "ready",
                    "recommendations": [
                        {
                            "recommendation_id": "rec-1",
                            "user_id": RECIPIENT_USER_ID,
                            "headline": "AI infra founder",
                        }
                    ],
                },
            )
            return
        if method == "GET" and path == "/api/v1/match/find-requests":
            self._json(
                200,
                {
                    "find_requests": [{"find_request_id": "find-1", "status": "pending"}],
                    "candidates": [{"user_id": RECIPIENT_USER_ID, "score": 0.91}],
                },
            )
            return
        if method == "POST" and path == "/api/v1/match/find-requests":
            self._json(202, {"find_request_id": "find-1", "status": "pending"})
            return
        self._json(404, {"error": "match route not mocked", "path": path, "query": query, "body": body})

    def _handle_dm(self, method, path, query, body):
        if method == "GET" and path == "/api/v1/dm/threads":
            self._json(
                200,
                {
                    "threads": [
                        {
                            "thread_id": THREAD_ID,
                            "participant_user_id": RECIPIENT_USER_ID,
                            "latest_message_preview": "hello from smoke",
                        }
                    ]
                },
            )
            return
        if method == "POST" and path == "/api/v1/dm/threads":
            self._json(200, {"thread_id": THREAD_ID, "created": True})
            return
        if method == "POST" and path == f"/api/v1/dm/threads/{THREAD_ID}/messages":
            self._json(200, {"message_id": MESSAGE_ID, "status": "sent"})
            return
        self._json(404, {"error": "dm route not mocked", "path": path, "query": query, "body": body})

    def _handle_safety(self, method, path, body):
        if method == "POST" and path == "/api/v1/safety/dm-first-message-review":
            self._json(200, {"allowed": True, "reason": "clean"})
            return
        if method == "POST" and path == "/api/v1/safety/reports":
            self._json(200, {"report_ticket_id": REPORT_ID, "status": "submitted"})
            return
        if method == "POST" and path == "/api/v1/safety/blocks":
            self._json(200, {"blocked_user_id": RECIPIENT_USER_ID, "created_at": "2026-05-28T00:00:00Z"})
            return
        if method == "GET" and path == f"/api/v1/safety/appeals/{APPEAL_ID}":
            self._json(200, {"appeal_id": APPEAL_ID, "status": "under_review"})
            return
        self._json(404, {"error": "safety route not mocked", "path": path, "body": body})

    def _handle_admin(self, method, path, body):
        if method == "GET" and path == "/api/v1/admin/reports":
            self._json(200, {"reports": [{"report_ticket_id": REPORT_ID, "status": "under_review"}]})
            return
        if method == "GET" and path == f"/api/v1/admin/reports/{REPORT_ID}":
            self._json(200, {"report_ticket_id": REPORT_ID, "status": "under_review", "reason": "harassment"})
            return
        if method == "POST" and path == f"/api/v1/admin/reports/{REPORT_ID}/action":
            self._json(200, {"report_ticket_id": REPORT_ID, "action_status": "queued", "action": body.get("action")})
            return
        if method == "GET" and path == "/api/v1/admin/appeals":
            self._json(200, {"appeals": [{"appeal_id": APPEAL_ID, "status": "under_review"}]})
            return
        if method == "GET" and path == "/api/v1/admin/metrics":
            self._json(200, {"queue_depth": 1, "open_reports": 1, "open_appeals": 1})
            return
        self._json(404, {"error": "admin route not mocked", "path": path, "body": body})


class NamedServer(ThreadingHTTPServer):
    def __init__(self, address, service_name):
        super().__init__(address, MockHandler)
        self.service_name = service_name
        self.requests = []


def wait_for_tcp(port, timeout=15):
    start = time.time()
    while time.time() - start < timeout:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                return True
        except OSError:
            time.sleep(0.1)
    return False


def wait_for_json(url, expected_key, timeout=15):
    start = time.time()
    while time.time() - start < timeout:
        try:
            status, body = json_request("GET", url)
            if status == 200 and expected_key in body:
                return body
        except Exception:
            pass
        time.sleep(0.2)
    raise RuntimeError("timeout waiting for %s" % url)


def start_mock(port, name):
    server = NamedServer(("127.0.0.1", port), name)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    print("[mock-start] %s on %s" % (name, port))
    return server


def write_report(report_path, health_snapshot, smoke_results, mock_servers):
    lines = [
        "# API Smoke Report",
        "",
        "## BFF Health Snapshot",
        "",
        "- `/health`: `%s`" % json.dumps(health_snapshot["health"], ensure_ascii=False, sort_keys=True),
        "- `/ready`: `%s`" % json.dumps(health_snapshot["ready"], ensure_ascii=False, sort_keys=True),
        "- `/metrics`: `%s`" % json.dumps(health_snapshot["metrics"], ensure_ascii=False, sort_keys=True),
        "",
        "## Critical API Smoke",
        "",
        "| Case | Status | Notes |",
        "| --- | --- | --- |",
    ]
    for item in smoke_results:
        lines.append("| %s | %s | %s |" % (item["name"], item["status"], item["notes"]))

    lines.extend(
        [
            "",
            "## Mock Downstream Coverage",
            "",
            "| Service | Requests |",
            "| --- | --- |",
        ]
    )
    for name, server in mock_servers.items():
        lines.append("| %s | %s |" % (name, len(server.requests)))

    with open(report_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace-root", required=True)
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--report-md", required=True)
    parser.add_argument("--bff-port", type=int, default=28083)
    args = parser.parse_args()

    mock_ports = {
        "identity": 28081,
        "profile": 28082,
        "match": 28087,
        "safety": 28088,
        "dm": 28089,
        "admin": 28090,
    }

    os.makedirs(os.path.dirname(args.report_md), exist_ok=True)

    mock_servers = {}
    bff_proc = None
    try:
        for name, port in mock_ports.items():
            mock_servers[name] = start_mock(port, name)

        bff_bin = os.path.join(args.repo_root, "target", "debug", "bff")
        env = os.environ.copy()
        env.update(
            {
                "PORT": str(args.bff_port),
                "ONELINK_ENV": "dev",
                "INTERNAL_SHARED_SECRET": INTERNAL_TOKEN,
                "IDENTITY_SERVICE_BASE_URL": "http://127.0.0.1:%d" % mock_ports["identity"],
                "PROFILE_SERVICE_BASE_URL": "http://127.0.0.1:%d" % mock_ports["profile"],
                "MATCH_SERVICE_BASE_URL": "http://127.0.0.1:%d" % mock_ports["match"],
                "SAFETY_SERVICE_BASE_URL": "http://127.0.0.1:%d" % mock_ports["safety"],
                "DM_SERVICE_BASE_URL": "http://127.0.0.1:%d" % mock_ports["dm"],
                "ADMIN_SERVICE_BASE_URL": "http://127.0.0.1:%d" % mock_ports["admin"],
                "CORS_ALLOWED_ORIGINS": "http://localhost:3000",
                "RUST_LOG": "info",
            }
        )
        bff_proc = subprocess.Popen(
            [bff_bin],
            cwd=args.repo_root,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        print("[bff-start] pid=%s port=%s" % (bff_proc.pid, args.bff_port))
        if not wait_for_tcp(args.bff_port):
            raise RuntimeError("bff did not start on port %s" % args.bff_port)

        base = "http://127.0.0.1:%d" % args.bff_port
        health = wait_for_json(base + "/health", "service")
        ready = wait_for_json(base + "/ready", "service")
        metrics = wait_for_json(base + "/metrics", "service")

        smoke_results = []

        def record(name, method, path, expected_status, payload=None, expected_key=None):
            headers = {}
            if path not in ("/api/v1/bff/auth/login", "/api/v1/bff/auth/register"):
                headers["Authorization"] = AUTH_TOKEN
            status, body = json_request(method, base + path, payload=payload, headers=headers)
            ok = status == expected_status and (expected_key is None or expected_key in body)
            notes = "http=%s body=%s" % (status, json.dumps(body, ensure_ascii=False, sort_keys=True))
            print("[smoke] %s => %s" % (name, notes))
            smoke_results.append(
                {"name": name, "status": "pass" if ok else "fail", "notes": notes}
            )
            if not ok:
                raise RuntimeError("smoke failed: %s" % name)

        record(
            "login",
            "POST",
            "/api/v1/bff/auth/login",
            200,
            payload={"provider": "email", "email": "smoke@example.com", "password": "smoke-pw"},
            expected_key="session",
        )
        record("recommendation", "GET", "/api/v1/bff/recommendations", 200, expected_key="recommendations")
        record("match_results", "GET", "/api/v1/bff/find/results", 200, expected_key="candidates")
        record(
            "dm_send",
            "POST",
            "/api/v1/bff/dm/send",
            200,
            payload={"recipient_user_id": RECIPIENT_USER_ID, "content": "hello from prelaunch smoke"},
            expected_key="message_id",
        )
        record(
            "safety_report",
            "POST",
            "/api/v1/bff/safety/report",
            200,
            payload={"reported_user_id": RECIPIENT_USER_ID, "reason": "harassment", "description": "smoke"},
            expected_key="report_ticket_id",
        )
        record("admin_reports", "GET", "/api/v1/bff/admin/reports", 200, expected_key="reports")
        record("admin_appeals", "GET", "/api/v1/bff/admin/appeals", 200, expected_key="appeals")
        record("locale_registry", "GET", "/api/v1/bff/settings/locale", 200, expected_key="default_locale")
        record("compliance_summary", "GET", "/api/v1/bff/compliance/summary", 200, expected_key="user_id")

        write_report(
            args.report_md,
            {"health": health, "ready": ready, "metrics": metrics},
            smoke_results,
            mock_servers,
        )
        print("[report] wrote %s" % args.report_md)
        return 0
    finally:
        if bff_proc is not None and bff_proc.poll() is None:
            bff_proc.send_signal(signal.SIGTERM)
            try:
                bff_proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                bff_proc.kill()
        if bff_proc is not None and bff_proc.stdout is not None:
            leftover = bff_proc.stdout.read()
            if leftover:
                print("[bff-log]")
                print(leftover.rstrip())
        for server in mock_servers.values():
            server.shutdown()
            server.server_close()


if __name__ == "__main__":
    raise SystemExit(main())
