use pyo3::{
    exceptions::{PyOSError, PyRuntimeError},
    PyErr,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WindowsAudioError {
    #[error(transparent)]
    WindowsErr(#[from] windows::core::Error),
    #[error(transparent)]
    Utf16StringError(#[from] std::string::FromUtf16Error),
}

impl From<WindowsAudioError> for PyErr {
    fn from(err: WindowsAudioError) -> Self {
        match err {
            WindowsAudioError::WindowsErr(e) => PyOSError::new_err(e.to_string()),
            _ => PyRuntimeError::new_err(err.to_string()),
        }
    }
}
