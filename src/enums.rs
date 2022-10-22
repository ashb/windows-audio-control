use windows::Win32::Media::Audio::{
    self, eAll, eCapture, eCommunications, eConsole, eMultimedia, eRender,
};

pub(crate) struct EDataFlow(pub Audio::EDataFlow);

impl ::core::fmt::Debug for EDataFlow {
    #[allow(unused_imports, non_upper_case_globals)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            eRender => f.write_str("eRender"),
            eCapture => f.write_str("eConsole"),
            eAll => f.write_str("eAll"),
            _ => f.write_fmt(format_args!("EDataFlow({})", self.0 .0)),
        }
    }
}

pub(crate) struct ERole(pub Audio::ERole);

impl ::core::fmt::Debug for ERole {
    #[allow(unused_imports, non_upper_case_globals)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            eConsole => f.write_str("eConsole"),
            eMultimedia => f.write_str("eMultimedia"),
            eCommunications => f.write_str("eCommunications"),
            _ => f.write_fmt(format_args!("ERole({})", self.0 .0)),
        }
    }
}
