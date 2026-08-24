#![allow(dead_code)]
#![allow(clippy::items_after_test_module)]

include!("main.rs");

/// Executa um endereço OSC já validado pela camada UDP da aplicação.
///
/// # Safety
///
/// `command` deve apontar para uma string C válida, terminada por NUL, e
/// permanecer legível durante esta chamada.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zoomosc_execute(command: *const std::ffi::c_char) -> i32 {
    if command.is_null() {
        return 2;
    }
    let command = unsafe { std::ffi::CStr::from_ptr(command) };
    let Ok(command) = command.to_str() else {
        return 2;
    };
    match execute(command) {
        Ok(()) => 0,
        Err(error) => {
            log(&format!("GUI: ERRO: {error}"));
            1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn zoomosc_accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

#[unsafe(no_mangle)]
pub extern "C" fn zoomosc_request_accessibility() -> bool {
    if unsafe { AXIsProcessTrusted() } {
        return true;
    }
    unsafe {
        let options = CFDictionaryCreateMutable(ptr::null(), 0, ptr::null(), ptr::null());
        if options.is_null() {
            return false;
        }
        CFDictionarySetValue(options, kAXTrustedCheckOptionPrompt, kCFBooleanTrue);
        let trusted = AXIsProcessTrustedWithOptions(options);
        CFRelease(options);
        trusted
    }
}
