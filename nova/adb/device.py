"""Device information collection."""

import typing as t

from nova.adb.client import ADBClient


def get_device_info(adb_client: ADBClient, device_id: str) -> t.Dict[str, str]:
    """Collect device information using getprop.
    
    Args:
        adb_client: ADB client instance
        device_id: Device identifier
        
    Returns:
        Dictionary of device properties
    """
    properties = [
        ("Device Model", "ro.product.model"),
        ("Brand", "ro.product.brand"),
        ("Manufacturer", "ro.product.manufacturer"),
        ("Device", "ro.product.device"),
        ("Android Version", "ro.build.version.release"),
        ("API Level", "ro.build.version.sdk"),
        ("Build ID", "ro.build.id"),
        ("Build Type", "ro.build.type"),
        ("Serial Number", "ro.serialno"),
        ("Hardware", "ro.hardware"),
        ("CPU ABI", "ro.product.cpu.abi"),
        ("Security Patch", "ro.build.version.security_patch"),
        ("Bootloader", "ro.bootloader"),
        ("Radio Version", "gsm.version.baseband"),
    ]
    
    device_info = {}
    
    for display_name, prop_name in properties:
        try:
            value = adb_client.get_property(device_id, prop_name)
            if value:
                device_info[display_name] = value
            else:
                device_info[display_name] = "Unknown"
        except Exception:
            device_info[display_name] = "Error"
    
    return device_info