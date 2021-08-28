use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScpiError {
    CommandError,
    ExecutionError,
    DevDependError,
    QueryError,
}

impl std::fmt::Display for ScpiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ScpiError::*;
        match self {
            CommandError => write!(f, "command error"),
            ExecutionError => write!(f, "execution error"),
            DevDependError => write!(f, "device-dependent error"),
            QueryError => write!(f, "query error"),
        }
    }
}
