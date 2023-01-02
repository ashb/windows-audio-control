use anyhow::Context;
use async_std::task;
use log::debug;

use async_std::channel::Sender;
use windows::{
    core::{implement, AgileReference, AsImpl, Result, PCWSTR},
    Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Media::Audio::{
            ERole,
            Endpoints::{
                IAudioEndpointVolume, IAudioEndpointVolumeCallback,
                IAudioEndpointVolumeCallback_Impl,
            },
            IMMDevice,
        },
        System::Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ},
    },
};

use crate::policy_config::{IPolicyConfig, PolicyConfig};

// use super::enums;
use super::errors::WindowsAudioError;

pub struct AudioDevice {
    pub id: String,
    pub friendly_name: String,
    device: IMMDevice,
    volume_listener: Option<AgileReference<IAudioEndpointVolumeCallback>>,
}

impl AudioDevice {
    pub fn new(device: IMMDevice) -> anyhow::Result<Self> {
        let friendly_name = unsafe {
            let properties = device.OpenPropertyStore(STGM_READ)?;
            let prop = properties.GetValue(&PKEY_Device_FriendlyName)?;
            prop.Anonymous
                .Anonymous
                .Anonymous
                .pwszVal
                .to_string()
                .map_err(WindowsAudioError::from)?
        };
        let id = unsafe {
            device
                .GetId()
                .context("Unable to get device ID")?
                .to_string()
                .map_err(WindowsAudioError::from)?
        };

        anyhow::Ok(AudioDevice {
            id,
            friendly_name,
            device,
            volume_listener: None,
        })
    }

    pub fn toggle_mute(&self) -> Result<()> {
        unsafe {
            let endpoint: IAudioEndpointVolume = self.device.Activate(CLSCTX_ALL, None)?;
            let current = endpoint.GetMute()?.as_bool();
            endpoint.SetMute(!current, std::ptr::null())?;
        };
        Ok(())
    }

    pub fn register_volume_change(&mut self, channel: Sender<VolumeChangeEvent>) -> Result<()> {
        let vcallback = VolumeCallbackClient::new(&self.device, channel)?;

        if self.volume_listener.is_some() {
            self.stop_listening()
        }

        self.volume_listener = Some(AgileReference::new(&vcallback)?);

        Ok(())
    }

    pub fn stop_listening(&mut self) {
        if let Some(agile_ref) = self.volume_listener.as_ref() {
            unsafe {
                if let Ok(interface) = agile_ref.resolve() {
                    let cb = interface.as_impl();
                    debug!("Stop listening to changes from {:?}", self.friendly_name);
                    let _ = cb.endpoint.UnregisterControlChangeNotify(&interface);
                }
            }
            self.volume_listener = None;
        }
    }

    pub fn set_default(&self, role: ERole) -> Result<()> {
        let mut text = self.id.encode_utf16().collect::<Vec<_>>();
        text.push(0);
        let wstr = PCWSTR::from_raw(text.as_ptr());
        unsafe {
            let policy_config: IPolicyConfig = CoCreateInstance(&PolicyConfig, None, CLSCTX_ALL)?;

            policy_config.SetDefaultEndpoint(wstr, role).ok()?;
        }

        Ok(())
    }
}

impl Drop for AudioDevice {
    fn drop(&mut self) {
        self.stop_listening()
    }
}

#[derive(Debug)]
pub struct VolumeChangeEvent {
    pub mute: bool,
    pub volume: f32,
    pub channel_volumes: Box<[f32]>,
}

#[implement(IAudioEndpointVolumeCallback)]
pub struct VolumeCallbackClient {
    endpoint: IAudioEndpointVolume,
    channel: Sender<VolumeChangeEvent>,
}

impl VolumeCallbackClient {
    #[allow(clippy::new_ret_no_self)]
    fn new(
        device: &IMMDevice,
        channel: Sender<VolumeChangeEvent>,
    ) -> Result<IAudioEndpointVolumeCallback> {
        let endpoint: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None)? };

        let val = VolumeCallbackClient {
            endpoint: endpoint.clone(),
            channel,
        };

        unsafe {
            let i_cb: IAudioEndpointVolumeCallback = val.into();
            endpoint.RegisterControlChangeNotify(&i_cb)?;
            Ok(i_cb)
        }
    }
}

impl IAudioEndpointVolumeCallback_Impl for VolumeCallbackClient {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn OnNotify(
        &self,
        pnotify: *mut windows::Win32::Media::Audio::AUDIO_VOLUME_NOTIFICATION_DATA,
    ) -> Result<()> {
        let notify;
        let volumes;
        unsafe {
            notify = *pnotify;
            // afChannelVolumes is defined as a array of 1, but it's actually an array of nChannels.
            let range = (*pnotify).afChannelVolumes.as_ptr_range();

            volumes = std::slice::from_raw_parts(range.start, (notify).nChannels as usize).into();
        }
        let event = VolumeChangeEvent {
            mute: notify.bMuted.as_bool(),
            volume: notify.fMasterVolume,
            channel_volumes: volumes,
        };

        let channel = self.channel.clone();
        // Send the event up, but don't block this thread waiting for the result
        task::spawn(async move { channel.send(event).await });

        Ok(())
    }
}
