#!/usr/bin/env python3
import json
import os
import sys

scenario = os.environ.get("FAKE_QODER_SCENARIO", "success")

if "--version" in sys.argv:
    print("1.2.3-test")
    raise SystemExit(0)

def send(value):
    sys.stdout.write(json.dumps(value, separators=(",", ":")) + "\n")
    sys.stdout.flush()

def call(request_id, method, params):
    send({"jsonrpc":"2.0","id":request_id,"method":method,"params":params})
    response = json.loads(sys.stdin.readline())
    if response.get("id") != request_id:
        raise RuntimeError("client response id mismatch")
    return response

cwd = None

for line in sys.stdin:
    request = json.loads(line)
    method = request.get("method")
    request_id = request.get("id")
    if method == "initialize":
        version = 99 if scenario == "bad-version" else 1
        send({"jsonrpc":"2.0","id":request_id,"result":{
            "protocolVersion":version,
            "agentInfo":{"name":"fake-qoder","version":"1.2.3-test"},
            "agentCapabilities":{"loadSession":True,"promptCapabilities":{}},
            "authMethods":[]
        }})
    elif method == "session/new":
        cwd = request["params"]["cwd"]
        send({"jsonrpc":"2.0","id":request_id,"result":{"sessionId":"fake-session"}})
    elif method == "session/prompt":
        if scenario == "client-tools":
            path = os.path.join(cwd, "acp-written.txt")
            assert "result" in call(100, "fs/write_text_file", {
                "sessionId":"fake-session", "path":path, "content":"written through ACP\n"
            })
            read = call(101, "fs/read_text_file", {
                "sessionId":"fake-session", "path":path, "line":1, "limit":1
            })
            assert read["result"]["content"] == "written through ACP\n"
            terminal = call(102, "terminal/create", {
                "sessionId":"fake-session", "command":"/bin/pwd", "cwd":cwd,
                "outputByteLimit":4096
            })
            terminal_id = terminal["result"]["terminalId"]
            output = call(103, "terminal/output", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })
            assert cwd in output["result"]["output"]
            permission = call(104, "session/request_permission", {
                "sessionId":"fake-session", "toolCall":{"toolCallId":"tool-1"},
                "options":[
                    {"optionId":"allow", "name":"Allow once", "kind":"allow_once"},
                    {"optionId":"reject", "name":"Reject once", "kind":"reject_once"}
                ]
            })
            assert permission["result"]["outcome"] == {"outcome":"selected", "optionId":"allow"}
        text = '{"status":"implemented"}'
        if scenario == "invalid-result":
            text = '```json\n{"status":"implemented"}\n```'
        if scenario == "oversized":
            text = '{"value":"' + ('x' * 4096) + '"}'
        send({"jsonrpc":"2.0","method":"session/update","params":{
            "sessionId":"fake-session",
            "update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":text}}
        }})
        send({"jsonrpc":"2.0","id":request_id,"result":{"stopReason":"end_turn"}})
    elif method == "session/cancel":
        continue

