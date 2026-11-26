#!/usr/bin/env python3
"""Minimal Ferrite Cloud MCP client (Python standard library only).

Use it to talk to Ferrite with a token you hold yourself (for example an
ephemeral trial account), outside a connector that already sends its own
OAuth identity.

    ferrite_mcp.py provision OUT_FILE              provision_ephemeral; credentials go to OUT_FILE (mode 0600)
    ferrite_mcp.py call TOOL [JSON_ARGS|@FILE]     call any tool and print its JSON result
    ferrite_mcp.py files APP_DIR                   print the files map (Cargo.toml, ferrite.json, src/, tests/)
    ferrite_mcp.py commit APP_DIR --app NAME       rcs_commit the files map to the app's hosted repository
    ferrite_mcp.py deploy APP_DIR                  deploy_source with APP_DIR/ferrite.json and the sources
    ferrite_mcp.py wait BUILD_ID                   poll build_status until the build settles

The token comes from --token-file (a JSON file with a "token" field, as
written by `provision`, or a file holding just the token) or from
$FERRITE_TOKEN. It is sent as a Bearer header and never printed. The server
URL defaults to https://mcp.ferrite.cloud/mcp ($FERRITE_MCP_URL).
"""

import argparse
import json
import os
import pathlib
import sys
import time
import urllib.error
import urllib.request

DEFAULT_URL = "https://mcp.ferrite.cloud/mcp"
SETTLED = {"deployed", "rejected", "failed", "stuck", "tested", "test_failed"}


class McpError(RuntimeError):
    pass


class Client:
    """An MCP client over streamable HTTP (JSON or SSE responses)."""

    def __init__(self, url, token=None):
        self.url = url
        self.token = token
        self.session = None
        self.next_id = 1

    def _post(self, payload):
        headers = {
            "content-type": "application/json",
            "accept": "application/json, text/event-stream",
        }
        if self.token:
            headers["authorization"] = f"Bearer {self.token}"
        if self.session:
            headers["mcp-session-id"] = self.session
        request = urllib.request.Request(
            self.url, data=json.dumps(payload).encode(), headers=headers, method="POST"
        )
        try:
            with urllib.request.urlopen(request, timeout=600) as response:
                session = response.headers.get("mcp-session-id")
                if session:
                    self.session = session
                return response.headers.get("content-type", ""), response.read().decode()
        except urllib.error.HTTPError as e:
            raise McpError(f"HTTP {e.code}: {e.read().decode(errors='replace')[:2000]}") from None

    def _request(self, method, params):
        payload = {"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}
        self.next_id += 1
        content_type, body = self._post(payload)
        if "text/event-stream" in content_type:
            messages = [
                json.loads(line[5:].strip())
                for line in body.splitlines()
                if line.startswith("data:") and line[5:].strip()
            ]
            reply = next((m for m in messages if m.get("id") == payload["id"]), None)
        else:
            reply = json.loads(body) if body.strip() else None
        if reply is None:
            raise McpError(f"no reply to {method}: {body[:2000]}")
        if "error" in reply:
            raise McpError(f"{method} failed: {json.dumps(reply['error'])}")
        return reply["result"]

    def initialize(self):
        self._request(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "swatchbook-ferrite-cli", "version": "0.1"},
            },
        )
        self._post({"jsonrpc": "2.0", "method": "notifications/initialized"})
        return self

    def call(self, tool, arguments):
        """Call a tool. Returns (is_error, data) with data parsed as JSON if possible."""
        result = self._request("tools/call", {"name": tool, "arguments": arguments})
        if result.get("structuredContent") is not None:
            data = result["structuredContent"]
        else:
            texts = [c.get("text", "") for c in result.get("content", []) if c.get("type") == "text"]
            joined = "\n".join(texts)
            try:
                data = json.loads(joined)
            except ValueError:
                data = joined
        return bool(result.get("isError")), data


def read_token(path):
    if path:
        text = pathlib.Path(path).read_text().strip()
        try:
            return json.loads(text)["token"]
        except (ValueError, KeyError, TypeError):
            return text
    return os.environ.get("FERRITE_TOKEN")


def app_files(app_dir, include_spec=True):
    """The files a Ferrite build needs: Cargo.toml, src/**, tests/** and the spec."""
    root = pathlib.Path(app_dir)
    files = {"Cargo.toml": (root / "Cargo.toml").read_text()}
    if include_spec and (root / "ferrite.json").exists():
        files["ferrite.json"] = (root / "ferrite.json").read_text()
    for sub in ("src", "tests"):
        for path in sorted((root / sub).rglob("*.rs")):
            files[path.relative_to(root).as_posix()] = path.read_text()
    return files


def print_json(data):
    print(json.dumps(data, indent=2, sort_keys=True))


def parse_arguments(text):
    """Tool arguments as inline JSON, or `@path` to read them from a file."""
    if text.startswith("@"):
        text = pathlib.Path(text[1:]).read_text()
    return json.loads(text)


def build_state(data):
    """The state field of a build_status answer, whatever envelope it uses."""
    if isinstance(data, dict):
        for value in data.values():
            if isinstance(value, dict) and "state" in value:
                return value["state"]
        if "PreviewDeployed" in data:
            return "deployed"
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--token-file", help="JSON credentials file (from `provision`) or a bare token file")
    parser.add_argument("--url", default=os.environ.get("FERRITE_MCP_URL", DEFAULT_URL))
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("provision")
    p.add_argument("out_file")
    p = sub.add_parser("call")
    p.add_argument("tool")
    p.add_argument("arguments", nargs="?", default="{}")
    p = sub.add_parser("files")
    p.add_argument("app_dir")
    p = sub.add_parser("commit")
    p.add_argument("app_dir")
    p.add_argument("--app", required=True)
    p.add_argument("--branch")
    p.add_argument("--message", default="Update from ferrite_mcp.py")
    p = sub.add_parser("deploy")
    p.add_argument("app_dir")
    p = sub.add_parser("wait")
    p.add_argument("build_id")
    p.add_argument("--interval", type=float, default=10.0)
    p.add_argument("--timeout", type=float, default=1800.0)
    args = parser.parse_args()

    if args.command == "files":
        print_json(app_files(args.app_dir))
        return 0

    if args.command == "provision":
        client = Client(args.url).initialize()
        is_error, data = client.call("provision_ephemeral", {})
        if is_error or not isinstance(data, dict) or "EphemeralProvisioned" not in data:
            print_json(data)
            return 1
        creds = data["EphemeralProvisioned"]
        out = pathlib.Path(args.out_file)
        # Create the file owner-only before any secret is written; refuse to
        # overwrite an existing file.
        fd = os.open(out, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "w") as handle:
            handle.write(json.dumps(creds))
        print_json({"account": creds.get("account"), "expires_at_ms": creds.get("expires_at_ms"), "saved_to": str(out)})
        return 0

    token = read_token(args.token_file)
    if not token:
        print("no token: pass --token-file or set FERRITE_TOKEN", file=sys.stderr)
        return 2
    client = Client(args.url, token).initialize()

    if args.command == "call":
        is_error, data = client.call(args.tool, parse_arguments(args.arguments))
        print_json(data)
        return 1 if is_error else 0

    if args.command == "commit":
        arguments = {"app": args.app, "files": app_files(args.app_dir), "message": args.message}
        if args.branch:
            arguments["branch"] = args.branch
        is_error, data = client.call("rcs_commit", arguments)
        print_json(data)
        return 1 if is_error else 0

    if args.command == "deploy":
        spec = (pathlib.Path(args.app_dir) / "ferrite.json").read_text()
        files = app_files(args.app_dir, include_spec=False)
        is_error, data = client.call("deploy_source", {"spec": spec, "files": files})
        print_json(data)
        return 1 if is_error else 0

    if args.command == "wait":
        deadline = time.time() + args.timeout
        last = None
        while True:
            is_error, data = client.call("build_status", {"build_id": args.build_id})
            state = build_state(data)
            if state != last:
                print(f"[{time.strftime('%H:%M:%S')}] state: {state}", file=sys.stderr, flush=True)
                last = state
            if is_error or state in SETTLED or state is None or time.time() > deadline:
                print_json(data)
                return 0 if state in {"deployed", "tested"} else 1
            time.sleep(args.interval)

    return 2


if __name__ == "__main__":
    try:
        sys.exit(main())
    except McpError as e:
        print(f"error: {e}", file=sys.stderr)
        sys.exit(1)
