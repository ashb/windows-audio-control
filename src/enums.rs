///"Type" enums
use pyo3::{exceptions::PyValueError, prelude::*};

use bitflags::bitflags;
use num_enum::TryFromPrimitive;

use windows::Win32::Media::Audio::{
    eAll, eCapture, eCommunications, eConsole, eMultimedia, eRender, EDataFlow,
    DEVICE_STATEMASK_ALL, DEVICE_STATE_ACTIVE, DEVICE_STATE_DISABLED, DEVICE_STATE_NOTPRESENT,
    DEVICE_STATE_UNPLUGGED,
};

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Clone, Copy)]
#[pyclass(name = "DataFlow")]
#[repr(i32)]
pub enum DataFlow {
    #[pyo3(name = "RENDER")]
    Render = eRender.0,
    #[pyo3(name = "CAPTURE")]
    Capture = eCapture.0,
    #[pyo3(name = "ALL")]
    All = eAll.0,
}

impl From<DataFlow> for EDataFlow {
    fn from(e: DataFlow) -> Self {
        EDataFlow(e as i32)
    }
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Clone, Copy)]
#[pyclass(name = "Role")]
#[repr(i32)]
pub enum Role {
    #[pyo3(name = "CONSOLE")]
    Console = eConsole.0,
    #[pyo3(name = "COMMS")]
    Communications = eCommunications.0,
    #[pyo3(name = "MULTIMEDIA")]
    Multimedia = eMultimedia.0,
}

bitflags! {
    #[derive(Debug, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceState: u32 {
        const Active = DEVICE_STATE_ACTIVE;
        const Disabled = DEVICE_STATE_DISABLED;
        const NotPresent = DEVICE_STATE_NOTPRESENT;
        const Unplugged = DEVICE_STATE_UNPLUGGED;
        const All = DEVICE_STATEMASK_ALL;
    }
}

impl DeviceState {
    pub fn py_name(self) -> &'static str {
        match self {
            DeviceState::Active => "ACTIVE",
            DeviceState::Disabled => "DISABLED",
            DeviceState::NotPresent => "NOT_PRESENT",
            DeviceState::Unplugged => "UNPLUGGED",
            DeviceState::All => "ALL",
            _ => panic!("Unexpected value"),
        }
    }
}

impl From<DeviceState> for u32 {
    fn from(value: DeviceState) -> u32 {
        value.bits()
    }
}
impl From<u32> for DeviceState {
    fn from(value: u32) -> DeviceState {
        DeviceState::from_bits_truncate(value)
    }
}

impl ToPyObject for DeviceState {
    fn to_object(&self, py: Python) -> PyObject {
        self.bits().into_py(py)
    }
}

impl IntoPy<PyObject> for DeviceState {
    fn into_py(self, py: Python) -> PyObject {
        self.to_object(py)
    }
}

impl<'source> FromPyObject<'source> for DeviceState {
    fn extract(obj: &'source PyAny) -> PyResult<Self> {
        let raw = u32::extract(obj)?;
        match DeviceState::from_bits(raw) {
            None => Err(PyValueError::new_err(format!(
                "Failed to extract `DeviceState` from {:?}",
                raw
            ))),
            Some(flags) => Ok(flags),
        }
    }
}
