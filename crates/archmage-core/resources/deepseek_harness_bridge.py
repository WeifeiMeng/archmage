#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from importlib.metadata import version

from deepseek_harness import DeepSeekHarness, DeepSeekHarnessConfig


PROTOCOL = 1


def emit(frame: dict[str, object]) -> None:
    sys.stdout.write(json.dumps(frame, separators=(",", ":"), ensure_ascii=False) + "\n")
    sys.stdout.flush()


def require_string(frame: dict[str, object], name: str) -> str:
    value = frame.get(name)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{name} must be a nonblank string")
    return value


def read_frame() -> dict[str, object] | None:
    line = sys.stdin.readline()
    if line == "":
        return None
    frame = json.loads(line)
    if not isinstance(frame, dict):
        raise ValueError("protocol frame must be an object")
    return frame


def main() -> int:
    harness = None
    current_id = None
    try:
        start = read_frame()
        if start is None or start.get("op") != "start" or start.get("protocol") != PROTOCOL:
            raise ValueError("first frame must start protocol 1")
        sdk_version = version("deepseek-harness-sdk")
        runtime_version = version("deepseek-harness-runtime-bin")
        if sdk_version != runtime_version:
            raise RuntimeError("DeepSeek Harness SDK/runtime version mismatch")
        session_id = require_string(start, "sessionId")
        max_result_bytes = start.get("maxResultBytes")
        if not isinstance(max_result_bytes, int) or max_result_bytes <= 0:
            raise ValueError("maxResultBytes must be a positive integer")
        timeout = start.get("requestTimeoutSeconds")
        if not isinstance(timeout, (int, float)) or timeout <= 0:
            raise ValueError("requestTimeoutSeconds must be positive")
        config = DeepSeekHarnessConfig(
            provider=require_string(start, "provider"),
            model=require_string(start, "model"),
            cwd=require_string(start, "cwd"),
            runtime_cwd=require_string(start, "cwd"),
            session_root=require_string(start, "sessionRoot"),
            cordis=require_string(start, "cordis"),
            request_timeout_seconds=float(timeout),
        )
        harness = DeepSeekHarness(config)
        session = harness.start_session(session_id)
        emit({
            "op": "ready",
            "protocol": PROTOCOL,
            "sessionId": session_id,
            "sdkVersion": sdk_version,
            "runtimeVersion": runtime_version,
        })
        seen: set[str] = set()
        while True:
            frame = read_frame()
            if frame is None:
                raise ValueError("stdin closed before close frame")
            operation = frame.get("op")
            if operation == "close":
                harness.close()
                harness = None
                emit({"op": "closed", "sessionId": session_id})
                return 0
            if operation != "invoke":
                raise ValueError("unknown protocol operation")
            current_id = require_string(frame, "id")
            if current_id in seen:
                raise ValueError("duplicate invocation id")
            seen.add(current_id)
            task = require_string(frame, "task")
            schema = frame.get("schema")
            if not isinstance(schema, dict):
                raise ValueError("schema must be an object")
            prompt = (
                task
                + "\n\nOutput JSON Schema:\n"
                + json.dumps(schema, separators=(",", ":"), ensure_ascii=False)
                + "\n\nReturn exactly one JSON object matching this schema, without Markdown fences."
            )
            result = session.run(prompt)
            if result.finish_reason != "completed":
                raise RuntimeError("DeepSeek Harness turn did not complete")
            text = result.final_response
            if not isinstance(text, str) or not text:
                raise RuntimeError("DeepSeek Harness returned no final response")
            if len(text.encode("utf-8")) > max_result_bytes:
                raise RuntimeError("DeepSeek Harness final response exceeds the configured limit")
            emit({
                "op": "result",
                "id": current_id,
                "sessionId": session_id,
                "text": text,
                "finishReason": result.finish_reason,
            })
            current_id = None
    except Exception as error:
        emit({
            "op": "error",
            "id": current_id,
            "code": "bridge_error",
            "message": str(error)[:512],
        })
        return 1
    finally:
        if harness is not None:
            try:
                harness.close()
            except Exception:
                pass


if __name__ == "__main__":
    raise SystemExit(main())
