pub trait RequestIdGenerator: Send + Sync {
    fn next(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct FixedRequestIds {
    value: String,
}

impl FixedRequestIds {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl RequestIdGenerator for FixedRequestIds {
    fn next(&self) -> String {
        self.value.clone()
    }
}
