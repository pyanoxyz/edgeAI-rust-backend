
#[derive(Debug, Clone)]
pub enum RequestType {
    Chat,
    Refactor,
    TestCases,
    DocString,
    FindBugs,
    Explain

}

// Convert enum variants to string
impl RequestType {
    pub fn to_string(&self) -> &str {
        match self {
            RequestType::Explain => "EXPLAIN",
            RequestType::Chat => "CHAT",
            RequestType::Refactor => "REFACTOR",
            RequestType::TestCases => "TEST_CASES",
            RequestType::DocString => "DOCSTRING",
            RequestType::FindBugs => "FIND_BUGS",

        }
    }

}
