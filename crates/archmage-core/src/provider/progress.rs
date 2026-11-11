use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const MAX_PROGRESS_TEXT_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderProgress {
    pub kind: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub metadata: Map<String, Value>,
}

impl ProviderProgress {
    pub fn message(label: impl Into<String>, text: impl AsRef<str>) -> Self {
        Self {
            kind: "message".into(),
            label: label.into(),
            status: None,
            text: Some(bounded_text(text.as_ref())),
            metadata: Map::new(),
        }
    }

    fn new(kind: &str, label: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            label: label.into(),
            status: None,
            text: None,
            metadata: Map::new(),
        }
    }
}

pub fn codex_progress(event: &Value) -> Option<ProviderProgress> {
    match event.get("type")?.as_str()? {
        "item.started" | "item.updated" | "item.completed" => {
            let item = event.get("item")?;
            let lifecycle = event["type"].as_str()?.strip_prefix("item.")?;
            match item.get("type")?.as_str()? {
                "agent_message" => item
                    .get("text")
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                    .map(|text| ProviderProgress::message("Agent message", text)),
                "command_execution" => {
                    let mut progress = ProviderProgress::new("tool", "Run command");
                    progress.status = item
                        .get("status")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .or_else(|| Some(lifecycle.to_owned()));
                    progress.text = item
                        .get("command")
                        .and_then(Value::as_str)
                        .map(bounded_text);
                    copy_primitive(item, &mut progress.metadata, "exit_code");
                    Some(progress)
                }
                "file_change" => {
                    let mut progress = ProviderProgress::new("tool", "File changes");
                    progress.status = item
                        .get("status")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .or_else(|| Some(lifecycle.to_owned()));
                    Some(progress)
                }
                "mcp_tool_call" => {
                    let label = item
                        .get("tool")
                        .and_then(Value::as_str)
                        .filter(|value| !value.is_empty())
                        .unwrap_or("MCP tool");
                    let mut progress = ProviderProgress::new("tool", label);
                    progress.status = item
                        .get("status")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .or_else(|| Some(lifecycle.to_owned()));
                    copy_primitive(item, &mut progress.metadata, "server");
                    Some(progress)
                }
                _ => None,
            }
        }
        "turn.started" => Some(ProviderProgress {
            kind: "status".into(),
            label: "Turn started".into(),
            status: Some("running".into()),
            text: None,
            metadata: Map::new(),
        }),
        "turn.completed" => {
            let mut progress = ProviderProgress::new("usage", "Turn completed");
            progress.status = Some("completed".into());
            if let Some(usage) = event.get("usage") {
                for key in [
                    "input_tokens",
                    "cached_input_tokens",
                    "output_tokens",
                    "reasoning_output_tokens",
                ] {
                    copy_primitive(usage, &mut progress.metadata, key);
                }
            }
            Some(progress)
        }
        _ => None,
    }
}

pub fn qoder_progress(frame: &Value) -> Option<ProviderProgress> {
    let update = frame.pointer("/params/update")?;
    match update.get("sessionUpdate")?.as_str()? {
        "agent_message_chunk" => update
            .pointer("/content/text")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| ProviderProgress::message("Agent message", text)),
        "plan" => {
            let mut progress = ProviderProgress::new("plan", "Plan updated");
            progress.status = update
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_owned);
            progress.text = update
                .get("title")
                .or_else(|| update.get("text"))
                .and_then(Value::as_str)
                .map(bounded_text);
            Some(progress)
        }
        "tool_call" | "tool_call_update" => {
            let label = update
                .get("title")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .unwrap_or("Tool call");
            let mut progress = ProviderProgress::new("tool", label);
            progress.status = update
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_owned);
            copy_primitive(update, &mut progress.metadata, "toolCallId");
            Some(progress)
        }
        "current_mode_update" | "config_option_update" => {
            let mut progress = ProviderProgress::new("status", "Session configuration updated");
            copy_primitive(update, &mut progress.metadata, "modeId");
            copy_primitive(update, &mut progress.metadata, "configId");
            Some(progress)
        }
        "usage_update" => {
            let mut progress = ProviderProgress::new("usage", "Usage updated");
            for key in ["inputTokens", "outputTokens", "totalTokens"] {
                copy_primitive(update, &mut progress.metadata, key);
            }
            Some(progress)
        }
        "available_commands_update" => None,
        "user_message_chunk" | "agent_thought_chunk" => None,
        "notice" => update
            .get("message")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| ProviderProgress::message("Provider notice", text))
            .map(|mut progress| {
                progress.kind = "notice".into();
                progress
            }),
        _ => None,
    }
}

fn bounded_text(value: &str) -> String {
    if value.len() <= MAX_PROGRESS_TEXT_BYTES {
        return value.to_owned();
    }
    let mut end = MAX_PROGRESS_TEXT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn copy_primitive(source: &Value, target: &mut Map<String, Value>, key: &str) {
    if let Some(value) = source.get(key)
        && (value.is_string() || value.is_number() || value.is_boolean())
    {
        target.insert(key.to_owned(), value.clone());
    }
}
