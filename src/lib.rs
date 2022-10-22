use async_std::channel::{bounded, Receiver, RecvError};
use pyo3::types::PyTuple;
use pyo3::{
    exceptions::{PyOSError, PyStopAsyncIteration},
    prelude::*,
};

mod comapi;
mod enums;

#[pyclass(module = "windows_audio_events", name = "VolumeChangeEvent")]
#[derive(Debug)]
pub struct PyVolumeChangeEvent {
    #[pyo3(get)]
    pub friendly_name: String,
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub mute: bool,
    #[pyo3(get)]
    pub volume: f32,
    pub channel_volumes: Box<[f32]>,
}

#[pymethods]
impl PyVolumeChangeEvent {
    #[getter]
    fn get_channel_volumes<'a>(&self, py: Python<'a>) -> &'a PyTuple {
        PyTuple::new(py, self.channel_volumes.iter())
    }

    fn __repr__(&self) -> String {
        format!(
            "<VolumChangeEvent friendly_name={:?} id={:?} mute={} volume={} channel_volumes={:?}",
            self.friendly_name, self.id, self.mute, self.volume, self.channel_volumes,
        )
    }
}

impl From<comapi::VolumeChangeEvent> for PyVolumeChangeEvent {
    fn from(e: comapi::VolumeChangeEvent) -> Self {
        PyVolumeChangeEvent {
            friendly_name: e.friendly_name,
            id: e.id,
            mute: e.mute,
            volume: e.volume,
            channel_volumes: e.channel_volumes,
        }
    }
}

#[pyclass(module = "windows_audio_events", subclass, unsendable)]
struct WindowsAudioEvents {
    rx: Receiver<comapi::VolumeChangeEvent>,
    // We don't use it, but want to keep it alive as long as we are
    _event_listener: comapi::AudioEventListener,
}

#[derive(Debug)]
struct PyWindowsErr(windows::core::Error);

impl From<windows::core::Error> for PyWindowsErr {
    fn from(err: windows::core::Error) -> Self {
        PyWindowsErr(err)
    }
}

impl std::fmt::Display for PyWindowsErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oh no!")
    }
}

impl From<PyWindowsErr> for PyErr {
    fn from(err: PyWindowsErr) -> PyErr {
        PyOSError::new_err(err.to_string())
    }
}

#[pymethods]
impl WindowsAudioEvents {
    #[new]
    pub fn new() -> PyResult<Self> {
        let (tx, rx) = bounded(1);
        let listener = comapi::AudioEventListener::new(tx).map_err(PyWindowsErr::from)?;
        Ok(Self {
            rx,
            _event_listener: listener,
        })
    }

    pub fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn _next_event<'a>(&'a mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let rx = self.rx.clone();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            match rx.recv().await {
                Ok(val) => {
                    let pyevent: PyVolumeChangeEvent = val.into();
                    Ok(Python::with_gil(|py| pyevent.into_py(py)))
                }
                Err(RecvError) => Err(PyStopAsyncIteration::new_err("sender closed")),
            }
        })
    }
}

/// Native implementation
#[pymodule]
fn windows_audio_events(_py: Python, m: &PyModule) -> PyResult<()> {
    pyo3_log::init();

    m.add_class::<WindowsAudioEvents>()?;
    Ok(())
}
