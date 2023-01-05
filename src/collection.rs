use std::sync::Arc;

use anyhow::Context;
use async_std::task;
use log::debug;

use async_std::channel::Sender;
use windows::{
    core::{implement, AgileReference, Result, PCWSTR},
    Win32::{
        Media::Audio::{
            eConsole, IMMDeviceCollection, IMMDeviceEnumerator, IMMNotificationClient,
            IMMNotificationClient_Impl, MMDeviceEnumerator,
        },
        System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
    },
};

use crate::{com, device::AudioDevice};

use super::device;
use super::enums;
use super::errors::WindowsAudioError;

#[derive(Debug)]
pub enum DeviceNotificationEvent {
    StateChanged(String, enums::DeviceState),
    Added(String),
    Removed(String),
    DefaultChanged(String, enums::DataFlow, enums::Role),
}

#[implement(IMMNotificationClient)]
pub struct NotificationClient {
    channel: Sender<anyhow::Result<DeviceNotificationEvent>>,
}

impl NotificationClient {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        rx: Sender<anyhow::Result<DeviceNotificationEvent>>,
    ) -> anyhow::Result<IMMNotificationClient> {
        let val = NotificationClient { channel: rx };

        Ok(val.into())
    }
}

impl IMMNotificationClient_Impl for NotificationClient {
    fn OnDeviceStateChanged(
        &self,
        deviceid: &windows::core::PCWSTR,
        dwnewstate: u32,
    ) -> Result<()> {
        fn process(
            win_device_id: &windows::core::PCWSTR,
            dwnewstate: u32,
        ) -> anyhow::Result<DeviceNotificationEvent> {
            let device_id = unsafe { win_device_id.to_string()? };
            let new_state = enums::DeviceState::try_from(dwnewstate)?;

            Ok(DeviceNotificationEvent::StateChanged(device_id, new_state))
        }

        let msg = process(deviceid, dwnewstate)
            .context("Failed to convert OnDeviceStateChange to expected types");

        let channel = self.channel.clone();
        task::spawn(async move {
            _ = channel.send(msg).await;
        });
        Ok(())
    }

    fn OnDeviceAdded(&self, win_device_id: &windows::core::PCWSTR) -> Result<()> {
        let device_id = unsafe { win_device_id.to_string()? };

        let channel = self.channel.clone();
        task::spawn(async move {
            _ = channel
                .send(Ok(DeviceNotificationEvent::Added(device_id)))
                .await;
        });
        Ok(())
    }

    fn OnDeviceRemoved(&self, win_device_id: &windows::core::PCWSTR) -> Result<()> {
        let device_id = unsafe { win_device_id.to_string()? };

        let channel = self.channel.clone();
        task::spawn(async move {
            _ = channel
                .send(Ok(DeviceNotificationEvent::Removed(device_id)))
                .await;
        });
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        flow: windows::Win32::Media::Audio::EDataFlow,
        role: windows::Win32::Media::Audio::ERole,
        device_id: &windows::core::PCWSTR,
    ) -> Result<()> {
        fn process(
            win_device_id: &windows::core::PCWSTR,
            flow: &windows::Win32::Media::Audio::EDataFlow,
            role: &windows::Win32::Media::Audio::ERole,
        ) -> anyhow::Result<DeviceNotificationEvent> {
            let device_id = unsafe { win_device_id.to_string()? };
            let flow = enums::DataFlow::try_from(flow.0)?;
            let role = enums::Role::try_from(role.0)?;

            Ok(DeviceNotificationEvent::DefaultChanged(
                device_id, flow, role,
            ))
        }

        let msg = process(device_id, &flow, &role)
            .context("Failed to convert OnDefaultDeviceChanged to expected types");

        let channel = self.channel.clone();
        task::spawn(async move {
            _ = channel.send(msg).await;
        });
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _pwstrdeviceid: &windows::core::PCWSTR,
        _key: &windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY,
    ) -> Result<()> {
        Ok(())
    }
}

pub struct DeviceCollection(Arc<IMMDeviceCollection>);

// This feels like a bad idea, but AgileReference doesn't work for IMMDeviceCollection
unsafe impl Send for DeviceCollection {}
unsafe impl Sync for DeviceCollection {}

impl DeviceCollection {
    pub fn length(&self) -> anyhow::Result<u32> {
        Ok(unsafe { self.0.GetCount()? })
    }

    pub fn get(&self, idx: u32) -> anyhow::Result<device::AudioDevice> {
        let device = unsafe { self.0.Item(idx)? };
        device::AudioDevice::new(device)
    }
}

pub struct DeviceEnumerator(AgileReference<IMMDeviceEnumerator>);

impl DeviceEnumerator {
    pub fn new() -> Result<Self> {
        let device_enumerator: IMMDeviceEnumerator;
        unsafe {
            com::com_initialized();

            device_enumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
        }

        Ok(DeviceEnumerator(AgileReference::new(&device_enumerator)?))
    }

    pub fn get_device(&self, device_id: &str) -> anyhow::Result<AudioDevice> {
        match self.0.resolve() {
            Ok(enumerator) => {
                let mut text = device_id.encode_utf16().collect::<Vec<_>>();
                text.push(0);
                let wstr = PCWSTR::from_raw(text.as_ptr());

                match unsafe { enumerator.GetDevice(wstr) } {
                    Ok(device) => device::AudioDevice::new(device),
                    Err(e) => Err(WindowsAudioError::from(e).into()),
                }
            }
            Err(e) => Err(WindowsAudioError::from(e).into()),
        }
    }

    pub fn get_collection(
        &self,
        dataflow: enums::DataFlow,
        state_mask: enums::DeviceState,
    ) -> anyhow::Result<DeviceCollection> {
        match self.0.resolve() {
            Ok(enumerator) => {
                let collection =
                    unsafe { enumerator.EnumAudioEndpoints(dataflow.into(), state_mask.into()) }
                        .context("unable to get collection")?;

                Ok(DeviceCollection(Arc::new(collection)))
            }
            Err(e) => Err(WindowsAudioError::from(e).into()),
        }
    }

    pub fn get_default_device(
        &self,
        dataflow: windows::Win32::Media::Audio::EDataFlow,
    ) -> anyhow::Result<device::AudioDevice> {
        match self.0.resolve() {
            Ok(enumerator) => {
                let device = unsafe { enumerator.GetDefaultAudioEndpoint(dataflow, eConsole)? };
                device::AudioDevice::new(device)
            }
            Err(e) => Err(WindowsAudioError::from(e).into()),
        }
    }

    pub fn register_notification(&self, client: &IMMNotificationClient) -> anyhow::Result<()> {
        debug!("Registering notification client {:?}", client);
        let enumerator = self.0.resolve()?;
        unsafe { enumerator.RegisterEndpointNotificationCallback(client)? };
        Ok(())
    }

    pub fn unregister_notification(&self, client: &IMMNotificationClient) -> anyhow::Result<()> {
        debug!("Unregistering notification client {:?}", client);
        let enumerator = self.0.resolve()?;
        unsafe { enumerator.UnregisterEndpointNotificationCallback(client)? };
        Ok(())
    }
}
