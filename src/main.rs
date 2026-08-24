use std::env;
use std::ffi::{CStr, CString, c_char, c_void};
use std::fs::OpenOptions;
use std::io::Write;
use std::net::UdpSocket;
use std::process::Command;
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFArrayRef = *const c_void;
type CFMutableDictionaryRef = *mut c_void;
type CFIndex = isize;
type CFTypeId = usize;
type AXUIElementRef = *const c_void;
type AXError = i32;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGSize {
    width: f64,
    height: f64,
}

const UTF8: u32 = 0x0800_0100;
const AX_SUCCESS: AXError = 0;
const COMMAND_SHIFT: u64 = 0x0010_0000 | 0x0002_0000;
const KEY_A: u16 = 0;
const KEY_S: u16 = 1;
const KEY_V: u16 = 9;
const KEY_RETURN: u16 = 36;
const KEY_ESCAPE: u16 = 53;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
    fn AXUIElementGetTypeID() -> CFTypeId;
    fn AXValueGetValue(value: CFTypeRef, value_type: i32, output: *mut c_void) -> bool;

    fn CGEventCreateKeyboardEvent(
        source: *const c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> CFTypeRef;
    fn CGEventCreateMouseEvent(
        source: *const c_void,
        mouse_type: u32,
        mouse_cursor_position: CGPoint,
        mouse_button: u32,
    ) -> CFTypeRef;
    fn CGEventSetFlags(event: CFTypeRef, flags: u64);
    fn CGEventPost(tap: u32, event: CFTypeRef);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        c_str: *const c_char,
        encoding: u32,
    ) -> CFStringRef;
    fn CFStringGetCString(
        string: CFStringRef,
        buffer: *mut c_char,
        buffer_size: CFIndex,
        encoding: u32,
    ) -> bool;
    fn CFStringGetTypeID() -> CFTypeId;
    fn CFArrayGetTypeID() -> CFTypeId;
    fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, index: CFIndex) -> CFTypeRef;
    fn CFGetTypeID(value: CFTypeRef) -> CFTypeId;
    fn CFRetain(value: CFTypeRef) -> CFTypeRef;
    fn CFRelease(value: CFTypeRef);
    fn CFDictionaryCreateMutable(
        allocator: *const c_void,
        capacity: CFIndex,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> CFMutableDictionaryRef;
    fn CFDictionarySetValue(
        dictionary: CFMutableDictionaryRef,
        key: *const c_void,
        value: *const c_void,
    );
    static kCFBooleanTrue: CFTypeRef;
}

struct CfOwned(CFTypeRef);

impl Drop for CfOwned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) };
        }
    }
}

#[derive(Clone, Copy)]
struct AxElement(AXUIElementRef);

fn log(message: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/zoomosc-lite.log")
    {
        let _ = writeln!(file, "{message}");
    }
}

impl AxElement {
    fn attribute(self, name: &str) -> Option<CfOwned> {
        let attribute = cf_string(name)?;
        let mut value: CFTypeRef = ptr::null();
        let result = unsafe {
            AXUIElementCopyAttributeValue(self.0, attribute.0, &mut value as *mut CFTypeRef)
        };
        if result == AX_SUCCESS && !value.is_null() {
            Some(CfOwned(value))
        } else {
            None
        }
    }

    fn string_attribute(self, name: &str) -> Option<String> {
        let value = self.attribute(name)?;
        cf_to_string(value.0)
    }

    fn children(self) -> Vec<CfOwned> {
        let Some(value) = self.attribute("AXChildren") else {
            return Vec::new();
        };
        unsafe {
            if CFGetTypeID(value.0) != CFArrayGetTypeID() {
                return Vec::new();
            }
            let count = CFArrayGetCount(value.0 as CFArrayRef).min(500);
            (0..count)
                .filter_map(|index| {
                    let item = CFArrayGetValueAtIndex(value.0 as CFArrayRef, index);
                    if !item.is_null() && CFGetTypeID(item) == AXUIElementGetTypeID() {
                        Some(CfOwned(CFRetain(item)))
                    } else {
                        None
                    }
                })
                .collect()
        }
    }

    fn press(self) -> Result<(), String> {
        let press = cf_string("AXPress").ok_or("não foi possível criar AXPress")?;
        let press_result = unsafe { AXUIElementPerformAction(self.0, press.0) };
        if press_result == AX_SUCCESS {
            return Ok(());
        }
        let confirm = cf_string("AXConfirm").ok_or("não foi possível criar AXConfirm")?;
        let confirm_result = unsafe { AXUIElementPerformAction(self.0, confirm.0) };
        if confirm_result == AX_SUCCESS {
            return Ok(());
        }
        Err(format!(
            "AXPress falhou com código {press_result}; AXConfirm falhou com código {confirm_result}"
        ))
    }

    fn searchable_text(self) -> String {
        ["AXTitle", "AXDescription", "AXValue", "AXHelp"]
            .iter()
            .filter_map(|name| self.string_attribute(name))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn click_center(self) -> Result<(), String> {
        let position_value = self
            .attribute("AXPosition")
            .ok_or("elemento sem AXPosition")?;
        let size_value = self.attribute("AXSize").ok_or("elemento sem AXSize")?;
        let mut position = CGPoint::default();
        let mut size = CGSize::default();
        let position_ok = unsafe {
            AXValueGetValue(
                position_value.0,
                1,
                &mut position as *mut CGPoint as *mut c_void,
            )
        };
        let size_ok =
            unsafe { AXValueGetValue(size_value.0, 2, &mut size as *mut CGSize as *mut c_void) };
        if !position_ok || !size_ok || size.width <= 0.0 || size.height <= 0.0 {
            return Err("posição/tamanho AX inválidos".to_owned());
        }
        let center = CGPoint {
            x: position.x + size.width / 2.0,
            y: position.y + size.height / 2.0,
        };
        unsafe {
            let down = CGEventCreateMouseEvent(ptr::null(), 1, center, 0);
            let up = CGEventCreateMouseEvent(ptr::null(), 2, center, 0);
            if down.is_null() || up.is_null() {
                if !down.is_null() {
                    CFRelease(down);
                }
                if !up.is_null() {
                    CFRelease(up);
                }
                return Err("não foi possível criar clique do rato".to_owned());
            }
            CGEventPost(0, down);
            thread::sleep(Duration::from_millis(35));
            CGEventPost(0, up);
            CFRelease(down);
            CFRelease(up);
        }
        Ok(())
    }
}

fn cf_string(value: &str) -> Option<CfOwned> {
    let value = CString::new(value).ok()?;
    let string = unsafe { CFStringCreateWithCString(ptr::null(), value.as_ptr(), UTF8) };
    (!string.is_null()).then_some(CfOwned(string))
}

fn cf_to_string(value: CFTypeRef) -> Option<String> {
    unsafe {
        if value.is_null() || CFGetTypeID(value) != CFStringGetTypeID() {
            return None;
        }
        let mut buffer = vec![0_i8; 4096];
        if !CFStringGetCString(
            value as CFStringRef,
            buffer.as_mut_ptr(),
            buffer.len() as CFIndex,
            UTF8,
        ) {
            return None;
        }
        Some(
            CStr::from_ptr(buffer.as_ptr())
                .to_string_lossy()
                .into_owned(),
        )
    }
}

fn zoom_pid() -> Result<i32, String> {
    let output = Command::new("/usr/bin/pgrep")
        .args(["-x", "zoom.us"])
        .output()
        .map_err(|error| format!("não foi possível procurar o Zoom: {error}"))?;
    let value = String::from_utf8_lossy(&output.stdout);
    value
        .lines()
        .next()
        .and_then(|line| line.trim().parse().ok())
        .ok_or_else(|| "cliente Zoom não encontrado".to_owned())
}

fn zoom_application() -> Result<CfOwned, String> {
    let pid = zoom_pid()?;
    let element = unsafe { AXUIElementCreateApplication(pid) };
    if element.is_null() {
        Err("não foi possível ligar à interface do Zoom".to_owned())
    } else {
        Ok(CfOwned(element))
    }
}

fn focused_window(application: AxElement) -> Option<CfOwned> {
    let value = application.attribute("AXFocusedWindow")?;
    unsafe {
        if CFGetTypeID(value.0) == AXUIElementGetTypeID() {
            Some(value)
        } else {
            None
        }
    }
}

fn contains_alias(text: &str, aliases: &[&str]) -> bool {
    let normalized = text.to_lowercase();
    aliases
        .iter()
        .any(|alias| normalized.contains(&alias.to_lowercase()))
}

fn find_and_press(root: AxElement, aliases: &[&str]) -> bool {
    let retained_root = unsafe { CFRetain(root.0) };
    let mut stack = vec![(CfOwned(retained_root), 0_usize)];
    let mut visited = 0_usize;
    while let Some((owned, depth)) = stack.pop() {
        let element = AxElement(owned.0 as AXUIElementRef);
        visited += 1;
        if visited > 10_000 {
            break;
        }
        if contains_alias(&element.searchable_text(), aliases) && element.press().is_ok() {
            return true;
        }
        if depth < 30 {
            stack.extend(
                element
                    .children()
                    .into_iter()
                    .rev()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    false
}

fn find_share_button_and_press(root: AxElement) -> bool {
    let retained_root = unsafe { CFRetain(root.0) };
    let mut stack = vec![(CfOwned(retained_root), 0_usize)];
    let mut visited = 0_usize;
    while let Some((owned, depth)) = stack.pop() {
        let element = AxElement(owned.0 as AXUIElementRef);
        visited += 1;
        if visited > 10_000 {
            break;
        }
        let role = element.string_attribute("AXRole").unwrap_or_default();
        let title = element
            .string_attribute("AXTitle")
            .unwrap_or_default()
            .to_lowercase();
        let is_share_action = title == "share"
            || title == "partilhar"
            || title == "compartilhar"
            || title.starts_with("share -")
            || title.starts_with("partilhar -")
            || title.starts_with("compartilhar -");
        if role == "AXButton" && is_share_action && element.press().is_ok() {
            return true;
        }
        if depth < 30 {
            stack.extend(
                element
                    .children()
                    .into_iter()
                    .rev()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    false
}

fn tree_contains_title(root: AxElement, aliases: &[&str]) -> bool {
    let retained_root = unsafe { CFRetain(root.0) };
    let mut stack = vec![(CfOwned(retained_root), 0_usize)];
    let mut visited = 0_usize;
    while let Some((owned, depth)) = stack.pop() {
        let element = AxElement(owned.0 as AXUIElementRef);
        visited += 1;
        if visited > 10_000 {
            break;
        }
        let title = element
            .string_attribute("AXTitle")
            .unwrap_or_default()
            .to_lowercase();
        if aliases.iter().any(|alias| title == alias.to_lowercase()) {
            return true;
        }
        if depth < 30 {
            stack.extend(
                element
                    .children()
                    .into_iter()
                    .rev()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    false
}

fn find_exact_title_and_press(root: AxElement, aliases: &[&str]) -> bool {
    let retained_root = unsafe { CFRetain(root.0) };
    let mut stack = vec![(CfOwned(retained_root), 0_usize)];
    let mut visited = 0_usize;
    while let Some((owned, depth)) = stack.pop() {
        let element = AxElement(owned.0 as AXUIElementRef);
        visited += 1;
        if visited > 10_000 {
            break;
        }
        let title = element
            .string_attribute("AXTitle")
            .unwrap_or_default()
            .to_lowercase();
        if aliases.iter().any(|alias| title == alias.to_lowercase()) && element.press().is_ok() {
            return true;
        }
        if depth < 30 {
            stack.extend(
                element
                    .children()
                    .into_iter()
                    .rev()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    false
}

fn wait_exact_title_and_press(root: AxElement, aliases: &[&str], timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if find_exact_title_and_press(root, aliases) {
            return true;
        }
        thread::sleep(Duration::from_millis(150));
    }
    false
}

fn wait_and_press(root: AxElement, aliases: &[&str], timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let search_root = focused_window(root)
            .map(|window| {
                let retained = unsafe { CFRetain(window.0) };
                AxElement(retained as AXUIElementRef)
            })
            .unwrap_or(root);
        if find_and_press(search_root, aliases) {
            if search_root.0 != root.0 {
                unsafe { CFRelease(search_root.0) };
            }
            return true;
        }
        if search_root.0 != root.0 {
            unsafe { CFRelease(search_root.0) };
        }
        // O seletor de partilha do Zoom pode ser uma sheet ou uma janela
        // secundária sem se tornar imediatamente AXFocusedWindow.
        if find_and_press(root, aliases) {
            return true;
        }
        thread::sleep(Duration::from_millis(150));
    }
    false
}

fn click_profile_in_menu(root: AxElement, aliases: &[&str], timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        for window in root.children() {
            let window_element = AxElement(window.0 as AXUIElementRef);
            if window_element.string_attribute("AXTitle").as_deref() != Some("Menu window") {
                continue;
            }
            let mut stack = window_element.children();
            while let Some(owned) = stack.pop() {
                let element = AxElement(owned.0 as AXUIElementRef);
                if contains_alias(&element.searchable_text(), aliases)
                    && element.click_center().is_ok()
                {
                    return true;
                }
                stack.extend(element.children());
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

fn meeting_shows_profile(root: AxElement, aliases: &[&str], timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        for window in root.children() {
            let window_element = AxElement(window.0 as AXUIElementRef);
            let title = window_element
                .string_attribute("AXTitle")
                .unwrap_or_default();
            if title != "Reunião Zoom" && title != "Zoom Meeting" {
                continue;
            }
            let mut stack = window_element.children();
            let mut visited = 0;
            while let Some(owned) = stack.pop() {
                let element = AxElement(owned.0 as AXUIElementRef);
                visited += 1;
                if contains_alias(&element.searchable_text(), aliases) {
                    return true;
                }
                if visited < 2_000 {
                    stack.extend(element.children());
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

fn log_accessibility_tree(root: AxElement) {
    log("--- árvore de Acessibilidade do Zoom ---");
    let retained_root = unsafe { CFRetain(root.0) };
    let mut stack = vec![(CfOwned(retained_root), 0_usize)];
    let mut count = 0;
    while let Some((owned, depth)) = stack.pop() {
        let element = AxElement(owned.0 as AXUIElementRef);
        count += 1;
        if count > 3_000 {
            log("--- limite de 3000 elementos ---");
            break;
        }
        let role = element.string_attribute("AXRole").unwrap_or_default();
        let text = element.searchable_text();
        if !text.is_empty() {
            log(&format!("{}{}: {}", "  ".repeat(depth), role, text));
        }
        if depth < 25 {
            stack.extend(
                element
                    .children()
                    .into_iter()
                    .rev()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    log("--- fim da árvore ---");
}

fn activate_zoom() -> Result<(), String> {
    let status = Command::new("/usr/bin/open")
        .args(["-a", "zoom.us"])
        .status()
        .map_err(|error| format!("não foi possível ativar o Zoom: {error}"))?;
    if status.success() {
        thread::sleep(Duration::from_millis(350));
        Ok(())
    } else {
        Err("o macOS recusou ativar o Zoom".to_owned())
    }
}

fn send_command_shift_s() -> Result<(), String> {
    send_key(KEY_S, COMMAND_SHIFT)
}

fn send_return() -> Result<(), String> {
    send_key(KEY_RETURN, 0)
}

fn send_escape() -> Result<(), String> {
    send_key(KEY_ESCAPE, 0)
}

fn close_audio_menu_if_open(root: AxElement) {
    if tree_contains_title(root, &["Menu window"]) && send_escape().is_ok() {
        log("audio-profile: menu do microfone fechado com Escape");
    }
}

fn send_command_shift_a() -> Result<(), String> {
    send_key(KEY_A, COMMAND_SHIFT)
}

fn send_command_shift_v() -> Result<(), String> {
    send_key(KEY_V, COMMAND_SHIFT)
}

fn send_key(key_code: u16, flags: u64) -> Result<(), String> {
    unsafe {
        let down = CGEventCreateKeyboardEvent(ptr::null(), key_code, true);
        let up = CGEventCreateKeyboardEvent(ptr::null(), key_code, false);
        if down.is_null() || up.is_null() {
            if !down.is_null() {
                CFRelease(down);
            }
            if !up.is_null() {
                CFRelease(up);
            }
            return Err("não foi possível criar o atalho de teclado".to_owned());
        }
        CGEventSetFlags(down, flags);
        CGEventSetFlags(up, flags);
        CGEventPost(0, down);
        thread::sleep(Duration::from_millis(40));
        CGEventPost(0, up);
        CFRelease(down);
        CFRelease(up);
    }
    Ok(())
}

fn require_accessibility() -> Result<(), String> {
    if unsafe { AXIsProcessTrusted() } {
        Ok(())
    } else {
        Err(
            "sem permissão de Acessibilidade; adiciona ZoomOSC Lite em Definições do Sistema > Privacidade e Segurança > Acessibilidade"
                .to_owned(),
        )
    }
}

fn prompt_for_accessibility_if_needed() {
    if !unsafe { AXIsProcessTrusted() } {
        eprintln!(
            "Autoriza ZoomOSC Lite em Definições do Sistema > Privacidade e Segurança > Acessibilidade"
        );
        unsafe {
            let options = CFDictionaryCreateMutable(ptr::null(), 0, ptr::null(), ptr::null());
            if !options.is_null() {
                CFDictionarySetValue(options, kAXTrustedCheckOptionPrompt, kCFBooleanTrue);
                AXIsProcessTrustedWithOptions(options);
                CFRelease(options);
            }
        }
    }
}

fn start_camera_share() -> Result<(), String> {
    log("share-camera: início");
    require_accessibility()?;
    log("share-camera: Acessibilidade autorizada");
    activate_zoom()?;
    log("share-camera: Zoom ativado");
    let application = zoom_application()?;
    let root = AxElement(application.0 as AXUIElementRef);

    let camera_aliases = [
        "content from 2nd camera",
        "content from second camera",
        "conteúdo da segunda câmara",
        "conteudo da segunda camara",
        "conteúdo da 2.ª câmara",
        "segunda câmera",
        "second camera",
    ];

    // Se o seletor já estiver aberto na secção avançada, não volta a enviar
    // Cmd+Shift+S, pois isso pode fechar ou alterar o diálogo.
    let mut camera_pressed = find_and_press(root, &camera_aliases);
    if !camera_pressed {
        send_command_shift_s()?;
        log("share-camera: atalho Cmd+Shift+S enviado");
        camera_pressed = wait_and_press(root, &camera_aliases, Duration::from_secs(2));
    }
    if !camera_pressed {
        // Nesta versão do Zoom o seletor usa um único botão "Mudar" que
        // percorre Telas (1/3), Documentos (2/3) e Mais (3/3).
        for section in 1..=3 {
            if find_and_press(root, &["mudar", "switch"]) {
                log(&format!("share-camera: secção alterada ({section}/3)"));
                thread::sleep(Duration::from_millis(350));
                camera_pressed = find_and_press(root, &camera_aliases);
                if camera_pressed {
                    break;
                }
            } else {
                break;
            }
        }
    }
    if !camera_pressed {
        log("share-camera: falhou localizar/pressionar segunda câmara");
        log_accessibility_tree(root);
        return Err("a opção Conteúdo da segunda câmara não foi encontrada".to_owned());
    }
    log("share-camera: segunda câmara selecionada");

    thread::sleep(Duration::from_millis(250));
    let window = focused_window(root).ok_or("a janela de partilha deixou de estar acessível")?;
    let mut share_pressed = find_share_button_and_press(AxElement(window.0 as AXUIElementRef));
    if !share_pressed {
        share_pressed = find_share_button_and_press(root);
    }
    if !share_pressed {
        log("share-camera: ação AX recusada; a tentar Return no botão principal");
        send_return()?;
        thread::sleep(Duration::from_millis(500));
        log("share-camera: Return enviado ao diálogo");
    } else {
        log("share-camera: Partilhar pressionado com sucesso");
    }
    Ok(())
}

fn stop_share() -> Result<(), String> {
    require_accessibility()?;
    activate_zoom()?;
    send_command_shift_s()
}

fn set_audio_muted(muted: bool) -> Result<(), String> {
    require_accessibility()?;
    activate_zoom()?;
    let application = zoom_application()?;
    let root = AxElement(application.0 as AXUIElementRef);
    let currently_muted = tree_contains_title(
        root,
        &[
            "ativar áudio",
            "ativar audio",
            "ativar som",
            "unmute audio",
            "unmute my audio",
        ],
    );
    let currently_unmuted = tree_contains_title(
        root,
        &[
            "desativar áudio",
            "desativar audio",
            "desativar som",
            "mute audio",
            "mute my audio",
        ],
    );
    if !currently_muted && !currently_unmuted {
        return Err("não foi possível determinar o estado atual do microfone".to_owned());
    }
    if muted != currently_muted {
        send_command_shift_a()?;
        log(if muted {
            "audio: mute enviado"
        } else {
            "audio: unmute enviado"
        });
    } else {
        log("audio: estado pretendido já estava ativo");
    }
    Ok(())
}

fn set_video_enabled(enabled: bool) -> Result<(), String> {
    require_accessibility()?;
    activate_zoom()?;
    let application = zoom_application()?;
    let root = AxElement(application.0 as AXUIElementRef);
    let currently_off = tree_contains_title(
        root,
        &[
            "iniciar vídeo",
            "iniciar video",
            "start video",
            "start my video",
        ],
    );
    let currently_on = tree_contains_title(
        root,
        &[
            "interromper vídeo",
            "interromper video",
            "parar vídeo",
            "parar video",
            "stop video",
            "stop my video",
        ],
    );
    if !currently_off && !currently_on {
        return Err("não foi possível determinar o estado atual do vídeo".to_owned());
    }
    if enabled != currently_on {
        send_command_shift_v()?;
        log(if enabled {
            "video: ligar enviado"
        } else {
            "video: desligar enviado"
        });
    } else {
        log("video: estado pretendido já estava ativo");
    }
    Ok(())
}

fn set_live_performance_profile(root: AxElement) -> Result<(), String> {
    if !find_exact_title_and_press(root, &["Configurações...", "Settings..."]) {
        log("audio-profile: não encontrou Configurações; a tentar o botão principal");
        if !find_exact_title_and_press(root, &["Configurações", "Settings"]) {
            log_accessibility_tree(root);
            return Err("não foi possível abrir as Configurações do Zoom".to_owned());
        }
    }
    thread::sleep(Duration::from_millis(500));

    if !wait_exact_title_and_press(root, &["Áudio", "Audio"], Duration::from_secs(5)) {
        log("audio-profile: não encontrou a secção Áudio");
        log_accessibility_tree(root);
        return Err("não foi possível abrir a secção Áudio".to_owned());
    }
    thread::sleep(Duration::from_millis(400));

    let aliases = [
        "Áudio de performance ao vivo",
        "Áudio de apresentação ao vivo",
        "Live performance audio",
    ];
    if !wait_exact_title_and_press(root, &aliases, Duration::from_secs(5)) {
        log("audio-profile: live-performance não encontrado");
        log_accessibility_tree(root);
        return Err("perfil de áudio indisponível: live-performance".to_owned());
    }
    log("audio-profile: live-performance selecionado");
    Ok(())
}

fn set_audio_profile(profile: &str) -> Result<(), String> {
    require_accessibility()?;
    activate_zoom()?;
    let application = zoom_application()?;
    let root = AxElement(application.0 as AXUIElementRef);

    // O Zoom só expõe Live Performance na página de definições de Áudio.
    if profile == "live-performance" {
        return set_live_performance_profile(root);
    }

    let profile_aliases: &[&str] = match profile {
        "noise-removal" => &[
            "Remoção de ruído",
            "Remoção de ruído do Zoom",
            "Noise removal",
            "Zoom background noise removal",
        ],
        "isolation" => &[
            "Isolamento de áudio personalizado",
            "Personalized audio isolation",
        ],
        "original" => &[
            "Áudio original para músicos",
            "Som original para músicos",
            "Original sound for musicians",
        ],
        _ => return Err(format!("perfil de áudio desconhecido: {profile}")),
    };

    // Traz a janela da reunião para a frente caso outra janela do Zoom esteja ativa.
    let _ = find_exact_title_and_press(root, &["Reunião Zoom", "Zoom Meeting"]);
    thread::sleep(Duration::from_millis(300));

    let audio_options = [
        "Opções de áudio",
        "Opções do áudio",
        "Mais opções de áudio",
        "Audio options",
        "More audio options",
        "Select audio device",
    ];
    let mut menu_opened = wait_and_press(root, &audio_options, Duration::from_secs(1));
    if !menu_opened {
        // A barra inferior do Zoom desaparece poucos instantes depois de a
        // janela ganhar foco. Quando isso acontece, ativa a opção nativa para
        // manter os controlos visíveis e volta a procurar a seta do áudio.
        let controls_toggled = find_exact_title_and_press(
            root,
            &[
                "Sempre Exibir Controles De Reunião",
                "Mostrar sempre controlos da reunião",
                "Always Show Meeting Controls",
            ],
        );
        if controls_toggled {
            log("audio-profile: controlos da reunião tornados persistentes");
            thread::sleep(Duration::from_millis(400));
            menu_opened = wait_and_press(root, &audio_options, Duration::from_secs(3));
        }
    }
    if !menu_opened {
        log("audio-profile: botão de expansão do áudio não encontrado");
        log_accessibility_tree(root);
        return Err("não foi possível abrir o menu expandido do microfone".to_owned());
    }
    log("audio-profile: menu expandido do microfone aberto");

    if !click_profile_in_menu(root, profile_aliases, Duration::from_secs(3)) {
        log(&format!(
            "audio-profile: perfil {profile} não encontrado no menu"
        ));
        log_accessibility_tree(root);
        close_audio_menu_if_open(root);
        return Err(format!("perfil de áudio indisponível no menu: {profile}"));
    }
    log(&format!(
        "audio-profile: {profile} selecionado pelo menu do microfone"
    ));
    thread::sleep(Duration::from_millis(150));
    close_audio_menu_if_open(root);
    if !meeting_shows_profile(root, profile_aliases, Duration::from_secs(2)) {
        log(&format!(
            "audio-profile: seleção de {profile} não confirmada no botão do microfone"
        ));
        return Err(format!("a seleção do perfil não foi confirmada: {profile}"));
    }
    log(&format!("audio-profile: {profile} confirmado"));
    Ok(())
}

fn dump_tree() -> Result<(), String> {
    require_accessibility()?;
    activate_zoom()?;
    let application = zoom_application()?;
    let root = AxElement(application.0 as AXUIElementRef);
    let focused = focused_window(root);
    let root = focused
        .as_ref()
        .map(|window| AxElement(window.0 as AXUIElementRef))
        .unwrap_or(root);
    let retained_root = unsafe { CFRetain(root.0) };
    let mut stack = vec![(CfOwned(retained_root), 0_usize)];
    let mut count = 0;
    while let Some((owned, depth)) = stack.pop() {
        let element = AxElement(owned.0 as AXUIElementRef);
        count += 1;
        if count > 3000 {
            println!("… limite de 3000 elementos atingido");
            break;
        }
        let role = element.string_attribute("AXRole").unwrap_or_default();
        let text = element.searchable_text();
        if !text.is_empty() {
            println!("{}{}: {}", "  ".repeat(depth), role, text);
        }
        if depth < 25 {
            stack.extend(
                element
                    .children()
                    .into_iter()
                    .rev()
                    .map(|child| (child, depth + 1)),
            );
        }
    }
    Ok(())
}

fn osc_address(packet: &[u8]) -> Result<&str, String> {
    if packet.first() != Some(&b'/') {
        return Err("pacote não é uma mensagem OSC".to_owned());
    }
    let end = packet
        .iter()
        .position(|byte| *byte == 0)
        .ok_or("endereço OSC sem terminador")?;
    std::str::from_utf8(&packet[..end]).map_err(|_| "endereço OSC não é UTF-8".to_owned())
}

fn execute(command: &str) -> Result<(), String> {
    match command {
        "/zoom/share/camera/start" | "/zoom/me/startCameraShare" => start_camera_share(),
        "/zoom/share/stop" | "/zoom/me/stopShare" => stop_share(),
        "/zoom/audio/mute" | "/zoom/me/mute" => set_audio_muted(true),
        "/zoom/audio/unmute" | "/zoom/me/unmute" => set_audio_muted(false),
        "/zoom/video/on" | "/zoom/me/startVideo" => set_video_enabled(true),
        "/zoom/video/off" | "/zoom/me/stopVideo" => set_video_enabled(false),
        "/zoom/audio/profile/noise-removal" => set_audio_profile("noise-removal"),
        "/zoom/audio/profile/isolation" => set_audio_profile("isolation"),
        "/zoom/audio/profile/original" => set_audio_profile("original"),
        "/zoom/audio/profile/live-performance" => set_audio_profile("live-performance"),
        _ => Err(format!("comando OSC desconhecido: {command}")),
    }
}

fn serve(bind: &str) -> Result<(), String> {
    let _ = std::fs::write("/tmp/zoomosc-lite.log", "ZoomOSC Lite iniciado\n");
    prompt_for_accessibility_if_needed();
    let socket = UdpSocket::bind(bind)
        .map_err(|error| format!("não foi possível escutar em {bind}: {error}"))?;
    println!("ZoomOSC Lite ativo em osc.udp://{bind}");
    println!(
        "Comandos: /zoom/share/camera/start, /zoom/share/stop, /zoom/audio/mute, /zoom/audio/unmute, /zoom/video/on, /zoom/video/off, /zoom/audio/profile/<noise-removal|isolation|original|live-performance>"
    );
    let mut buffer = [0_u8; 65_535];
    loop {
        let (length, peer) = socket
            .recv_from(&mut buffer)
            .map_err(|error| format!("erro ao receber OSC: {error}"))?;
        match osc_address(&buffer[..length]).and_then(execute) {
            Ok(()) => {
                log(&format!("{peer}: OK"));
                println!("{peer}: OK");
            }
            Err(error) => {
                log(&format!("{peer}: ERRO: {error}"));
                eprintln!("{peer}: {error}");
            }
        }
    }
}

fn print_help() {
    println!(
        "ZoomOSC Lite\n\n\
         Uso:\n\
           zoomosc-lite serve [endereço]       Escuta OSC (predefinição: 127.0.0.1:9000)\n\
           zoomosc-lite share-camera           Partilha a segunda câmara agora\n\
           zoomosc-lite stop-share             Para a partilha\n\
           zoomosc-lite inspect                Mostra a árvore de acessibilidade do Zoom\n"
    );
}

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let result = match arguments.first().map(String::as_str) {
        Some("serve") => serve(
            arguments
                .get(1)
                .map(String::as_str)
                .unwrap_or("127.0.0.1:9000"),
        ),
        Some("share-camera") => start_camera_share(),
        Some("stop-share") => stop_share(),
        Some("inspect") => dump_tree(),
        Some("help" | "--help" | "-h") => {
            print_help();
            Ok(())
        }
        None => serve("0.0.0.0:9000"),
        Some(other) => Err(format!("opção desconhecida: {other}")),
    };
    if let Err(error) = result {
        eprintln!("Erro: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::osc_address;

    #[test]
    fn reads_padded_osc_address() {
        let packet = b"/zoom/video/on\0,\0\0\0";
        assert_eq!(osc_address(packet).unwrap(), "/zoom/video/on");
    }

    #[test]
    fn rejects_non_osc_packet() {
        assert!(osc_address(b"zoom/video/on\0").is_err());
    }
}
