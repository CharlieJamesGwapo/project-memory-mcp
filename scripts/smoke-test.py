#!/usr/bin/env python3
"""Exercise a built binary over real stdio, outside the repository, with temporary data."""
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import threading


class Client:
    def __init__(self, binary, database, cwd, policy=None):
        self.process = subprocess.Popen(
            [str(binary), "--db", str(database), *(["--policy", str(policy)] if policy else [])], cwd=cwd,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8", bufsize=1,
        )
        self.lines = queue.Queue()
        self.errors = []
        self.sequence = 0
        def read_stdout():
            for line in self.process.stdout:
                self.lines.put(line)
            self.lines.put(None)
        def read_stderr():
            for line in self.process.stderr:
                if len(self.errors) < 20:
                    self.errors.append(line.rstrip())
        threading.Thread(target=read_stdout, daemon=True).start()
        threading.Thread(target=read_stderr, daemon=True).start()

    def send(self, value):
        self.process.stdin.write(json.dumps(value) + "\n")
        self.process.stdin.flush()

    def request(self, method, params):
        self.sequence += 1
        request_id = self.sequence
        self.send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        while True:
            try:
                line = self.lines.get(timeout=10)
            except queue.Empty as exc:
                raise AssertionError(f"Timeout for {method}; stderr={self.errors}") from exc
            if line is None:
                raise AssertionError(f"Server closed stdout; stderr={self.errors}")
            response = json.loads(line)  # Any non-protocol stdout makes the check fail.
            if response.get("id") == request_id:
                if "error" in response:
                    raise AssertionError(response["error"])
                return response["result"]

    def initialize(self):
        self.request("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "project-memory-blackbox", "version": "0.1.0"}})
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def tool(self, name, args, error=None):
        result = self.request("tools/call", {"name": name, "arguments": args})
        value = result["structuredContent"]
        if error:
            assert result.get("isError") is True, result
            assert value["error"]["code"] == error, value
        else:
            assert result.get("isError") is not True, result
        return value

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
            raise AssertionError("Server did not stop after stdin EOF")
        assert self.process.returncode == 0, self.errors


def smoke(binary):
    with tempfile.TemporaryDirectory(prefix="project memory smoke ") as directory:
        cwd = Path(directory)
        database = cwd / "private memory.sqlite"
        client = Client(binary, database, cwd)
        try:
            client.initialize()
            default_tools=client.request("tools/list", {})["tools"]
            assert len(default_tools)==15 and not any(t['name']=='delete_project' for t in default_tools)
            assert client.tool("get_access_policy", {})["allow_delete"] is False
        finally:
            client.close()
        policy=cwd / "explicit-deletion-policy.json"
        policy.write_text(json.dumps({"allow_delete":True}))
        client = Client(binary, database, cwd, policy)
        try:
            client.initialize()
            tools = client.request("tools/list", {})["tools"]
            assert len(tools) == 16
            a = client.tool("create_project", {"name": "Atlas", "purpose": "Synthetic MCP test"})["id"]
            b = client.tool("create_project", {"name": "Atlas", "purpose": "Separate synthetic project"})["id"]
            assert len(client.tool("list_projects", {})["items"]) == 2
            note = client.tool("save_note", {"project_id": a, "title": "Literal", "content": "100%_complete 日本語", "author_type": "assistant", "sources": []})
            found = client.tool("search_notes", {"project_id": a, "query": "%_"})
            assert found["items"][0]["id"] == note["id"]
            client.tool("get_record", {"project_id": b, "record_id": note["id"]}, error="NOT_FOUND")
            old = client.tool("record_decision", {"project_id": a, "decision": "Use local storage", "rationale": "Local use", "author_type": "user", "sources": []})
            current = client.tool("record_decision", {"project_id": a, "decision": "Use SQLite", "rationale": "Structured local data", "author_type": "user", "sources": [], "supersedes": old["id"]})
            client.tool("save_handoff", {"project_id": a, "summary": "Smoke test", "completed": ["Saved synthetic records"], "unfinished": [], "next_actions": ["Review output"], "blockers": [], "verification": ["This temporary MCP call succeeded"], "author_type": "assistant", "sources": []})
            context = client.tool("get_project_context", {"project_id": a})
            assert context["active_decisions"]["items"][0]["id"] == current["id"]
            unified = client.tool("search_memory", {"project_id": a, "query": "Structured local data"})
            assert unified["items"][0]["id"] == current["id"]
            bundle = client.tool("get_context_bundle", {"project_id": a, "record_ids": [note["id"], current["id"]], "max_bytes": 16000})
            assert [record["id"] for record in bundle["records"]] == [note["id"], current["id"]]
            assert bundle["records"][0]["content"] == "100%_complete 日本語"
            assert bundle["budget"]["used_bytes"] <= 16000
            assert bundle["selection"]["selection_complete"] is True
            secret="ghp_"+"A"*36
            client.tool("save_note", {"project_id":a,"title":"Reject synthetic credential","content":secret,"author_type":"user"},error="SECRET_DETECTED")
            review=client.tool("save_review", {"project_id":a,"repository":"synthetic-smoke","base_commit":"1"*40,"head_commit":"2"*40,"summary":"Release protocol review","checks":[{"name":"actual smoke","outcome":"passed","result":"This temporary protocol operation completed"}],"findings":[],"author_type":"assistant","sources":[]})
            history=client.tool("get_review_history", {"project_id":a,"current_head":"3"*40})
            assert history["items"][0]["id"]==review["id"] and history["items"][0]["freshness"]=="stale"
            assert history["budget"]["used_bytes"]<=16000
            health=client.tool("get_memory_health", {"project_id":a,"current_head":"3"*40,"max_bytes":4096})
            assert health["summary"]["review_count"]==1 and health["budget"]["used_bytes"]<=4096
            assert any(any(signal["code"]=="review_head_changed" for signal in item["signals"]) for item in health["items"])
            assert len(client.request("resources/templates/list", {})["resourceTemplates"])==2
            resource=client.request("resources/read", {"uri":f"project-memory://projects/{a}/brief"})
            assert json.loads(resource["contents"][0]["text"])["project"]["id"]==a
            assert len(client.request("prompts/list", {})["prompts"])==3
            prompt=client.request("prompts/get", {"name":"resume_project","arguments":{"project_id":a}})
            assert "untrusted" in prompt["messages"][0]["content"]["text"]
            export = client.tool("export_project", {"project_id": a})
            assert "superseded_by" in export["markdown"]
            assert len(export["record_ids"]) == 5
            client.tool("delete_project", {"project_id": a, "confirm": False}, error="VALIDATION_ERROR")
        finally:
            client.close()
        client = Client(binary, database, cwd, policy)
        try:
            client.initialize()
            assert client.tool("get_record", {"project_id": a, "record_id": note["id"]})["content"] == "100%_complete 日本語"
            assert client.tool("delete_project", {"project_id": a, "confirm": True})["deleted"] is True
            projects = client.tool("list_projects", {})["items"]
            assert len(projects) == 1 and projects[0]["id"] == b
        finally:
            client.close()
    print("Black-box MCP smoke passed: 15 default/16 deletion-enabled tools, local policy, credential rejection, commit-linked review history, memory health, resources/prompts, unified literal search, budgeted full records, history, export, isolation, rejection, restart persistence, deletion and protocol-only stdout")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python3 scripts/smoke-test.py /absolute/path/to/project-memory-mcp")
    binary = Path(sys.argv[1]).resolve(strict=True)
    smoke(binary)
