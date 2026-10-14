use crate::{
    capability::CapabilityKind,
    error::{Error, Result},
    team::TeamGraph,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Component, Path};

#[derive(Clone, Debug, Default)]
pub struct ContractContext {
    pub tester_accepted: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlowRouting {
    pub decision: String,
    pub next: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FlowEnvelope {
    deliverable: Value,
    routing: FlowRouting,
}

pub fn flow_output_schema(kind: CapabilityKind) -> Result<String> {
    let deliverable: Value =
        serde_json::from_str(kind.schema()).map_err(|error| invalid(error.to_string()))?;
    let schema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["deliverable", "routing"],
        "properties": {
            "deliverable": deliverable,
            "routing": {
                "type": "object",
                "additionalProperties": false,
                "required": ["decision", "next"],
                "properties": {
                    "decision": {"type": "string", "enum": ["continue", "complete"]},
                    "next": {"type": "array", "items": {"type": "string"}}
                }
            }
        }
    });
    serde_json::to_string(&schema).map_err(|error| invalid(error.to_string()))
}

pub fn validate_flow_output(
    kind: CapabilityKind,
    bytes: &[u8],
    context: &ContractContext,
    graph: &TeamGraph,
) -> Result<(Value, FlowRouting)> {
    let envelope: FlowEnvelope =
        serde_json::from_slice(bytes).map_err(|error| invalid(error.to_string()))?;
    let deliverable_bytes =
        serde_json::to_vec(&envelope.deliverable).map_err(|error| invalid(error.to_string()))?;
    let deliverable = validate_output(kind, &deliverable_bytes, context)?;
    let routing = envelope.routing;
    let from = kind.key();
    let unique: HashSet<&str> = routing.next.iter().map(String::as_str).collect();
    if unique.len() != routing.next.len() {
        return Err(invalid("routing recipients must be unique"));
    }
    match routing.decision.as_str() {
        "complete" => {
            if graph.terminal.as_deref() != Some(from) || !routing.next.is_empty() {
                return Err(invalid(
                    "only the terminal capability may complete without recipients",
                ));
            }
        }
        "continue" => {
            if routing.next.is_empty()
                || routing.next.iter().any(|target| {
                    !graph
                        .edges
                        .iter()
                        .any(|edge| edge.from == from && edge.to == *target)
                })
            {
                return Err(invalid("routing recipients must follow outgoing edges"));
            }
        }
        _ => return Err(invalid("routing decision must be continue or complete")),
    }
    if kind == CapabilityKind::Reviewer && graph.terminal.as_deref() == Some(kind.key()) {
        let approved = deliverable["status"] == "approved";
        if approved != (routing.decision == "complete") {
            return Err(invalid("review status and routing decision disagree"));
        }
    }
    if kind == CapabilityKind::Coder && deliverable["status"] == "blocked" {
        return Err(invalid("blocked coder output cannot route a flow"));
    }
    if kind == CapabilityKind::Tester && deliverable["status"] == "blocked" {
        return Err(invalid("blocked tester output cannot route a flow"));
    }
    if kind == CapabilityKind::Tester
        && deliverable["status"] == "failed"
        && routing.decision == "complete"
    {
        return Err(invalid("failed testing cannot complete a flow"));
    }
    Ok((deliverable, routing))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Designer {
    schema_version: String,
    summary_markdown: String,
    design_markdown: String,
    acceptance_criteria: Vec<String>,
    implementation_steps: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Coder {
    schema_version: String,
    summary_markdown: String,
    status: CoderStatus,
    changed_paths: Vec<String>,
    notes: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum CoderStatus {
    Implemented,
    Blocked,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tester {
    schema_version: String,
    summary_markdown: String,
    status: CheckStatus,
    commands: Vec<TestCommand>,
    limitations: Vec<String>,
}
#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Passed,
    Failed,
    Blocked,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TestCommand {
    command: String,
    exit_code: Option<i32>,
    status: CheckStatus,
    summary: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reviewer {
    schema_version: String,
    summary_markdown: String,
    status: ReviewStatus,
    issues: Vec<Issue>,
}
#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReviewStatus {
    Approved,
    ChangesRequested,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Issue {
    severity: Severity,
    path: String,
    message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsultationOutput {
    pub schema_version: String,
    pub decision: String,
    pub summary: String,
    pub suggestions: Vec<String>,
}

pub const CONSULTATION_SCHEMA: &str = include_str!("../capabilities/v1/consultation.schema.json");

pub fn validate_consultation(bytes: &[u8]) -> Result<ConsultationOutput> {
    let output: ConsultationOutput =
        serde_json::from_slice(bytes).map_err(|error| invalid(error.to_string()))?;
    version(&output.schema_version)?;
    nonblank(&output.summary, "consultation summary")?;
    if !matches!(output.decision.as_str(), "approved" | "changes_requested")
        || output.suggestions.iter().any(|item| item.trim().is_empty())
        || (output.decision == "changes_requested" && output.suggestions.is_empty())
        || (output.decision == "approved" && !output.suggestions.is_empty())
    {
        return Err(invalid("consultation decision and suggestions disagree"));
    }
    Ok(output)
}
#[derive(Deserialize)]
enum Severity {
    P0,
    P1,
    P2,
    P3,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Preflight(format!("contract_invalid: {}", message.into()))
}
fn version(value: &str) -> Result<()> {
    if value == "1" {
        Ok(())
    } else {
        Err(invalid("schema_version must be 1"))
    }
}
fn nonblank(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(invalid(format!("{field} must be nonblank")))
    } else {
        Ok(())
    }
}

pub fn validate_builtin_schemas() -> Result<()> {
    for kind in [
        CapabilityKind::Designer,
        CapabilityKind::Coder,
        CapabilityKind::Tester,
        CapabilityKind::Reviewer,
    ] {
        let value: Value =
            serde_json::from_str(kind.schema()).map_err(|e| invalid(e.to_string()))?;
        if value["type"] != "object" || value["additionalProperties"] != false {
            return Err(invalid("builtin schema root must be strict object"));
        }
        require_explicit_types(&value)?;
    }
    let value: Value =
        serde_json::from_str(CONSULTATION_SCHEMA).map_err(|e| invalid(e.to_string()))?;
    if value["type"] != "object" || value["additionalProperties"] != false {
        return Err(invalid("consultation schema root must be strict object"));
    }
    require_explicit_types(&value)?;
    Ok(())
}

fn require_explicit_types(value: &Value) -> Result<()> {
    match value {
        Value::Object(object) => {
            if (object.contains_key("const") || object.contains_key("enum"))
                && !object.contains_key("type")
            {
                return Err(invalid(
                    "const and enum schema nodes require an explicit type",
                ));
            }
            for child in object.values() {
                require_explicit_types(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                require_explicit_types(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn validate_output(
    kind: CapabilityKind,
    bytes: &[u8],
    context: &ContractContext,
) -> Result<Value> {
    let value: Value = serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
    match kind {
        CapabilityKind::Designer => {
            let v: Designer =
                serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
            version(&v.schema_version)?;
            nonblank(&v.summary_markdown, "summary_markdown")?;
            nonblank(&v.design_markdown, "design_markdown")?;
            if v.acceptance_criteria.is_empty()
                || v.acceptance_criteria.iter().any(|s| s.trim().is_empty())
                || v.implementation_steps.is_empty()
            {
                return Err(invalid("design criteria and steps must be nonempty"));
            }
        }
        CapabilityKind::Coder => {
            let v: Coder =
                serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
            version(&v.schema_version)?;
            nonblank(&v.summary_markdown, "summary_markdown")?;
            let _ = (v.status, v.changed_paths, v.notes);
        }
        CapabilityKind::Tester => {
            let v: Tester =
                serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
            version(&v.schema_version)?;
            nonblank(&v.summary_markdown, "summary_markdown")?;
            for command in &v.commands {
                nonblank(&command.command, "test command")?;
                nonblank(&command.summary, "test command summary")?;
                if (command.status == CheckStatus::Passed) != (command.exit_code == Some(0)) {
                    return Err(invalid("test command status contradicts exit code"));
                }
            }
            if v.limitations.iter().any(|item| item.trim().is_empty()) {
                return Err(invalid("tester limitations must be nonblank"));
            }
            match v.status {
                CheckStatus::Passed
                    if v.commands.is_empty()
                        || v.commands
                            .iter()
                            .any(|command| command.status != CheckStatus::Passed) =>
                {
                    return Err(invalid("passed testing requires successful commands"));
                }
                CheckStatus::Failed
                    if !v
                        .commands
                        .iter()
                        .any(|command| command.status == CheckStatus::Failed) =>
                {
                    return Err(invalid("failed testing requires a failed command"));
                }
                CheckStatus::Blocked if v.limitations.is_empty() => {
                    return Err(invalid("blocked testing requires a limitation"));
                }
                _ => {}
            }
        }
        CapabilityKind::Reviewer => {
            let v: Reviewer =
                serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
            version(&v.schema_version)?;
            nonblank(&v.summary_markdown, "summary_markdown")?;
            if !context.tester_accepted && v.status == ReviewStatus::Approved {
                return Err(invalid("failed or blocked testing cannot be approved"));
            }
            for issue in v.issues {
                let path = Path::new(&issue.path);
                if issue.path.is_empty()
                    || path.is_absolute()
                    || path.components().any(|c| {
                        matches!(
                            c,
                            Component::ParentDir | Component::RootDir | Component::Prefix(_)
                        )
                    })
                {
                    return Err(invalid("review issue path escapes workspace"));
                }
                nonblank(&issue.message, "issue message")?;
                let _ = issue.severity;
            }
        }
    }
    Ok(value)
}
