#[cfg(target_os = "windows")]
use common::win32::Direct3DDevice;

#[cfg(target_os = "windows")]
use parking_lot::RwLock;

#[cfg(target_os = "windows")]
static DIRECT_3D_DEVICE: RwLock<Option<Direct3DDevice>> = RwLock::new(None);

// Check if the D3D device has been created. If not, create a global one.
#[cfg(target_os = "windows")]
pub(crate) fn get_direct3d() -> Direct3DDevice {
    if DIRECT_3D_DEVICE.read().is_none() {
        DIRECT_3D_DEVICE
            .write()
            .replace(Direct3DDevice::new().expect("D3D device was not initialized successfully!"));
    }

    DIRECT_3D_DEVICE.read().as_ref().unwrap().clone()
}
