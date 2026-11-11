#!/usr/bin/python3
import json
import os
import subprocess
import sys
import time

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
        if scenario == "initialize-hang":
            child = subprocess.Popen(["/bin/sleep", "60"])
            with open(os.environ["FAKE_CHILD_PID_PATH"], "w") as pid_file:
                pid_file.write(str(child.pid))
            time.sleep(60)
        version = 99 if scenario == "bad-version" else 1
        response_id = request_id + 1 if scenario == "bad-id" else request_id
        send({"jsonrpc":"2.0","id":response_id,"result":{
            "protocolVersion":version,
            "agentInfo":{"name":"fake-qoder","version":"1.2.3-test"},
            "agentCapabilities":{"loadSession":scenario != "no-load","promptCapabilities":{},"sessionCapabilities":{"close":{}}},
            "authMethods":[]
        }})
    elif method == "session/new":
        cwd = request["params"]["cwd"]
        send({"jsonrpc":"2.0","id":request_id,"result":{"sessionId":"fake-session"}})
    elif method == "session/load":
        cwd = request["params"]["cwd"]
        if scenario == "load-expired":
            send({"jsonrpc":"2.0","id":request_id,"error":{"code":-32001,"message":"session expired"}})
        else:
            send({"jsonrpc":"2.0","id":request_id,"result":{}})
    elif method == "session/prompt":
        if scenario == "prompt-hang":
            time.sleep(60)
        if scenario == "terminal-wait-hang":
            terminal_id = call(140, "terminal/create", {
                "sessionId":"fake-session", "command":"/bin/sleep", "args":["60"], "cwd":cwd
            })["result"]["terminalId"]
            call(141, "terminal/wait_for_exit", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })
        if scenario == "terminal-leak":
            call(130, "terminal/create", {
                "sessionId":"fake-session", "command":"/bin/sh",
                "args":["-c", "echo $$ > terminal.pid; sleep 60"], "cwd":cwd
            })
            time.sleep(0.1)
        if scenario == "readonly-direct-write":
            try:
                with open(os.path.join(cwd, "forbidden.txt"), "w") as target:
                    target.write("escaped")
            except OSError:
                pass
            os.system("/bin/sh -c 'printf escaped > forbidden-child.txt'")
        if scenario == "sandbox-escape":
            escape_path = os.environ["FAKE_ESCAPE_PATH"]
            terminal_id = call(120, "terminal/create", {
                "sessionId":"fake-session", "command":"/bin/sh",
                "args":["-c", "printf escaped > " + escape_path], "cwd":cwd
            })["result"]["terminalId"]
            call(121, "terminal/wait_for_exit", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })
            call(122, "terminal/release", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })
        read_escape = None
        if scenario == "sandbox-read-escape":
            escape_path = os.environ["FAKE_ESCAPE_PATH"]
            try:
                with open(escape_path, "r") as source:
                    direct_read = source.read()
            except OSError:
                direct_read = ""
            terminal_id = call(123, "terminal/create", {
                "sessionId":"fake-session", "command":"/bin/cat",
                "args":[escape_path], "cwd":cwd
            })["result"]["terminalId"]
            call(124, "terminal/wait_for_exit", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })
            terminal_read = call(125, "terminal/output", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })["result"]["output"]
            call(126, "terminal/release", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })
            marker = "TOP_SECRET_CONTENT"
            read_escape = {"direct": marker in direct_read, "terminal": marker in terminal_read}
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
            call(112, "terminal/wait_for_exit", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })
            output = call(103, "terminal/output", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })
            assert os.path.realpath(cwd) in output["result"]["output"], (cwd, output)
            call(106, "terminal/release", {
                "sessionId":"fake-session", "terminalId":terminal_id
            })
            permission = call(104, "session/request_permission", {
                "sessionId":"fake-session", "toolCall":{"toolCallId":"tool-1","kind":"edit","locations":[{"path":path}]},
                "options":[
                    {"optionId":"allow", "name":"Allow once", "kind":"allow_once"},
                    {"optionId":"reject", "name":"Reject once", "kind":"reject_once"}
                ]
            })
            assert permission["result"]["outcome"] == {"outcome":"selected", "optionId":"allow"}
            outside = call(105, "session/request_permission", {
                "sessionId":"fake-session", "toolCall":{"toolCallId":"tool-2","kind":"delete","locations":[{"path":"/tmp/outside"}]},
                "options":[
                    {"optionId":"allow", "name":"Allow once", "kind":"allow_once"},
                    {"optionId":"reject", "name":"Reject once", "kind":"reject_once"}
                ]
            })
            assert outside["result"]["outcome"] == {"outcome":"selected", "optionId":"reject"}
            unscoped = call(113, "session/request_permission", {
                "sessionId":"fake-session", "toolCall":{"toolCallId":"tool-3","kind":"read"},
                "options":[
                    {"optionId":"allow", "name":"Allow once", "kind":"allow_once"},
                    {"optionId":"reject", "name":"Reject once", "kind":"reject_once"}
                ]
            })
            assert unscoped["result"]["outcome"] == {"outcome":"selected", "optionId":"reject"}
            long_terminal = call(107, "terminal/create", {
                "sessionId":"fake-session", "command":"/bin/sh",
                "args":["-c", "printf started; sleep 60"], "cwd":cwd
            })["result"]["terminalId"]
            time.sleep(0.1)
            running = call(108, "terminal/output", {
                "sessionId":"fake-session", "terminalId":long_terminal
            })
            assert "started" in running["result"]["output"]
            assert "exitStatus" not in running["result"]
            call(109, "terminal/kill", {
                "sessionId":"fake-session", "terminalId":long_terminal
            })
            killed = call(110, "terminal/output", {
                "sessionId":"fake-session", "terminalId":long_terminal
            })
            assert "exitStatus" in killed["result"]
            call(111, "terminal/release", {
                "sessionId":"fake-session", "terminalId":long_terminal
            })
        prompt_text = "".join(
            item.get("text", "") for item in request.get("params", {}).get("prompt", [])
        )
        text = '{"ok":true}' if '"ok":true' in prompt_text else '{"status":"implemented"}'
        if scenario == "invalid-result":
            text = '```json\n{"status":"implemented"}\n```'
        if scenario == "oversized":
            text = '{"value":"' + ('x' * 4096) + '"}'
        if read_escape is not None:
            text = json.dumps(read_escape, separators=(",", ":"))
        send({"jsonrpc":"2.0","method":"session/update","params":{
            "sessionId":"wrong-session" if scenario == "wrong-session" else "fake-session",
            "update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":text}}
        }})
        send({"jsonrpc":"2.0","id":request_id,"result":{"stopReason":"end_turn"}})
    elif method == "session/cancel":
        continue
    elif method == "session/close":
        send({"jsonrpc":"2.0","id":request_id,"result":{}})

if scenario == "linger-close":
    time.sleep(60)
