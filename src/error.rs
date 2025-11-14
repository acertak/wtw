use std::fmt;

use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum AppError {
    #[error("{0}")]
    User(String),
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Git(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn exit_code(&self) -> u8 {
        match self {
            AppError::User(_) => 1,
            AppError::Config(_) => 2,
            AppError::Git(_) => 3,
            AppError::Internal(_) => 10,
        }
    }

    pub fn user(message: impl Into<String>) -> Self {
        AppError::User(message.into())
    }

    pub fn config(message: impl Into<String>) -> Self {
        AppError::Config(message.into())
    }

    pub fn git(message: impl Into<String>) -> Self {
        AppError::Git(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        AppError::Internal(message.into())
    }

    pub fn internal_from(error: impl fmt::Display) -> Self {
        AppError::Internal(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_variants() {
        assert_eq!(AppError::user("u").exit_code(), 1);
        assert_eq!(AppError::config("c").exit_code(), 2);
        assert_eq!(AppError::git("g").exit_code(), 3);
        assert_eq!(AppError::internal("i").exit_code(), 10);
    }
}
