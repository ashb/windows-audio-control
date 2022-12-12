from __future__ import annotations
import typing

@typing.final
class AudioDevice:
    device_id: str
    events: AudioDeviceEventIterator
    name: str

    def toggle_mute(self, /): ...

@typing.final
class AudioDeviceEventIterator:
    """Async iterator of changes to a device's volume"""

    device: typing.Any

    def __aiter__(self, /): ...
    def __anext__(self, /): ...

@typing.final
class CollectionEventsIterator:
    """Async iterator of changes to devices in a collection"""

    def close(self, /):
        """Close the iterator"""
    def __aiter__(self, /): ...
    def __anext__(self, /): ...

@typing.final
class DataFlow:
    ALL = ...
    CAPTURE = ...
    RENDER = ...

@typing.final
class DeviceCollection:
    events: CollectionEventsIterator

    def get_default_input_device(self, /) -> AudioDevice:
        """Get the current default input device (aka microphone)"""
    def get_default_output_device(self, /) -> AudioDevice:
        """Get the current default output device (aka speakers)"""

@typing.final
class DeviceCollectionEvent:
    dataflow: DataFlow | None
    device_id: str
    kind: DeviceCollectionEventType
    role: Role | None
    state: DeviceState | None

@typing.final
class DeviceCollectionEventType:
    ADDED = ...
    DEFAULT_CHANGED = ...
    REMOVED = ...
    STATE_CHANGED = ...

@typing.final
class DeviceState:
    ACTIVE = ...
    DISABLED = ...
    NOT_PRESENT = ...
    UNPLUGGED = ...

@typing.final
class Role:
    COMMS = ...
    CONSOLE = ...
    MULTIMEDIA = ...

@typing.final
class VolumeChangeEvent:
    channel_volumes: tuple[float, ...]
    device: AudioDevice
    mute: bool
    volume: float
