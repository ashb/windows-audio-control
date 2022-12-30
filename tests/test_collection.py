import os
import sys
import pytest

from windows_audio_events import DeviceCollection, AudioDevice, DeviceState, DataFlow


@pytest.fixture(scope="module")
def collection():
    return DeviceCollection()


def test_import(collection: DeviceCollection):
    pass


@pytest.mark.skipif(int(os.environ.get('NUM_AUDIO_DEVICES', '1')) <= 0, reason="No audio devices found")
def test_default_output(collection: DeviceCollection):
    assert isinstance(collection.get_default_output_device(), AudioDevice)


@pytest.mark.parametrize(
    ["state"],
    [[DeviceState.ACTIVE], [DeviceState.ACTIVE | DeviceState.UNPLUGGED], [DeviceState.ALL]],
    ids=lambda state: state.name,
)
def test_device_filter(state, collection: DeviceCollection):
    devices = collection.devices(DataFlow.RENDER, state)
    # We can't test anything we get back, just that we have a length
    assert isinstance(len(devices), int)

    with pytest.raises(IndexError):
        devices[sys.maxsize]
