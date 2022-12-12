import os
import pytest

from windows_audio_events import DeviceCollection, AudioDevice


@pytest.fixture(scope="module")
def collection():
    return DeviceCollection()


def test_import(collection: DeviceCollection):
    pass


@pytest.mark.skipif(int(os.environ.get('NUM_AUDIO_DEVICES', '1')) <= 0, reason="No audio devices found")
def test_default_output(collection: DeviceCollection):
    assert isinstance(collection.get_default_output_device(), AudioDevice)
