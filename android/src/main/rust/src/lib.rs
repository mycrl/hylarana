mod discovery;
mod receiver;
mod sender;

use std::{cell::RefCell, ffi::c_void, ptr::null_mut, sync::Arc};

use anyhow::Result;
use common::{logger, runtime::get_runtime_handle};
use jni::{
    JNIEnv, JavaVM,
    objects::{JByteArray, JClass, JObject, JString},
    sys::JNI_VERSION_1_6,
};

use parking_lot::Mutex;

use self::{
    discovery::{DiscoveryService, DiscoveryServiceObserver},
    receiver::Receiver,
    sender::Sender,
};

// Each function is accessible at a fixed offset through the JNIEnv argument.
// The JNIEnv type is a pointer to a structure storing all JNI function
// pointers. It is defined as follows:
//
// typedef const struct JNINativeInterface *JNIEnv;
// The VM initializes the function table, as shown by the following code
// example. Note that the first three entries are reserved for future
// compatibility with COM. In addition, we reserve a number of additional NULL
// entries near the beginning of the function table, so that, for example, a
// future class-related JNI operation can be added after FindClass, rather than
// at the end of the table.
thread_local! {
    pub static ENV: RefCell<Option<*mut jni::sys::JNIEnv>> = const { RefCell::new(None) };
}

static JVM: Mutex<Option<JavaVM>> = Mutex::new(None);

pub(crate) fn get_current_env<'local>() -> JNIEnv<'local> {
    unsafe {
        JNIEnv::from_raw(
            ENV.with(|cell| {
                let mut env = cell.borrow_mut();
                if env.is_none() {
                    let vm = JVM.lock();
                    env.replace(
                        vm.as_ref()
                            .unwrap()
                            .attach_current_thread_as_daemon()
                            .unwrap()
                            .get_raw(),
                    );
                }

                *env
            })
            .unwrap(),
        )
        .unwrap()
    }
}

/// JNI_OnLoad
///
/// jint JNI_OnLoad(JavaVM *vm, void *reserved);
///
/// The VM calls JNI_OnLoad when the native library is loaded (for example,
/// through System.loadLibrary). JNI_OnLoad must return the JNI version
/// needed by the native library.
/// In order to use any of the new JNI functions, a native library must
/// export a JNI_OnLoad function that returns JNI_VERSION_1_2. If the
/// native library does not export a JNI_OnLoad function, the VM assumes
/// that the library only requires JNI version JNI_VERSION_1_1. If the
/// VM does not recognize the version number returned by JNI_OnLoad, the
/// VM will unload the library and act as if the library was +never
/// loaded.
///
/// JNI_Onload_L(JavaVM *vm, void *reserved);
///
/// If a library L is statically linked, then upon the first invocation of
/// System.loadLibrary("L") or equivalent API, a JNI_OnLoad_L function will
/// be invoked with the same arguments and expected return value as
/// specified for the JNI_OnLoad function. JNI_OnLoad_L must return the
/// JNI version needed by the native library. This version must be
/// JNI_VERSION_1_8 or later. If the VM does not recognize the version
/// number returned by JNI_OnLoad_L, the VM will act as if the library
/// was never loaded.
///
/// LINKAGE:
/// Exported from native libraries that contain native method
/// implementation.
#[unsafe(export_name = "JNI_OnLoad")]
extern "system" fn load(vm: JavaVM, _: *mut c_void) -> i32 {
    logger::android::init_logger("com.github.mycrl.hylarana", log::LevelFilter::Info);
    logger::enable_panic_logger();

    transport::startup();
    JVM.lock().replace(vm);

    JNI_VERSION_1_6
}

/// JNI_OnUnload
///
/// void JNI_OnUnload(JavaVM *vm, void *reserved);
///
/// The VM calls JNI_OnUnload when the class loader containing the native
/// library is garbage collected. This function can be used to perform
/// cleanup operations. Because this function is called in an unknown
/// context (such as from a finalizer), the programmer should be
/// conservative on using Java VM services, and refrain from arbitrary
/// Java call-backs. Note that JNI_OnLoad and JNI_OnUnload are two
/// functions optionally supplied by JNI libraries, not exported from
/// the VM.
///
/// JNI_OnUnload_L(JavaVM *vm, void *reserved);
///
/// When the class loader containing a statically linked native library L is
/// garbage collected, the VM will invoke the JNI_OnUnload_L function of the
/// library if such a function is exported. This function can be used to
/// perform cleanup operations. Because this function is called in an
/// unknown context (such as from a finalizer), the programmer should be
/// conservative on using Java VM services, and refrain from arbitrary
/// Java call-backs.
///
/// Informational Note:
/// The act of loading a native library is the complete process of making
/// the library and its native entry points known and registered to the
/// Java VM and runtime. Note that simply performing operating system
/// level operations to load a native library, such as dlopen on a
/// UNIX(R) system, does not fully accomplish this goal. A native
/// function is normally called from the Java class loader to perform a
/// call to the host operating system that will load the library into
/// memory and return a handle to the native library. This handle will
/// be stored and used in subsequent searches for native library
/// entry points. The Java native class loader will complete the load
/// process once the handle is successfully returned to register the
/// library.
///
/// LINKAGE:
/// Exported from native libraries that contain native method
/// implementation.
#[unsafe(export_name = "JNI_OnUnload")]
extern "system" fn unload(_: JavaVM, _: *mut c_void) {
    transport::shutdown();
}

fn ok_or_check<'a, F, T>(env: &mut JNIEnv<'a>, func: F) -> Option<T>
where
    F: FnOnce(&mut JNIEnv<'a>) -> Result<T>,
{
    match func(env) {
        Ok(ret) => Some(ret),
        Err(e) => {
            log::error!("{:?}", e);
            None
        }
    }
}

/// Creates the sender, the return value indicates whether the creation was
/// successful or not.
#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Hylarana_senderCreate")]
extern "system" fn sender_create(
    mut env: JNIEnv,
    _this: JClass,
    bind: JString,
    options: JString,
) -> *const Sender {
    ok_or_check(&mut env, |env| {
        Ok(Box::into_raw(Box::new(Sender::new(env, &bind, &options)?)))
    })
    .unwrap_or_else(|| null_mut())
}

/// get transport sender pkt lose rate.
#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Hylarana_senderGetPktLoseRate")]
extern "system" fn sender_get_pkt_lose_rate(
    _env: JNIEnv,
    _this: JClass,
    sender: *const Sender,
) -> f64 {
    assert!(!sender.is_null());

    unsafe { &*sender }.get_pkt_lose_rate()
}

/// get transport sender port.
#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Hylarana_senderGetPort")]
extern "system" fn sender_get_port(_env: JNIEnv, _this: JClass, sender: *const Sender) -> i32 {
    assert!(!sender.is_null());

    unsafe { &*sender }.get_port() as i32
}

/// Sends the packet to the sender instance.
#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Hylarana_senderWrite")]
extern "system" fn sender_write(
    mut env: JNIEnv,
    _this: JClass,
    sender: *const Sender,
    ty: i32,
    flags: i32,
    timestamp: i64,
    buf: JByteArray,
) -> bool {
    assert!(!sender.is_null());

    ok_or_check(&mut env, |mut env| {
        unsafe { &*sender }.sink(&mut env, ty, flags, timestamp, buf)
    })
    .unwrap_or(false)
}

/// release transport sender.
#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Hylarana_senderRelease")]
extern "system" fn sender_release(_env: JNIEnv, _this: JClass, sender: *mut Sender) {
    assert!(!sender.is_null());

    drop(unsafe { Box::from_raw(sender) });
}

/// Creates the receiver, the return value indicates whether the creation was
/// successful or not.
#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Hylarana_receiverCreate")]
extern "system" fn receiver_create(
    mut env: JNIEnv,
    _this: JClass,
    addr: JString,
    options: JString,
    observer: JObject,
) -> *const Arc<Receiver> {
    ok_or_check(&mut env, |env| {
        let receiver = Arc::new(Receiver::new(env, &addr, &options, &observer)?);

        Ok(Box::into_raw(Box::new(receiver)))
    })
    .unwrap_or_else(|| null_mut())
}

/// release transport receiver.
#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Hylarana_receiverRelease")]
extern "system" fn receiver_release(_env: JNIEnv, _this: JClass, receiver: *mut Arc<Receiver>) {
    assert!(!receiver.is_null());

    drop(unsafe { Box::from_raw(receiver) });
}

/// Register the service, the service type is fixed, you can customize the
/// port number, id is the identifying information of the service, used to
/// distinguish between different publishers, in properties you can add
/// customized data to the published service.
#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Discovery_discoveryCreate")]
extern "system" fn discovery_create(
    mut env: JNIEnv,
    _this: JClass,
    bind: JString,
    observer: JObject,
) -> *const DiscoveryService {
    ok_or_check(&mut env, |env| {
        let bind: String = env.get_string(&bind)?.into();
        let observer = DiscoveryServiceObserver(env.new_global_ref(observer)?);

        Ok(Box::into_raw(Box::new(get_runtime_handle().block_on(
            DiscoveryService::new(bind.parse()?, observer),
        )?)))
    })
    .unwrap_or_else(|| null_mut())
}

#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Discovery_discoverySetMetadata")]
extern "system" fn discovery_set_metadata(
    mut env: JNIEnv,
    _this: JClass,
    discovery: *mut DiscoveryService,
    message: JByteArray,
) -> bool {
    assert!(!discovery.is_null());

    ok_or_check(&mut env, |env| {
        let mut bytes = vec![0; env.get_array_length(&message)? as usize];
        {
            env.get_byte_array_region(&message, 0, unsafe {
                std::mem::transmute::<&mut [u8], &mut [i8]>(&mut bytes[..])
            })?;
        }

        get_runtime_handle().block_on(unsafe { &*discovery }.set_metadata(bytes));
        Ok(true)
    })
    .unwrap_or(false)
}

/// release the discovery service
#[unsafe(export_name = "Java_com_github_mycrl_hylarana_Discovery_discoveryRelease")]
extern "system" fn discovery_release(
    _env: JNIEnv,
    _this: JClass,
    discovery: *mut DiscoveryService,
) {
    assert!(!discovery.is_null());

    drop(unsafe { Box::from_raw(discovery) });
}
