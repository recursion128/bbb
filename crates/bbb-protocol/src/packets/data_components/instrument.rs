use super::InstrumentSummary;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstrumentComponentSummary {
    pub registry_id: Option<i32>,
    pub direct: Option<Box<InstrumentSummary>>,
}
