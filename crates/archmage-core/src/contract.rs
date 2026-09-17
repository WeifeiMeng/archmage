use crate::{
    capability::CapabilityKind,
    error::{Error, Result},
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

#[derive(Clone, Debug)]
pub struct RequiredCheck {
    pub check_id: String,
    pub evidence_id: String,
    pub passed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ContractContext {
    pub required_checks: Vec<RequiredCheck>,
    pub checks_failed: bool,
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
    checks: Vec<CheckResult>,
}
#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Passed,
    Failed,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckResult {
    check_id: String,
    evidence_id: String,
    status: CheckStatus,
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
            let expected: BTreeMap<_, _> = context
                .required_checks
                .iter()
                .map(|c| (c.check_id.as_str(), c))
                .collect();
            if v.checks.len() != expected.len() {
                return Err(invalid("required checks do not match"));
            }
            for check in &v.checks {
                let required = expected
                    .get(check.check_id.as_str())
                    .ok_or_else(|| invalid("unknown check ID"))?;
                if check.evidence_id != required.evidence_id
                    || (check.status == CheckStatus::Passed) != required.passed
                {
                    return Err(invalid("check evidence mismatch"));
                }
            }
            let all_passed = context.required_checks.iter().all(|c| c.passed);
            if (v.status == CheckStatus::Passed) != all_passed {
                return Err(invalid("tester status contradicts evidence"));
            }
        }
        CapabilityKind::Reviewer => {
            let v: Reviewer =
                serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
            version(&v.schema_version)?;
            nonblank(&v.summary_markdown, "summary_markdown")?;
            if context.checks_failed && v.status == ReviewStatus::Approved {
                return Err(invalid("failed checks cannot be approved"));
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
