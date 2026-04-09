use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct AppError {
  pub code: String,
  pub message: String,
  pub field: Option<String>,
}

impl AppError {
  pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
    Self {
      code: code.into(),
      message: message.into(),
      field: None,
    }
  }

  pub fn field(
    code: impl Into<String>,
    message: impl Into<String>,
    field: impl Into<String>,
  ) -> Self {
    Self {
      code: code.into(),
      message: message.into(),
      field: Some(field.into()),
    }
  }
}

impl From<anyhow::Error> for AppError {
  fn from(value: anyhow::Error) -> Self {
    Self::new("internal_error", value.to_string())
  }
}

