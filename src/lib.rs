use std::sync::Arc;

use anyhow::{Context, Result};
use async_std::channel::{bounded, Receiver, RecvError};
use collection::NotificationClient;
use log::debug;
use pyo3::types::PyTuple;
use pyo3::{exceptions::PyStopAsyncIteration, prelude::*};
use windows::Win32::Media::Audio::IMMNotificationClient;

mod collection;
mod com;
mod device;
mod enums;
mod errors;

#[pyclass(module = "windows_audio_events", name = "VolumeChangeEvent")]
#[derive(Debug)]
pub struct PyVolumeChangeEvent {
    /// :rtype: AudioDevice
    #[pyo3(get)]
    device: Py<PyAudioDevice>,

    /// :rtype: bool
    #[pyo3(get)]
    pub mute: bool,

    /// :rtype: float
    #[pyo3(get)]
    pub volume: f32,

    channel_volumes: Box<[f32]>,
}

#[pymethods]
impl PyVolumeChangeEvent {
    /// :rtype: tuple(float, ...)
    #[getter]
    fn get_channel_volumes<'a>(&self, py: Python<'a>) -> &'a PyTuple {
        PyTuple::new(py, self.channel_volumes.iter())
    }

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let device = self.device.as_ref(py);
        Ok(format!(
            "<VolumChangeEvent device={} mute={} volume={} channel_volumes={:?}",
            device.repr()?,
            self.mute,
            self.volume,
            self.channel_volumes,
        ))
    }
}

impl PyVolumeChangeEvent {
    fn new(device: Py<PyAudioDevice>, e: device::VolumeChangeEvent) -> Self {
        PyVolumeChangeEvent {
            device,
            mute: e.mute,
            volume: e.volume,
            channel_volumes: e.channel_volumes,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
enum DeviceCollectionEventType {
    #[pyo3(name = "STATE_CHANGED")]
    StateChanged,
    #[pyo3(name = "ADDED")]
    Added,
    #[pyo3(name = "REMOVED")]
    Removed,
    #[pyo3(name = "DEFAULT_CHANGED")]
    DefaultChanged,
}

#[pyclass(name = "DeviceCollectionEvent")]
#[derive(Clone, Debug)]
struct PyDeviceCollectionEvent {
    /// :rtype: DeviceCollectionEventType
    #[pyo3(get)]
    kind: DeviceCollectionEventType,

    /// :rtype: str
    #[pyo3(get)]
    device_id: String,

    /// The new state of device.
    ///
    /// Only valid fvor STATE_CHANGED events
    ///
    /// :rtype: DeviceState | None
    #[pyo3(get)]
    state: Option<enums::DeviceState>,

    /// :rtype: DataFlow | None
    #[pyo3(get)]
    dataflow: Option<enums::DataFlow>,

    /// :rtype: Role | None
    #[pyo3(get)]
    role: Option<enums::Role>,
}

#[pymethods]
impl PyDeviceCollectionEvent {
    pub fn __repr__(&self, py: Python) -> Result<String> {
        let mut repr = format!(
            "<DeviceCollectionEvent kind={} device_id='{}'",
            self.kind.__pyo3__repr__(),
            self.device_id,
        );

        if let Some(state) = self.state {
            let pyobj = state.into_py(py);
            let s = pyobj.as_ref(py).repr()?;
            repr.push_str(&format!(" state={}", s));
        }
        if let Some(flow) = self.dataflow {
            let pyobj = flow.into_py(py);
            let s = pyobj.as_ref(py).repr()?;
            repr.push_str(&format!(" dataflow={}", s));
        }
        if let Some(role) = self.role {
            let pyobj = role.into_py(py);
            let s = pyobj.as_ref(py).repr()?;
            repr.push_str(&format!(" role={}", s));
        }
        repr.push('>');

        Ok(repr)
    }
}

impl From<collection::DeviceNotificationEvent> for PyDeviceCollectionEvent {
    fn from(src: collection::DeviceNotificationEvent) -> Self {
        match src {
            collection::DeviceNotificationEvent::StateChanged(device_id, state) => {
                PyDeviceCollectionEvent {
                    kind: DeviceCollectionEventType::StateChanged,
                    device_id,
                    state: Some(state),
                    dataflow: None,
                    role: None,
                }
            }

            collection::DeviceNotificationEvent::DefaultChanged(device_id, flow, role) => {
                PyDeviceCollectionEvent {
                    kind: DeviceCollectionEventType::DefaultChanged,
                    device_id,
                    state: None,
                    dataflow: Some(flow),
                    role: Some(role),
                }
            }

            collection::DeviceNotificationEvent::Added(device_id) => PyDeviceCollectionEvent {
                kind: DeviceCollectionEventType::Added,
                device_id,
                state: None,
                dataflow: None,
                role: None,
            },

            collection::DeviceNotificationEvent::Removed(device_id) => PyDeviceCollectionEvent {
                kind: DeviceCollectionEventType::Removed,
                device_id,
                state: None,
                dataflow: None,
                role: None,
            },
        }
    }
}

#[pyclass(module = "windows_audio_events", subclass)]
struct DeviceCollection(Arc<collection::DeviceEnumerator>);

#[pymethods]
impl DeviceCollection {
    #[new]
    pub fn __new__() -> Result<Self> {
        Ok(DeviceCollection(Arc::new(
            collection::DeviceEnumerator::new()?,
        )))
    }

    /// :rtype: AudioDevice
    ///
    /// Get the current default output device (aka speakers)
    #[pyo3(text_signature = "($self)")]
    pub fn get_default_output_device(&self) -> Result<PyAudioDevice> {
        let dev = self
            .0
            .get_default_device(windows::Win32::Media::Audio::eRender)?;
        Ok(PyAudioDevice(dev))
    }

    /// Get the current default input device (aka microphone)
    ///
    /// :rtype: AudioDevice
    #[pyo3(text_signature = "($self)")]
    pub fn get_default_input_device(&self) -> Result<PyAudioDevice> {
        let dev = self
            .0
            .get_default_device(windows::Win32::Media::Audio::eCapture)?;
        Ok(PyAudioDevice(dev))
    }

    /// :rtype: CollectionEventsIterator
    ///
    ///Asyncronoysly yield the events for this collection (device added or reomved, default changed, etc)
    #[getter]
    pub fn events(slf: Py<Self>, py: Python<'_>) -> Result<CollectionEventsIterator> {
        let (tx, rx) = bounded(1);

        let source = NotificationClient::new(tx)?;

        slf.borrow_mut(py).0.register_notification(&source)?;

        Ok(CollectionEventsIterator {
            collection: slf,
            source: Some(source),
            rx,
        })
    }
}

impl Drop for DeviceCollection {
    fn drop(&mut self) {
        debug!("Dropping (Py)DeviceCollection");
    }
}

#[pyclass(module = "windows_audio_events", subclass, unsendable)]
/// Async iterator of changes to devices in a collection
struct CollectionEventsIterator {
    // Keep the collection alive as long as the iterator is
    collection: Py<DeviceCollection>,
    source: Option<IMMNotificationClient>,
    rx: Receiver<anyhow::Result<collection::DeviceNotificationEvent>>,
}

impl CollectionEventsIterator {
    fn _next_event<'a>(&'a mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let rx = self.rx.clone();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            match rx.recv().await {
                Ok(val) => {
                    let event = val?;
                    let pyevent: PyDeviceCollectionEvent = event.into();

                    Ok(Python::with_gil(|py| pyevent.into_py(py)))
                }
                Err(RecvError) => Err(PyStopAsyncIteration::new_err("device enumerator closed")),
            }
        })
    }
}

#[pymethods]
impl CollectionEventsIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// :rtype: DeviceCollectionEvent
    ///
    pub fn __anext__<'a>(&'a mut self, py: Python<'a>) -> PyResult<Option<&'a PyAny>> {
        match self._next_event(py) {
            Ok(event) => Ok(Some(event)),
            Err(err) => Err(err),
        }
    }

    /// Close the iterator
    #[pyo3(text_signature = "($self)")]
    pub fn close(&mut self, py: Python) -> Result<()> {
        if let Some(source) = self.source.as_ref() {
            let obj = self.collection.borrow(py);
            let collection = obj.0.as_ref();
            collection
                .unregister_notification(source)
                .context("Unable to close CollectionEventsIterator")?;
            self.source = None
        }
        Ok(())
    }
}

impl Drop for CollectionEventsIterator {
    fn drop(&mut self) {
        _ = Python::with_gil(|py| self.close(py));
    }
}

#[pyclass(module = "windows_audio_events", subclass, unsendable)]
/// Async iterator of changes to a device's volume
struct AudioDeviceEventIterator {
    // Keep the device alive so we can use it in `repr`, but don't create a _rust_ memory cycle
    #[pyo3(get)]
    device: Py<PyAudioDevice>,
    rx: Receiver<device::VolumeChangeEvent>,
}

impl AudioDeviceEventIterator {
    pub fn _next_event<'a>(&'a mut self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let rx = self.rx.clone();
        let device = self.device.clone();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            match rx.recv().await {
                Ok(val) => {
                    let pyevent = PyVolumeChangeEvent::new(device, val);
                    Ok(Python::with_gil(|py| pyevent.into_py(py)))
                }
                Err(RecvError) => Err(PyStopAsyncIteration::new_err("audio session closed")),
            }
        })
    }
}

#[pymethods]
impl AudioDeviceEventIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __anext__<'a>(&'a mut self, py: Python<'a>) -> PyResult<Option<&'a PyAny>> {
        match self._next_event(py) {
            Ok(event) => Ok(Some(event)),
            Err(err) => Err(err),
        }
    }
}

impl Drop for AudioDeviceEventIterator {
    // When the iterator goes out of scope, stop listening for changes
    fn drop(&mut self) {
        Python::with_gil(|py| {
            let mut d = self.device.borrow_mut(py);
            d.0.stop_listening()
        })
    }
}

#[pyclass(
    module = "windows_audio_events",
    name = "AudioDevice",
    subclass,
    unsendable
)]
struct PyAudioDevice(device::AudioDevice);

#[pymethods]
impl PyAudioDevice {
    #[pyo3(text_signature = "($self)")]
    pub fn toggle_mute(&self) -> Result<()> {
        self.0.toggle_mute()?;
        Ok(())
    }

    /// :rtype: str
    ///
    /// Device name
    #[getter]
    pub fn name(&self) -> Result<&String> {
        Ok(&self.0.friendly_name)
    }

    /// :rtype: str
    #[getter]
    pub fn device_id(&self) -> Result<&String> {
        Ok(&self.0.id)
    }

    pub fn __repr__(&self) -> Result<String> {
        Ok(format!(
            "<AudioDevice name='{}', id='{}'>",
            self.0.friendly_name, self.0.id,
        ))
    }

    #[getter]
    ///Asyncronoysly yield the events for this device (volume change etc)
    ///
    /// :rtype: AudioDeviceEventIterator
    pub fn events(slf: Py<Self>, py: Python<'_>) -> Result<AudioDeviceEventIterator> {
        let (tx, rx) = bounded(1);
        slf.borrow_mut(py).0.register_volume_change(tx)?;
        Ok(AudioDeviceEventIterator { rx, device: slf })
    }
}

/// Native implementation
#[pymodule]
fn _native(_py: Python, m: &PyModule) -> PyResult<()> {
    pyo3_log::init();

    m.add_class::<DeviceCollection>()?;
    m.add_class::<PyAudioDevice>()?;
    m.add_class::<AudioDeviceEventIterator>()?;

    m.add_class::<CollectionEventsIterator>()?;
    m.add_class::<DeviceCollectionEventType>()?;
    m.add_class::<PyDeviceCollectionEvent>()?;
    m.add_class::<PyVolumeChangeEvent>()?;
    m.add_class::<enums::DeviceState>()?;
    m.add_class::<enums::DataFlow>()?;
    m.add_class::<enums::Role>()?;
    Ok(())
}
