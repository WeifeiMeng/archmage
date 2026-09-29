#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityKind {
    Designer,
    Coder,
    Tester,
    Reviewer,
}

impl CapabilityKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Designer => "designer",
            Self::Coder => "coder",
            Self::Tester => "tester",
            Self::Reviewer => "reviewer",
        }
    }

    pub fn prompt(self) -> &'static str {
        match self {
            Self::Designer => include_str!("../capabilities/v1/designer.md"),
            Self::Coder => include_str!("../capabilities/v1/coder.md"),
            Self::Tester => include_str!("../capabilities/v1/tester.md"),
            Self::Reviewer => include_str!("../capabilities/v1/reviewer.md"),
        }
    }
    pub fn schema(self) -> &'static str {
        match self {
            Self::Designer => include_str!("../capabilities/v1/designer.schema.json"),
            Self::Coder => include_str!("../capabilities/v1/coder.schema.json"),
            Self::Tester => include_str!("../capabilities/v1/tester.schema.json"),
            Self::Reviewer => include_str!("../capabilities/v1/reviewer.schema.json"),
        }
    }
}
