import json
import os
from dataclasses import dataclass


def _record(event, **fields):
    path = os.environ["FAKE_DSH_LOG"]
    with open(path, "a", encoding="utf-8") as stream:
        stream.write(json.dumps({"event": event, **fields}, separators=(",", ":")) + "\n")


@dataclass
class DeepSeekHarnessConfig:
    provider: str
    model: str
    cwd: str
    runtime_cwd: str
    profile: str
    patches: tuple[str, ...]
    dsh_home: str
    request_timeout_seconds: float


@dataclass
class RunResult:
    session_id: str
    final_response: str
    finish_reason: str | None


class Session:
    def __init__(self, session_id):
        self.id = session_id

    def run(self, prompt):
        _record("run", sessionId=self.id, prompt=prompt)
        if os.environ.get("FAKE_DSH_MODE") == "error":
            raise RuntimeError("fake sdk failure")
        text = "x" * 8192 if os.environ.get("FAKE_DSH_MODE") == "oversize" else '{"ok":true}'
        reason = "error" if os.environ.get("FAKE_DSH_MODE") == "finish-error" else "completed"
        return RunResult(self.id, text, reason)


class DeepSeekHarness:
    def __init__(self, config):
        self.config = config
        _record(
            "harness",
            provider=config.provider,
            model=config.model,
            cwd=config.cwd,
            runtimeCwd=config.runtime_cwd,
            profile=config.profile,
            patches=config.patches,
            dshHome=config.dsh_home,
        )

    def start_session(self, session_id):
        _record("session", sessionId=session_id)
        return Session(session_id)

    def close(self):
        _record("close")
