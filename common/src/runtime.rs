use std::sync::LazyLock;

use parking_lot::Mutex;
use tokio::runtime::{Handle, Runtime};

// A runtime created with a delay that automatically creates a multithreaded
// runtime internally if a runtime handle is not provided externally.
static RUNTIME: LazyLock<Mutex<Option<Runtime>>> = LazyLock::new(|| Mutex::new(None));

// Deferred creation of runtime handles, which may be externally provided or
// internally created themselves.
static HANDLE: LazyLock<Mutex<Option<Handle>>> = LazyLock::new(|| Mutex::new(None));

/// Set tokio's runtime handle, which is internally dependent on tokio's
/// asynchronous runtime, although the library itself does not provide an
/// asynchronous interface. To avoid creating multiple runtimes repeatedly, you
/// can provide external runtimes to the library internally.
pub fn set_runtime_handle(handle: Handle) {
    HANDLE.lock().replace(handle);
}

/// Get tokio asynchronous runtime handle.
///
/// Internally, a multithreaded runtime is created by default internally if no
/// runtime is provided externally.
pub fn get_runtime_handle() -> Handle {
    if let Ok(handle) = Handle::try_current() {
        return handle;
    }

    if let Some(handle) = HANDLE.lock().as_ref() {
        return handle.clone();
    }

    let runtime =
        Runtime::new().expect("failed to create tokio multithreaded runtime, this is a bug");

    let handle = runtime.handle().clone();

    {
        HANDLE.lock().replace(handle.clone());
        RUNTIME.lock().replace(runtime);
    }

    handle
}
