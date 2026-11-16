use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const MAX_PROGRESS_TEXT_BYTES: usize = 16 * 1024;
const MAX_PROGRESS_LABEL_BYTES: usize = 256;
const MAX_PROGRESS_STATUS_BYTES: usize = 128;
const MAX_PROGRESS_METADATA_ENTRIES: usize = 8;
const MAX_PROGRESS_METADATA_KEY_BYTES: usize = 64;
const MAX_PROGRESS_METADATA_VALUE_BYTES: usize = 256;

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

    pub(crate) fn sanitized(&self) -> Self {
        let mut metadata = Map::new();
        for (key, value) in self.metadata.iter().take(MAX_PROGRESS_METADATA_ENTRIES) {
            let value = match value {
                Value::String(value) => {
                    Value::String(bounded(value, MAX_PROGRESS_METADATA_VALUE_BYTES))
                }
                Value::Number(_) | Value::Bool(_) => value.clone(),
                _ => continue,
            };
            metadata.insert(bounded(key, MAX_PROGRESS_METADATA_KEY_BYTES), value);
        }
        Self {
            kind: match self.kind.as_str() {
                "message" | "plan" | "tool" | "notice" | "usage" | "status" => self.kind.clone(),
                _ => "status".into(),
            },
            label: bounded(&self.label, MAX_PROGRESS_LABEL_BYTES),
            status: self
                .status
                .as_deref()
                .map(|value| bounded(value, MAX_PROGRESS_STATUS_BYTES)),
            text: self.text.as_deref().map(bounded_text),
            metadata,
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
            let entries = update.get("entries")?.as_array()?;
            progress.status = entries
                .iter()
                .filter_map(|entry| entry.get("status").and_then(Value::as_str))
                .find(|status| *status == "in_progress")
                .or_else(|| {
                    entries
                        .iter()
                        .filter_map(|entry| entry.get("status").and_then(Value::as_str))
                        .find(|status| *status == "pending")
                })
                .or_else(|| {
                    entries
                        .iter()
                        .filter_map(|entry| entry.get("status").and_then(Value::as_str))
                        .next()
                })
                .map(str::to_owned);
            let summary = entries
                .iter()
                .filter_map(|entry| {
                    let content = entry.get("content")?.as_str()?;
                    let status = entry
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("pending");
                    Some(format!("[{status}] {content}"))
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !summary.is_empty() {
                progress.text = Some(bounded_text(&summary));
            }
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
        "current_mode_update" => {
            let mut progress = ProviderProgress::new("status", "Session configuration updated");
            copy_primitive(update, &mut progress.metadata, "currentModeId");
            Some(progress)
        }
        "config_option_update" => {
            let mut progress = ProviderProgress::new("status", "Session configuration updated");
            if let Some(count) = update
                .get("configOptions")
                .and_then(Value::as_array)
                .map(Vec::len)
            {
                progress.metadata.insert("optionCount".into(), count.into());
            }
            Some(progress)
        }
        "usage_update" => {
            let mut progress = ProviderProgress::new("usage", "Usage updated");
            for key in ["used", "size"] {
                copy_primitive(update, &mut progress.metadata, key);
            }
            if let Some(cost) = update.get("cost") {
                copy_primitive_as(cost, &mut progress.metadata, "amount", "costAmount");
                copy_primitive_as(cost, &mut progress.metadata, "currency", "costCurrency");
            }
            Some(progress)
        }
        "available_commands_update" => None,
        "user_message_chunk" | "agent_thought_chunk" => None,
        "notice" => {
            let title = update.get("title")?.as_str()?.trim();
            if title.is_empty() {
                return None;
            }
            let mut progress = ProviderProgress::new("notice", title);
            progress.status = update
                .get("severity")
                .and_then(Value::as_str)
                .map(str::to_owned);
            progress.text = update
                .get("description")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .map(bounded_text);
            Some(progress)
        }
        _ => None,
    }
}

fn bounded_text(value: &str) -> String {
    bounded(value, MAX_PROGRESS_TEXT_BYTES)
}

fn bounded(value: &str, maximum: usize) -> String {
    if value.len() <= maximum {
        return value.to_owned();
    }
    let mut end = maximum;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn copy_primitive(source: &Value, target: &mut Map<String, Value>, key: &str) {
    copy_primitive_as(source, target, key, key);
}

fn copy_primitive_as(
    source: &Value,
    target: &mut Map<String, Value>,
    source_key: &str,
    target_key: &str,
) {
    if let Some(value) = source.get(source_key)
        && (value.is_string() || value.is_number() || value.is_boolean())
    {
        target.insert(target_key.to_owned(), value.clone());
    }
}
