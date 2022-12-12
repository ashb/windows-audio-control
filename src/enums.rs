///"Type" enums
use pyo3::prelude::*;

use num_enum::TryFromPrimitive;

use windows::Win32::Media::Audio::{
    eAll, eCapture, eCommunications, eConsole, eMultimedia, eRender, DEVICE_STATE_ACTIVE,
    DEVICE_STATE_DISABLED, DEVICE_STATE_NOTPRESENT, DEVICE_STATE_UNPLUGGED,
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

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Clone, Copy)]
#[pyclass(name = "DeviceState")]
#[repr(u32)]
pub enum DeviceState {
    #[pyo3(name = "ACTIVE")]
    Active = DEVICE_STATE_ACTIVE,
    #[pyo3(name = "DISABLED")]
    Disabled = DEVICE_STATE_DISABLED,
    #[pyo3(name = "NOT_PRESENT")]
    NotPresent = DEVICE_STATE_NOTPRESENT,
    #[pyo3(name = "UNPLUGGED")]
    Unplugged = DEVICE_STATE_UNPLUGGED,
}
