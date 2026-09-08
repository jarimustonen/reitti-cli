use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct AppError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) invalid_value: Option<String>,
    pub(crate) expected: Option<Box<Value>>,
    pub(crate) retryable: bool,
    pub(crate) details: Box<Map<String, Value>>,
    pub(crate) exit: u8,
}

impl AppError {
    pub fn caller(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            invalid_value: None,
            expected: None,
            retryable: false,
            details: Box::default(),
            exit: 1,
        }
    }

    pub fn system(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            exit: 2,
            ..Self::caller(code, message)
        }
    }

    pub fn invalid(
        code: &'static str,
        message: impl Into<String>,
        value: impl Into<String>,
        expected: impl Into<Value>,
    ) -> Self {
        Self {
            invalid_value: Some(value.into()),
            expected: Some(Box::new(expected.into())),
            ..Self::caller(code, message)
        }
    }

    pub fn io(context: &str, error: &std::io::Error) -> Self {
        Self::system("io_error", format!("{context}: {error}"))
    }

    pub fn feature_incomplete(command: &str) -> Self {
        Self::system(
            "feature_incomplete",
            format!("Command '{command}' is not implemented in this foundation build."),
        )
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

#[derive(Serialize)]
pub struct ErrorDocument<'a> {
    pub schema_version: u8,
    pub error: ErrorBody<'a>,
}

#[derive(Serialize)]
pub struct ErrorBody<'a> {
    pub code: &'a str,
    pub message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_value: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<&'a Value>,
    pub retryable: bool,
    pub details: &'a Map<String, Value>,
}

impl<'a> From<&'a AppError> for ErrorDocument<'a> {
    fn from(error: &'a AppError) -> Self {
        Self {
            schema_version: 1,
            error: ErrorBody {
                code: error.code,
                message: &error.message,
                invalid_value: error.invalid_value.as_deref(),
                expected: error.expected.as_deref(),
                retryable: error.retryable,
                details: &error.details,
            },
        }
    }
}
