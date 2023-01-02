#![allow(non_snake_case)]

use windows::core::{IUnknown, IUnknown_Vtbl};
use windows::Win32::Media::Audio::ERole;

#[allow(non_upper_case_globals)]
pub const PolicyConfig: windows::core::GUID =
    ::windows::core::GUID::from_u128(0x870AF99C_171D_4F9E_AF0D_E63DF40C2BC9);

#[windows::core::interface("F8679F50-850A-41CF-9C72-430F290290C8")]
pub unsafe trait IPolicyConfig: IUnknown {
    // We don't actually care about any of the methods except SetDefaultEndpoint(), so
    // we'll just put dummies in to ensure SetDefaultEndpoint() has the correct index in
    // the vtable.

    fn dummy1(&self) -> ();
    fn dummy2(&self) -> ();
    fn dummy3(&self) -> ();
    fn dummy4(&self) -> ();
    fn dummy5(&self) -> ();
    fn dummy6(&self) -> ();
    fn dummy7(&self) -> ();
    fn dummy8(&self) -> ();
    fn dummy9(&self) -> ();
    fn dummy10(&self) -> ();

    // HRESULT STDMETHODCALLTYPE SetDefaultEndpoint(__in PCWSTR wszDeviceId, __in ERole role);
    pub unsafe fn SetDefaultEndpoint(
        &self,
        wszDeviceId: windows::core::PCWSTR,
        role: ERole,
    ) -> windows::core::HRESULT;

    // HRESULT STDMETHODCALLTYPE SetEndpointVisibility(PCWSTR, INT);
    // unsafe fn SetEndpointVisibility(&self) -> windows::core::HRESULT;
}
