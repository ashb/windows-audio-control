import asyncio
import os
import sys
import pytest

from windows_audio_control import (
    DeviceCollection,
    DeviceCollectionEvent,
    DeviceCollectionEventType,
    AudioDevice,
    DeviceState,
    DataFlow,
    Role,
)


@pytest.fixture(scope="module")
def collection():
    return DeviceCollection()


def test_device_not_found(collection: DeviceCollection):
    with pytest.raises(KeyError):
        collection.devices["I am not a valid device ID"]


@pytest.mark.skipif(int(os.environ.get('NUM_AUDIO_DEVICES', '1')) <= 0, reason="No audio devices found")
def test_default_output(collection: DeviceCollection):
    playback = collection.get_default_output_device()
    assert isinstance(playback, AudioDevice)

    assert playback == collection.get_default_output_device()
    # Check it's equal, but not the same object
    assert playback is not collection.get_default_output_device()

    assert collection.devices[playback.device_id] == playback


@pytest.mark.parametrize(
    ["state"],
    [[DeviceState.ACTIVE], [DeviceState.ACTIVE | DeviceState.UNPLUGGED], [DeviceState.ALL]],
    ids=lambda state: state.name,
)
def test_device_filter(state, collection: DeviceCollection):
    devices = collection.filter_devices(DataFlow.RENDER, state)
    # We can't test anything we get back, just that we have a length
    assert isinstance(len(devices), int)

    with pytest.raises(IndexError):
        devices[sys.maxsize]


@pytest.mark.skipif(int(os.environ.get('NUM_AUDIO_DEVICES', '2')) <= 1, reason="Test needs multiple audio devices")
async def test_make_default(collection: DeviceCollection):
    playback_devices = collection.filter_devices(DataFlow.RENDER, DeviceState.ACTIVE)

    async def read_events_until_device_change(expected_dev: AudioDevice):
        async for event in collection.events:
            assert isinstance(event, DeviceCollectionEvent)

            if event.kind != DeviceCollectionEventType.DEFAULT_CHANGED:
                continue
            if event.role != Role.MULTIMEDIA:
                continue
            if expected_dev and event.device_id == expected_dev.device_id:
                return event

    assert len(playback_devices) > 1, "Need more than a single device for this test"

    current = collection.get_default_output_device()

    try:
        for dev in playback_devices:
            assert isinstance(dev, AudioDevice)

            if dev == current:
                continue

            task = asyncio.create_task(read_events_until_device_change(dev))
            # Yield control and start the event watching
            await asyncio.sleep(0)
            dev.set_default(Role.MULTIMEDIA)

            event = await asyncio.wait_for(task, timeout=5)

            assert isinstance(event, DeviceCollectionEvent)
            assert collection.devices[event.device_id] == dev
            break
    finally:
        current.set_default(Role.MULTIMEDIA)
