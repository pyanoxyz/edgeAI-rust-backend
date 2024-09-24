
#[derive(Debug, Clone)]
pub enum RequestType {
    Infill,
    Chat,
    Refactor,
    TestCases,
}

// Convert enum variants to string
impl RequestType {
    fn as_str(&self) -> &str {
        match self {
            RequestType::Infill => "INFILL",
            RequestType::Chat => "CHAT",
            RequestType::Refactor => "REFACTOR",
            RequestType::TestCases => "TEST_CASES",
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            RequestType::Infill => "Text".to_string(),
            RequestType::Chat => "Chat".to_string(),
            RequestType::Refactor => "Refactor".to_string(),
            RequestType::TestCases => "TestCases".to_string(),
        }
    }
}
