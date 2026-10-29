//! Narrow C ABI used by the Android JNI bridge. Java never receives a private key.
use crate::{
    launcher::{self, Home},
    threads,
};
use std::{
    ffi::{c_char, CStr, CString},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
static STOP: AtomicBool = AtomicBool::new(false);
static STATUS: Mutex<String> = Mutex::new(String::new());
fn reply(result: Result<serde_json::Value, String>) -> *mut c_char {
    let value = result.unwrap_or_else(|e| serde_json::json!({"error":e}));
    CString::new(value.to_string())
        .expect("JSON contains no NUL")
        .into_raw()
}
unsafe fn text<'a>(s: *const c_char) -> Result<&'a str, String> {
    if s.is_null() {
        return Err("missing native argument".into());
    }
    unsafe { CStr::from_ptr(s) }
        .to_str()
        .map_err(|_| "native argument must be UTF-8".into())
}
#[no_mangle]
pub unsafe extern "C" fn pocol_mobile_identity(home: *const c_char) -> *mut c_char {
    reply(std::panic::catch_unwind(|| {
        let home = Home::open(Path::new(unsafe { text(home)? }))?;
        Ok(serde_json::json!({"address":home.address()?,"compatibilitySha256":crate::identity::COMPATIBILITY_HEX}))
    }).unwrap_or_else(|_| Err("identity failed".into())))
}
#[no_mangle]
pub unsafe extern "C" fn pocol_mobile_import(
    home: *const c_char,
    path: *const c_char,
) -> *mut c_char {
    reply(
        std::panic::catch_unwind(|| {
            let bytes = launcher::read_bounded(Path::new(unsafe { text(path)? }))?;
            Ok(Home::open(Path::new(unsafe { text(home)? }))?
                .import(&bytes)?
                .describe())
        })
        .unwrap_or_else(|_| Err("network import failed".into())),
    )
}
#[no_mangle]
pub unsafe extern "C" fn pocol_mobile_run(home: *const c_char) -> *mut c_char {
    let result = std::panic::catch_unwind(|| {
        let path = unsafe { text(home)? }.to_owned();
        let handle = threads::spawn("consensus", threads::CONSENSUS_STACK, move || {
            let home = Home::open(Path::new(&path))?;
            let mut next = Instant::now();
            home.run(None, None, |node| {
                if Instant::now() >= next {
                    if let Ok(mut status) = STATUS.lock() {
                        *status = node.status_json().to_string();
                    }
                    next = Instant::now() + Duration::from_secs(2);
                }
                STOP.load(Ordering::Relaxed)
            })?;
            Ok(serde_json::json!({"state":"stopped"}))
        })
        .map_err(|e| e.to_string())?;
        handle
            .join()
            .map_err(|_| "node thread failed".to_string())?
    })
    .unwrap_or_else(|_| Err("native node failed".into()));
    reply(result)
}
#[no_mangle]
pub extern "C" fn pocol_mobile_stop() {
    STOP.store(true, Ordering::Relaxed);
}
#[no_mangle]
pub extern "C" fn pocol_mobile_status() -> *mut c_char {
    let text = STATUS
        .lock()
        .ok()
        .map(|s| s.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "{\"state\":\"starting\"}".into());
    CString::new(text).expect("JSON contains no NUL").into_raw()
}
#[no_mangle]
pub unsafe extern "C" fn pocol_mobile_free(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}
