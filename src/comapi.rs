use async_std::task;
use log::debug;

use async_std::channel::Sender;
use windows::{
    core::{implement, AgileReference, AsImpl, Result},
    Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Media::Audio::{
            eConsole, eRender,
            Endpoints::{
                IAudioEndpointVolume, IAudioEndpointVolumeCallback,
                IAudioEndpointVolumeCallback_Impl,
            },
            IMMDevice, IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl,
            MMDeviceEnumerator,
        },
        System::Com::{
            // CoUninitialize,
            CoCreateInstance,
            CoInitializeEx,
            CoUninitialize,
            CLSCTX_ALL,
            CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED,
            STGM_READ,
        },
    },
};

use super::enums;

#[derive(Debug)]
pub struct VolumeChangeEvent {
    pub friendly_name: String,
    pub id: String,
    pub mute: bool,
    pub volume: f32,
    pub channel_volumes: Box<[f32]>,
}

#[implement(IAudioEndpointVolumeCallback)]
pub struct VolumeCallbackClient {
    pub friendly_name: String,
    pub id: String,
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

        let friendly_name;
        unsafe {
            let properties = device.OpenPropertyStore(STGM_READ)?;
            let prop = properties.GetValue(&PKEY_Device_FriendlyName)?;
            friendly_name = prop
                .Anonymous
                .Anonymous
                .Anonymous
                .pwszVal
                .to_string()
                .expect("invalid UTF-16 display name")
        }
        let val = VolumeCallbackClient {
            endpoint: endpoint.clone(),
            friendly_name,
            id: unsafe {
                device
                    .GetId()?
                    .to_string()
                    .expect("invalid UTF-16 device ID")
            },
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
            id: self.id.clone(),
            friendly_name: self.friendly_name.clone(),
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

#[implement(IMMNotificationClient)]
pub struct NoticationClient {}

impl IMMNotificationClient_Impl for NoticationClient {
    fn OnDeviceStateChanged(
        &self,
        _deviceid: &windows::core::PCWSTR,
        _dwnewstate: u32,
    ) -> Result<()> {
        // eprintln!("OnDeviceStateChanged called {}", unsafe {
        //     deviceid.display()
        // });
        Ok(())
    }

    fn OnDeviceAdded(&self, _pwstrdeviceid: &windows::core::PCWSTR) -> Result<()> {
        // eprintln!("OnDeviceAdded called");
        Ok(())
    }

    fn OnDeviceRemoved(&self, _pwstrdeviceid: &windows::core::PCWSTR) -> Result<()> {
        // eprintln!("OnDeviceRemoved called");
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        flow: windows::Win32::Media::Audio::EDataFlow,
        role: windows::Win32::Media::Audio::ERole,
        device_id: &windows::core::PCWSTR,
    ) -> Result<()> {
        unsafe {
            debug!(
                "DefaultDeviceChanged flow: {:?} role: {:?}, device: {} ",
                enums::EDataFlow(flow),
                enums::ERole(role),
                device_id.display()
            )
        };
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _pwstrdeviceid: &windows::core::PCWSTR,
        _key: &windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY,
    ) -> Result<()> {
        // eprintln!("OnPropertyValueChanged {:#?},{} called", key.fmtid, key.pid);
        Ok(())
    }
}

pub struct AudioEventListener {
    // device_enumerator: IMMDeviceEnumerator,
    volume_listeners: Vec<AgileReference<IAudioEndpointVolumeCallback>>,
    channel: Sender<VolumeChangeEvent>,
}

impl AudioEventListener {
    pub fn new(channel: Sender<VolumeChangeEvent>) -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)?;
        }

        let mut obj = AudioEventListener {
            volume_listeners: Vec::new(),
            channel,
        };

        let device = obj.get_default_output_device()?;
        obj.register_volume_change(device)?;
        Ok(obj)
    }

    pub fn get_default_output_device(&self) -> Result<IMMDevice> {
        unsafe {
            let device_enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
            device_enumerator.GetDefaultAudioEndpoint(eRender, eConsole)
        }
    }

    fn register_volume_change(&mut self, device: IMMDevice) -> Result<()> {
        let vcallback = VolumeCallbackClient::new(&device, self.channel.clone())?;

        self.volume_listeners.push(AgileReference::new(&vcallback)?);

        Ok(())
    }

    /// Stop listening for changes
    pub fn stop(&mut self) {
        for agile_ref in self.volume_listeners.iter() {
            unsafe {
                if let Ok(interface) = agile_ref.resolve() {
                    let cb = interface.as_impl();
                    debug!("Stop listening to changes from {:?}", cb.friendly_name);
                    let _ = cb.endpoint.UnregisterControlChangeNotify(&interface);
                }
            }
        }
        self.volume_listeners.clear()
    }
}

impl Drop for AudioEventListener {
    fn drop(&mut self) {
        debug!("Dropping <AudioEventListener {:p}>", self);
        self.stop();
        unsafe { CoUninitialize() }
    }
}

#[cfg(test)]
mod test {

    use async_std::channel::bounded;

    use super::*;

    #[test]
    fn test_sync_send() {
        fn trait_check<T: Send>(_: T) {}
        let (tx, _rx) = bounded::<VolumeChangeEvent>(1);
        let listener = AudioEventListener::new(tx);
        trait_check(listener);
    }
}
