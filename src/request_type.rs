
#[derive(Debug)]
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
}
