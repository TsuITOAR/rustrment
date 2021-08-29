use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScpiError {
    #[error("command error")]
    CommandError,
    #[error("execution error")]
    ExecutionError,
    #[error("device-dependent error")]
    DevDependError,
    #[error("query error")]
    QueryError,
}
