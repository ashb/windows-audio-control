from . import windows_audio_events as _native


class WindowsAudioEvents(_native.WindowsAudioEvents):
    async def __anext__(self):
        # Yes, this is silly. I wasn't able to get pyo3 to let me return an exception (other then
        # StopAsyncIteration) from __anext__ implemented in native code!
        return await self._next_event()
