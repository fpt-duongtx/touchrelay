//! Giao diện thanh menu (NSStatusItem) khi chạy dưới dạng TouchRelay.app.
//! Gọi Objective-C runtime trực tiếp qua FFI, không dùng crate ngoài.
#![allow(non_upper_case_globals)]

use crate::i18n::{is_vi, set_vi, tr, APP_NAME, CONTACT_EMAIL};
use crate::macos::{request_quit, with_app};
use std::ffi::{c_char, c_void, CString};
use std::mem::transmute;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

pub type Id = *mut c_void;
pub type Sel = *const c_void;
const NIL: Id = ptr::null_mut();

#[cfg_attr(target_os = "macos", link(name = "objc"))]
extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_allocateClassPair(superclass: Id, name: *const c_char, extra: usize) -> Id;
    fn objc_registerClassPair(cls: Id);
    fn class_addMethod(cls: Id, sel: Sel, imp: *const c_void, types: *const c_char) -> bool;
    fn objc_msgSend();
}

#[cfg_attr(target_os = "macos", link(name = "AppKit", kind = "framework"))]
#[cfg_attr(target_os = "macos", link(name = "Foundation", kind = "framework"))]
#[cfg_attr(target_os = "macos", link(name = "ServiceManagement", kind = "framework"))]
extern "C" {}

// ---------- helpers gửi message ----------
fn sel(name: &str) -> Sel {
    let c = CString::new(name).unwrap();
    unsafe { sel_registerName(c.as_ptr()) }
}

fn class(name: &str) -> Id {
    let c = CString::new(name).unwrap();
    unsafe { objc_getClass(c.as_ptr()) }
}

fn imp() -> *const c_void {
    objc_msgSend as *const c_void
}

unsafe fn msg(o: Id, s: &str) -> Id {
    let f: extern "C" fn(Id, Sel) -> Id = transmute(imp());
    f(o, sel(s))
}
unsafe fn msg_id(o: Id, s: &str, a: Id) -> Id {
    let f: extern "C" fn(Id, Sel, Id) -> Id = transmute(imp());
    f(o, sel(s), a)
}
unsafe fn msg_id2(o: Id, s: &str, a: Id, b: Id) -> Id {
    let f: extern "C" fn(Id, Sel, Id, Id) -> Id = transmute(imp());
    f(o, sel(s), a, b)
}
unsafe fn msg_int(o: Id, s: &str, a: isize) -> Id {
    let f: extern "C" fn(Id, Sel, isize) -> Id = transmute(imp());
    f(o, sel(s), a)
}
unsafe fn msg_bool(o: Id, s: &str, a: bool) -> Id {
    let f: extern "C" fn(Id, Sel, bool) -> Id = transmute(imp());
    f(o, sel(s), a)
}
unsafe fn msg_f64(o: Id, s: &str, a: f64) -> Id {
    let f: extern "C" fn(Id, Sel, f64) -> Id = transmute(imp());
    f(o, sel(s), a)
}
unsafe fn msg_ret_int(o: Id, s: &str) -> isize {
    let f: extern "C" fn(Id, Sel) -> isize = transmute(imp());
    f(o, sel(s))
}
unsafe fn msg_err(o: Id, s: &str) -> bool {
    let f: extern "C" fn(Id, Sel, *mut Id) -> bool = transmute(imp());
    let mut err: Id = NIL;
    f(o, sel(s), &mut err)
}

fn ns(s: &str) -> Id {
    unsafe {
        let f: extern "C" fn(Id, Sel, *const u8, usize, usize) -> Id = transmute(imp());
        let obj = msg(class("NSString"), "alloc");
        f(
            obj,
            sel("initWithBytes:length:encoding:"),
            s.as_ptr(),
            s.len(),
            4, /* UTF8 */
        )
    }
}

unsafe fn ns_app() -> Id {
    msg(class("NSApplication"), "sharedApplication")
}

// ---------- API cho macos.rs ----------

/// Khởi tạo NSApplication ở chế độ "accessory" (không icon Dock).
pub fn init_app() {
    unsafe {
        let app = ns_app();
        msg_int(app, "setActivationPolicy:", 1); // NSApplicationActivationPolicyAccessory
    }
    refresh_lang();
}

/// Đọc ngôn ngữ ưu tiên của hệ thống; trả về true nếu vừa đổi.
pub fn refresh_lang() -> bool {
    unsafe {
        let defaults = msg(class("NSUserDefaults"), "standardUserDefaults");
        let langs = msg_id(defaults, "objectForKey:", ns("AppleLanguages"));
        if langs.is_null() {
            return false;
        }
        let first = msg(langs, "firstObject");
        if first.is_null() {
            return false;
        }
        let c = msg(first, "UTF8String") as *const c_char;
        if c.is_null() {
            return false;
        }
        let vi = std::ffi::CStr::from_ptr(c).to_string_lossy().starts_with("vi");
        let changed = vi != is_vi();
        set_vi(vi);
        changed
    }
}

pub fn alert(title: &str, text: &str) {
    alert_buttons(title, text, &["OK"]);
}

/// Hộp thoại nhiều nút; trả về chỉ số nút được bấm (0 = nút đầu).
pub fn alert_buttons(title: &str, text: &str, buttons: &[&str]) -> usize {
    unsafe {
        let a = msg(msg(class("NSAlert"), "alloc"), "init");
        msg_id(a, "setMessageText:", ns(title));
        msg_id(a, "setInformativeText:", ns(text));
        for b in buttons {
            msg_id(a, "addButtonWithTitle:", ns(b));
        }
        msg_bool(ns_app(), "activateIgnoringOtherApps:", true);
        (msg_ret_int(a, "runModal") - 1000).max(0) as usize
    }
}

pub fn run_app() {
    unsafe {
        msg(ns_app(), "run");
    }
}

pub fn terminate() {
    unsafe {
        msg_id(ns_app(), "terminate:", NIL);
    }
}

pub fn open_url(url: &str) {
    let _ = std::process::Command::new("/usr/bin/open").arg(url).status();
}

// ---------- trạng thái dùng chung ----------

static STATUS_ITEM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static DEVICE_ITEM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static DISPLAY_MENU: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static TARGET: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static STATUS_BUTTON: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static WINDOW: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
/// Item bật/tắt trong menu, theo tag (chỉ số = tag).
static TOGGLE_ITEMS: [AtomicPtr<c_void>; 10] = [const { AtomicPtr::new(ptr::null_mut()) }; 10];
/// Submenu radio: tốc độ, hướng, cỡ giao diện.
static SPEED_MENU: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIENT_MENU: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SCALE_MENU: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

// Khoá tiếng Việt, dịch bằng tr() khi hiển thị.
const SPEEDS: [(&str, f64); 5] = [
    ("Rất chậm", 0.4),
    ("Chậm", 0.8),
    ("Vừa", 1.0),
    ("Nhanh (mặc định)", 1.2),
    ("Rất nhanh", 1.8),
];
const ORIENTS: [&str; 3] = ["Ngang", "Dọc (xoay phải 90°)", "Dọc (xoay trái 90°)"];
const SCALES: [&str; 4] = ["Gốc", "Lớn hơn (1.3×)", "Lớn (1.6×)", "Rất lớn (2×, nét)"];

// Tag của các công tắc.
const T_HIDE: isize = 1;
const T_INVERT: isize = 2;
const T_MOMENTUM: isize = 3;
const T_PINCH: isize = 4;
const T_SWIPE: isize = 5;
const T_ONEFINGER: isize = 6;
const T_PAUSE: isize = 7;
const T_LOGIN: isize = 8;
const T_SNAP: isize = 9;

/// (tag, tên trong cửa sổ, mô tả)
const TOGGLES: [(isize, &str, &str); 9] = [
    (T_HIDE, "Ẩn con trỏ khi chạm", "Ẩn con trỏ khi dùng cảm ứng, hiện lại khi bạn dùng chuột hoặc trackpad."),
    (T_INVERT, "Đảo chiều cuộn", "Mặc định nội dung đi theo ngón tay (kiểu iPad). Bật để đảo hướng."),
    (T_MOMENTUM, "Cuộn quán tính", "Nội dung tiếp tục trượt và giảm dần sau khi nhấc ngón."),
    (T_PINCH, "Chụm / xoè để zoom", "Chụm hoặc xoè 2 ngón để phóng to/thu nhỏ (Safari, Preview, Maps…)."),
    (T_SWIPE, "Vuốt 3 ngón", "Vuốt 3 ngón để chuyển Space, mở Mission Control và App Exposé."),
    (
        T_ONEFINGER,
        "Kéo 1 ngón để cuộn",
        "Kéo 1 ngón để cuộn như iPad. Giữ yên ~0.3s rồi kéo để dùng như chuột (thanh trượt, chọn chữ).",
    ),
    (T_PAUSE, "Tạm dừng", "Tạm ngừng xử lý cảm ứng (chạm sẽ không có tác dụng)."),
    (T_LOGIN, "Mở cùng macOS", "Tự chạy khi đăng nhập macOS."),
    (
        T_SNAP,
        "Chạm thông minh",
        "Khi chạm trượt một nút nhỏ, tự chọn nút gần nhất trong vòng ~20 điểm (cần quyền Accessibility).",
    ),
];

fn toggle_info(tag: isize) -> (&'static str, &'static str) {
    TOGGLES.iter().find(|t| t.0 == tag).map(|t| (t.1, t.2)).unwrap_or(("", ""))
}

fn save_prefs() {
    with_app(|a| a.prefs()).save();
}

fn nearest_speed(v: f64) -> usize {
    SPEEDS
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 .1 - v).abs().total_cmp(&(b.1 .1 - v).abs()))
        .map(|(i, _)| i)
        .unwrap_or(3)
}

unsafe fn login_enabled() -> bool {
    let cls = class("SMAppService");
    if cls.is_null() {
        return false;
    }
    let svc = msg(cls, "mainAppService");
    !svc.is_null() && msg_ret_int(svc, "status") == 1
}

/// Trạng thái hiện tại của một công tắc.
fn toggle_state(tag: isize) -> bool {
    let (p, paused) = with_app(|a| (a.prefs(), a.paused));
    match tag {
        T_HIDE => p.hide_cursor,
        T_INVERT => p.invert_scroll,
        T_MOMENTUM => p.momentum,
        T_PINCH => p.pinch,
        T_SWIPE => p.swipe,
        T_ONEFINGER => p.one_finger_scroll,
        T_PAUSE => paused,
        T_SNAP => p.snap,
        T_LOGIN => unsafe { login_enabled() },
        _ => false,
    }
}

/// Đảo một công tắc, lưu cài đặt, trả về trạng thái mới.
fn flip(tag: isize) -> bool {
    let on = match tag {
        T_HIDE => with_app(|a| a.toggle_hide_cursor()),
        T_SNAP => with_app(|a| a.toggle_snap()),
        T_INVERT => with_app(|a| a.toggle_invert_scroll()),
        T_MOMENTUM | T_PINCH | T_SWIPE | T_ONEFINGER => with_app(|a| {
            let c = &mut a.engine.cfg;
            let f = match tag {
                T_MOMENTUM => &mut c.momentum,
                T_PINCH => &mut c.pinch,
                T_SWIPE => &mut c.three_finger_swipe,
                _ => &mut c.one_finger_scroll,
            };
            *f = !*f;
            *f
        }),
        T_PAUSE => {
            let paused = with_app(|a| {
                a.set_paused(!a.paused);
                a.paused
            });
            refresh_status(paused);
            paused
        }
        T_LOGIN => unsafe {
            let svc = msg(class("SMAppService"), "mainAppService");
            if !svc.is_null() {
                let ok = if login_enabled() {
                    msg_err(svc, "unregisterAndReturnError:")
                } else {
                    msg_err(svc, "registerAndReturnError:")
                };
                if !ok {
                    alert(
                        &tr("Không đổi được \"Mở cùng macOS\""),
                        &tr("Hãy chép {app}.app vào thư mục Applications rồi thử lại. Có thể bật thủ công trong System Settings → General → Login Items."),
                    );
                }
            }
            login_enabled()
        },
        _ => false,
    };
    save_prefs();
    on
}

fn set_speed(idx: usize) {
    if let Some(&(_, v)) = SPEEDS.get(idx) {
        with_app(|a| a.set_scroll_speed(v));
        save_prefs();
    }
}

fn set_orientation(idx: usize) {
    with_app(|a| a.set_orientation(idx as u8));
    save_prefs();
}

fn set_scale(level: usize) {
    let err = with_app(|a| a.set_scale(level));
    if !err.is_empty() {
        alert(&tr("Không đổi được cỡ giao diện"), &err);
    }
}

fn refresh_status(paused: bool) {
    unsafe {
        let it = STATUS_ITEM.load(Ordering::SeqCst);
        if !it.is_null() {
            let t = if paused { "⏸  Đang tạm dừng" } else { "●  Đang hoạt động" };
            msg_id(it, "setTitle:", ns(&tr(t)));
        }
        let b = STATUS_BUTTON.load(Ordering::SeqCst);
        if !b.is_null() {
            msg_f64(b, "setAlphaValue:", if paused { 0.4 } else { 1.0 });
        }
    }
}

/// Cập nhật dòng thiết bị trong menu (gọi khi cắm/rút).
pub fn set_devices(devices: &[String]) {
    unsafe {
        let it = DEVICE_ITEM.load(Ordering::SeqCst);
        if it.is_null() {
            return;
        }
        let t = if devices.is_empty() {
            tr("    Chưa thấy màn hình cảm ứng – hãy cắm cáp USB")
        } else {
            format!("    {}", devices.join(", "))
        };
        msg_id(it, "setTitle:", ns(&t));
    }
}

unsafe fn set_radio(menu: Id, selected: usize) {
    if menu.is_null() {
        return;
    }
    let n = msg_ret_int(menu, "numberOfItems");
    for i in 0..n {
        let it = msg_int(menu, "itemAtIndex:", i);
        msg_int(it, "setState:", (i as usize == selected) as isize);
    }
}

/// Đồng bộ mọi dấu chọn trong menu với trạng thái thật (gọi khi menu sắp mở).
fn sync_menu() {
    unsafe {
        for (tag, _, _) in TOGGLES {
            let it = TOGGLE_ITEMS[tag as usize].load(Ordering::SeqCst);
            if !it.is_null() {
                msg_int(it, "setState:", toggle_state(tag) as isize);
            }
        }
        let (p, scale, paused) = with_app(|a| (a.prefs(), a.scale_level, a.paused));
        set_radio(SPEED_MENU.load(Ordering::SeqCst), nearest_speed(p.scroll_speed));
        set_radio(ORIENT_MENU.load(Ordering::SeqCst), p.orientation as usize);
        set_radio(SCALE_MENU.load(Ordering::SeqCst), scale);
        refresh_status(paused);
    }
    rebuild_displays();
}

/// Dựng lại danh sách "Màn hình đích" (gọi khi cắm/rút hoặc đổi cấu hình màn hình).
pub fn rebuild_displays() {
    unsafe {
        let sub = DISPLAY_MENU.load(Ordering::SeqCst);
        let target = TARGET.load(Ordering::SeqCst);
        if sub.is_null() || target.is_null() {
            return;
        }
        let cur = with_app(|a| a.display_choice());
        msg(sub, "removeAllItems");
        let it = add_item(sub, target, &tr("Tự động (màn hình rời)"), Some("setDisplay:"), "", Some(cur.is_none()));
        msg_int(it, "setTag:", 0);
        for (i, label) in crate::macos::display_labels().iter().enumerate() {
            let it = add_item(sub, target, label, Some("setDisplay:"), "", Some(cur == Some(i)));
            msg_int(it, "setTag:", i as isize + 1);
        }
    }
}

unsafe fn tag_of(sender: Id) -> isize {
    msg_ret_int(sender, "tag")
}

// ---------- handler (menu) ----------

extern "C" fn on_toggle(_this: Id, _cmd: Sel, sender: Id) {
    let on = flip(unsafe { tag_of(sender) });
    unsafe { msg_int(sender, "setState:", on as isize) };
}

extern "C" fn on_set_speed(_this: Id, _cmd: Sel, sender: Id) {
    let idx = unsafe { tag_of(sender) } as usize;
    set_speed(idx);
    unsafe { set_radio(msg(sender, "menu"), idx) };
}

extern "C" fn on_set_orientation(_this: Id, _cmd: Sel, sender: Id) {
    let idx = unsafe { tag_of(sender) } as usize;
    set_orientation(idx);
    unsafe { set_radio(msg(sender, "menu"), idx) };
}

extern "C" fn on_set_scale(_this: Id, _cmd: Sel, sender: Id) {
    let idx = unsafe { tag_of(sender) } as usize;
    set_scale(idx);
    unsafe { set_radio(msg(sender, "menu"), with_app(|a| a.scale_level)) };
}

fn choose_display(tag: isize) {
    with_app(|a| a.set_display(if tag <= 0 { None } else { Some(tag as usize - 1) }));
    rebuild_displays();
    save_prefs();
}

extern "C" fn on_set_display(_this: Id, _cmd: Sel, sender: Id) {
    choose_display(unsafe { tag_of(sender) });
}

/// Menu chính sắp mở: đồng bộ ngôn ngữ (dựng lại nếu đổi) và các dấu chọn.
extern "C" fn on_menu_update(_this: Id, _cmd: Sel, menu: Id) {
    unsafe {
        if refresh_lang() {
            let target = TARGET.load(Ordering::SeqCst);
            msg(menu, "removeAllItems");
            build_menu(menu, target);
        }
    }
    sync_menu();
    let devices = with_app(|a| a.devices.clone());
    set_devices(&devices);
}

extern "C" fn on_help(_this: Id, _cmd: Sel, _sender: Id) {
    refresh_lang();
    let text = tr(
        "☝︎  Chạm 1 ngón: click\n☝︎  Chạm 2 lần nhanh: double-click\n☝︎  Kéo 1 ngón: cuộn trang\n☝︎  Giữ yên 1 ngón ~0.3s rồi kéo: kéo chuột (thanh trượt, chọn chữ, kéo cửa sổ)\n✌︎  Chạm 2 ngón: click phải\n✌︎  Kéo 2 ngón: cuộn (có quán tính)\n✌︎  Chụm / xoè 2 ngón: phóng to, thu nhỏ\n✋  Vuốt 3 ngón trái/phải: chuyển Space\n✋  Vuốt 3 ngón lên/xuống: Mission Control / App Exposé\n\n{tip}",
    )
    .replace("{tip}", &tr("Cuộn nhanh hoặc chậm quá? Chỉnh ở \"Cài đặt…\" → Tốc độ cuộn."));
    alert(&tr("Cử chỉ trên màn hình cảm ứng"), &text);
}

extern "C" fn on_troubleshoot(_this: Id, _cmd: Sel, _sender: Id) {
    refresh_lang();
    let (paused, devices) = with_app(|a| (a.paused, a.device_summary()));
    let state = tr(if paused { "Đang tạm dừng" } else { "Đang hoạt động" });
    let text = tr("Trạng thái: {state}\n{devices}\n\nNếu chạm không có tác dụng:\n1. Kiểm tra cáp USB của màn hình cảm ứng.\n2. Bật Input Monitoring và Accessibility cho {app}.\n3. Thoát rồi mở lại {app}.")
        .replace("{state}", &state)
        .replace("{devices}", &devices);
    match alert_buttons(
        &tr("Khắc phục sự cố"),
        &text,
        &[&tr("Đóng"), &tr("Mở cài đặt quyền"), &tr("Xem nhật ký")],
    ) {
        1 => open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"),
        2 => on_open_log(NIL, ptr::null(), NIL),
        _ => {}
    }
}

extern "C" fn on_open_options(_this: Id, _cmd: Sel, _sender: Id) {
    let path = crate::options_file_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if !path.exists() {
        let _ = std::fs::write(&path, crate::OPTIONS_FILE_TEMPLATE);
    }
    let _ = std::process::Command::new("/usr/bin/open")
        .arg("-t")
        .arg(&path)
        .status();
}

extern "C" fn on_open_log(_this: Id, _cmd: Sel, _sender: Id) {
    let _ = std::process::Command::new("/usr/bin/open")
        .arg("-t")
        .arg(crate::log_file_path())
        .status();
}

extern "C" fn on_quit(_this: Id, _cmd: Sel, _sender: Id) {
    request_quit();
}

// ---------- Giới thiệu (hộp thoại About chuẩn của macOS) ----------

#[repr(C)]
#[derive(Clone, Copy)]
struct Range {
    location: usize,
    length: usize,
}

extern "C" fn on_about(_this: Id, _cmd: Sel, _sender: Id) {
    show_about();
}

pub fn show_about() {
    refresh_lang();
    unsafe {
        let desc = tr("Biến màn hình cảm ứng USB rời thành thao tác native trên macOS: chạm, cuộn, pinch và vuốt nhiều ngón.");
        let prefix = format!("{desc}\n\n{}: ", tr("Liên hệ"));
        let text = format!("{prefix}{CONTACT_EMAIL}");
        let credits = msg_id(msg(class("NSMutableAttributedString"), "alloc"), "initWithString:", ns(&text));
        let url = msg_id(class("NSURL"), "URLWithString:", ns(&format!("mailto:{CONTACT_EMAIL}")));
        let f: extern "C" fn(Id, Sel, Id, Id, Range) = transmute(imp());
        f(
            credits,
            sel("addAttribute:value:range:"),
            ns("NSLink"),
            url,
            Range {
                location: prefix.encode_utf16().count(),
                length: CONTACT_EMAIL.encode_utf16().count(),
            },
        );
        let opts = msg_int(class("NSMutableDictionary"), "dictionaryWithCapacity:", 4);
        msg_id2(opts, "setObject:forKey:", ns(APP_NAME), ns("ApplicationName"));
        msg_id2(opts, "setObject:forKey:", ns(env!("CARGO_PKG_VERSION")), ns("ApplicationVersion"));
        msg_id2(opts, "setObject:forKey:", ns(""), ns("Version"));
        msg_id2(opts, "setObject:forKey:", credits, ns("Credits"));
        msg_bool(ns_app(), "activateIgnoringOtherApps:", true);
        msg_id(ns_app(), "orderFrontStandardAboutPanelWithOptions:", opts);
    }
}

// ---------- cửa sổ cài đặt: kiểu System Settings (sidebar + nhóm form) ----------
// Quy tắc bố cục xem .claude/skills/macos-settings-ui/SKILL.md.

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Insets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

unsafe fn msg_insets(o: Id, s: &str, i: Insets) -> Id {
    let f: extern "C" fn(Id, Sel, Insets) -> Id = transmute(imp());
    f(o, sel(s), i)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Size {
    w: f64,
    h: f64,
}

unsafe fn msg_rect(o: Id, s: &str, r: Rect) -> Id {
    let f: extern "C" fn(Id, Sel, Rect) -> Id = transmute(imp());
    f(o, sel(s), r)
}

unsafe fn msg_sel(o: Id, s: &str, a: &str) -> Id {
    let f: extern "C" fn(Id, Sel, Sel) -> Id = transmute(imp());
    f(o, sel(s), sel(a))
}

unsafe fn msg_size(o: Id, s: &str, z: Size) -> Id {
    let f: extern "C" fn(Id, Sel, Size) -> Id = transmute(imp());
    f(o, sel(s), z)
}

unsafe fn font(size: f64, bold: bool) -> Id {
    msg_f64(class("NSFont"), if bold { "boldSystemFontOfSize:" } else { "systemFontOfSize:" }, size)
}

fn color(name: &str) -> Id {
    unsafe { msg(class("NSColor"), name) }
}

/// NSTextAlignmentCenter: arm64 dùng giá trị kiểu iOS (giữa = 1, phải = 2), x86_64 thì ngược lại.
#[cfg(target_arch = "aarch64")]
const TEXT_CENTER: isize = 1;
#[cfg(not(target_arch = "aarch64"))]
const TEXT_CENTER: isize = 2;

// Hằng số bố cục (bội số của 4/8).
const WIN_W: f64 = 780.0;
const WIN_H: f64 = 560.0;
const SIDEBAR_W: f64 = 200.0;
const PAD: f64 = 24.0; // lề trang
const GPAD: f64 = 16.0; // lề trong nhóm
const GAP: f64 = 20.0; // khoảng cách giữa các nhóm
const POPUP_W: f64 = 200.0;
const SWITCH_W: f64 = 40.0;
const BUTTON_W: f64 = 120.0;

/// Nhãn một dòng; `y` là mép dưới, `h` là chiều cao.
unsafe fn label(parent: Id, text: &str, r: Rect, size: f64, bold: bool, secondary: bool) {
    let l = msg_id(class("NSTextField"), "labelWithString:", ns(text));
    msg_rect(l, "setFrame:", r);
    msg_id(l, "setFont:", font(size, bold));
    if secondary {
        msg_id(l, "setTextColor:", color("secondaryLabelColor"));
    }
    msg_id(parent, "addSubview:", l);
}

/// Nhãn nhiều dòng tự xuống dòng.
unsafe fn wrap_label(parent: Id, text: &str, r: Rect, size: f64, secondary: bool, center: bool) {
    let l = msg_id(class("NSTextField"), "wrappingLabelWithString:", ns(text));
    msg_rect(l, "setFrame:", r);
    msg_id(l, "setFont:", font(size, false));
    if secondary {
        msg_id(l, "setTextColor:", color("secondaryLabelColor"));
    }
    if center {
        msg_int(l, "setAlignment:", TEXT_CENTER);
    }
    msg_id(parent, "addSubview:", l);
}

/// Ước lượng số dòng của đoạn chữ 11pt trong bề rộng `w`.
fn lines_for(text: &str, w: f64) -> f64 {
    let per_line = (w / 5.6).max(8.0);
    ((text.chars().count() as f64 / per_line).ceil()).clamp(1.0, 3.0)
}

enum Ctl {
    Switch(isize),
    Popup { items: Vec<String>, selected: usize, action: &'static str },
    Button { title: String, action: &'static str },
}

struct RowSpec {
    title: String,
    desc: String,
    ctl: Ctl,
}

fn switch_row(tag: isize) -> RowSpec {
    let (name, desc) = toggle_info(tag);
    RowSpec { title: tr(name), desc: tr(desc), ctl: Ctl::Switch(tag) }
}

fn ctl_width(c: &Ctl) -> f64 {
    match c {
        Ctl::Switch(_) => SWITCH_W,
        Ctl::Popup { .. } => POPUP_W,
        Ctl::Button { .. } => BUTTON_W,
    }
}

/// Chiều cao một hàng: 8 trên + tiêu đề 18 + 2 + mô tả (14/dòng) + 8 dưới; tối thiểu 52 (đủ cho ngón tay).
fn row_height(r: &RowSpec, cw: f64) -> f64 {
    let text_w = cw - 2.0 * GPAD - ctl_width(&r.ctl) - 12.0;
    let desc_h = if r.desc.is_empty() { 0.0 } else { lines_for(&r.desc, text_w) * 14.0 + 2.0 };
    (8.0 + 18.0 + desc_h + 8.0).max(52.0)
}

unsafe fn box_view(r: Rect, kind: isize) -> Id {
    let b = msg_rect(msg(class("NSBox"), "alloc"), "initWithFrame:", r);
    msg_int(b, "setBoxType:", kind); // 4 = custom, 2 = separator
    b
}

/// Một nhóm form: hộp bo góc 10pt + các hàng ngăn bằng đường kẻ thụt lề. `top` là mép trên; trả về mép dưới.
unsafe fn group(page: Id, target: Id, top: f64, cw: f64, rows: &[RowSpec]) -> f64 {
    let heights: Vec<f64> = rows.iter().map(|r| row_height(r, cw)).collect();
    let total: f64 = heights.iter().sum();
    let gx = PAD;
    let card = box_view(Rect { x: gx, y: top - total, w: cw, h: total }, 4);
    msg_int(card, "setTitlePosition:", 0);
    msg_f64(card, "setBorderWidth:", 0.5);
    msg_f64(card, "setCornerRadius:", 10.0);
    msg_id(card, "setBorderColor:", color("separatorColor"));
    msg_id(card, "setFillColor:", color("controlBackgroundColor"));
    msg_size(card, "setContentViewMargins:", Size { w: 0.0, h: 0.0 });
    msg_id(page, "addSubview:", card);

    let mut y = top;
    for (i, (r, h)) in rows.iter().zip(&heights).enumerate() {
        let (row_top, mid) = (y, y - h / 2.0);
        let cwid = ctl_width(&r.ctl);
        let text_w = cw - 2.0 * GPAD - cwid - 12.0;
        let tx = gx + GPAD;
        if r.desc.is_empty() {
            label(page, &r.title, Rect { x: tx, y: mid - 9.0, w: text_w, h: 18.0 }, 13.0, false, false);
        } else {
            label(page, &r.title, Rect { x: tx, y: row_top - 8.0 - 18.0, w: text_w, h: 18.0 }, 13.0, false, false);
            let dh = lines_for(&r.desc, text_w) * 14.0;
            wrap_label(page, &r.desc, Rect { x: tx, y: row_top - 8.0 - 18.0 - 2.0 - dh, w: text_w, h: dh }, 11.0, true, false);
        }
        let cx = gx + cw - GPAD - cwid;
        match &r.ctl {
            Ctl::Switch(tag) => {
                let sw = msg_rect(msg(class("NSSwitch"), "alloc"), "initWithFrame:", Rect { x: cx, y: mid - 11.0, w: cwid, h: 22.0 });
                msg_int(sw, "setState:", toggle_state(*tag) as isize);
                msg_id(sw, "setTarget:", target);
                msg_sel(sw, "setAction:", "winToggle:");
                msg_int(sw, "setTag:", *tag);
                msg_id(page, "addSubview:", sw);
            }
            Ctl::Popup { items, selected, action } => {
                let f: extern "C" fn(Id, Sel, Rect, bool) -> Id = transmute(imp());
                let pop = f(msg(class("NSPopUpButton"), "alloc"), sel("initWithFrame:pullsDown:"), Rect { x: cx, y: mid - 14.0, w: cwid, h: 28.0 }, false);
                for it in items {
                    msg_id(pop, "addItemWithTitle:", ns(it));
                }
                msg_int(pop, "selectItemAtIndex:", *selected as isize);
                msg_id(pop, "setTarget:", target);
                msg_sel(pop, "setAction:", action);
                msg_id(page, "addSubview:", pop);
            }
            Ctl::Button { title, action } => {
                let b = msg_rect(msg(class("NSButton"), "alloc"), "initWithFrame:", Rect { x: cx, y: mid - 14.0, w: cwid, h: 28.0 });
                msg_id(b, "setTitle:", ns(title));
                msg_int(b, "setBezelStyle:", if class("NSGlassEffectView").is_null() { 1 } else { 16 });
                msg_id(b, "setTarget:", target);
                msg_sel(b, "setAction:", action);
                msg_id(page, "addSubview:", b);
            }
        }
        y -= h;
        if i + 1 < rows.len() {
            let sep = box_view(Rect { x: gx + GPAD, y: y - 0.5, w: cw - GPAD, h: 1.0 }, 2);
            msg_id(page, "addSubview:", sep);
        }
    }
    top - total
}

/// Chú thích dưới nhóm; trả về mép dưới.
unsafe fn footnote(page: Id, top: f64, cw: f64, text: &str) -> f64 {
    let w = cw - 2.0 * GPAD;
    let h = lines_for(text, w) * 14.0;
    wrap_label(page, text, Rect { x: PAD + GPAD, y: top - 8.0 - h, w, h }, 11.0, true, false);
    top - 8.0 - h
}

// Trang của sidebar: (biểu tượng SF Symbol, khoá tên)
const PAGES: [(&str, &str); 6] = [
    ("gearshape", "Chung"),
    ("arrow.up.and.down.and.arrow.left.and.right", "Cuộn"),
    ("hand.tap", "Cử chỉ"),
    ("display", "Màn hình"),
    ("wrench.and.screwdriver", "Công cụ"),
    ("info.circle", "Giới thiệu"),
];

static DETAIL: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static TABLE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SELECTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Dựng nội dung trang `idx` vào `detail` (view có chiều cao `h`, gốc dưới-trái); trả về mép dưới nội dung.
unsafe fn build_page(detail: Id, idx: usize, h: f64) -> f64 {
    let target = TARGET.load(Ordering::SeqCst);
    let cw = WIN_W - SIDEBAR_W - 2.0 * PAD;
    let (p, scale, display, devices, calibrated, calib_stale) =
        with_app(|a| (a.prefs(), a.scale_level, a.display_choice(), a.device_summary(), a.is_calibrated(), a.calib_stale()));

    // Tiêu đề trang (dưới vùng titlebar trong suốt)
    let title_top = h - 52.0;
    label(detail, &tr(PAGES[idx].1), Rect { x: PAD, y: title_top - 28.0, w: cw, h: 28.0 }, 22.0, true, false);
    let top = title_top - 28.0 - GAP;

    let y_end = match idx {
        0 => {
            let rows = [switch_row(T_SNAP), switch_row(T_HIDE), switch_row(T_LOGIN), switch_row(T_PAUSE)];
            let y = group(detail, target, top, cw, &rows);
            let state = tr(if with_app(|a| a.paused) { "Đang tạm dừng" } else { "Đang hoạt động" });
            footnote(detail, y, cw, &format!("{state} · {}", devices.replace('\n', " · ")))
        }
        1 => {
            let speeds: Vec<String> = SPEEDS.iter().map(|s| tr(s.0)).collect();
            let rows = [
                RowSpec {
                    title: tr("Tốc độ cuộn"),
                    desc: tr("Tốc độ nội dung trượt khi bạn kéo ngón tay."),
                    ctl: Ctl::Popup { items: speeds, selected: nearest_speed(p.scroll_speed), action: "popSpeed:" },
                },
                switch_row(T_INVERT),
                switch_row(T_MOMENTUM),
            ];
            group(detail, target, top, cw, &rows)
        }
        2 => {
            let rows = [switch_row(T_PINCH), switch_row(T_SWIPE), switch_row(T_ONEFINGER)];
            group(detail, target, top, cw, &rows)
        }
        3 => {
            let mut targets = vec![tr("Tự động (màn hình rời)")];
            targets.extend(crate::macos::display_labels());
            let orients: Vec<String> = ORIENTS.iter().map(|s| tr(s)).collect();
            let scales: Vec<String> = SCALES.iter().map(|s| tr(s)).collect();
            let rows = [
                RowSpec {
                    title: tr("Màn hình đích"),
                    desc: tr("Màn hình nhận thao tác chạm. Tự động = màn hình rời đầu tiên."),
                    ctl: Ctl::Popup { items: targets, selected: display.map(|d| d + 1).unwrap_or(0), action: "winDisplay:" },
                },
                RowSpec {
                    title: tr("Hướng màn hình"),
                    desc: tr("Chọn nếu màn hình cảm ứng được xoay dọc, để vị trí chạm khớp đúng."),
                    ctl: Ctl::Popup { items: orients, selected: p.orientation as usize, action: "popOrient:" },
                },
                RowSpec {
                    title: tr("Cỡ giao diện"),
                    desc: tr("Phóng to giao diện hệ thống để nút dễ chạm hơn. Chỉ mức 2× là sắc nét; 1.3× và 1.6× có thể hơi mờ do màn hình không có độ phân giải trung gian. Tự khôi phục khi thoát."),
                    ctl: Ctl::Popup { items: scales, selected: scale, action: "popScale:" },
                },
            ];
            group(detail, target, top, cw, &rows)
        }
        4 => {
            let rows = [
                RowSpec {
                    title: tr("Hiệu chỉnh vị trí chạm"),
                    desc: tr("Nếu chạm bị lệch, chạm lần lượt vào 9 dấu + trên màn hình để căn chỉnh lại."),
                    ctl: Ctl::Button { title: tr("Bắt đầu"), action: "startCalib:" },
                },
                RowSpec {
                    title: tr("Kiểm tra vùng chạm"),
                    desc: tr("Rê ngón tay khắp màn hình: vùng nhận chạm hiện màu vàng, vùng tối là chỗ cảm biến không nhận. Kết quả dùng để kéo vùng chạm ra sát mép."),
                    ctl: Ctl::Button { title: tr("Bắt đầu"), action: "startTouchTest:" },
                },
                RowSpec {
                    title: tr("Kiểm tra màn hình"),
                    desc: tr("Hiện lần lượt các màu đặc để tìm điểm ảnh chết hoặc sáng không đều. Chạm để đổi màu."),
                    ctl: Ctl::Button { title: tr("Bắt đầu"), action: "startScreenTest:" },
                },
                RowSpec {
                    title: tr("Đặt lại hiệu chỉnh"),
                    desc: tr(if calibrated {
                        "Đang dùng hiệu chỉnh riêng. Đặt lại để về mặc định."
                    } else if calib_stale {
                        "Hiệu chỉnh cũ được lưu cho độ phân giải khác nên đang tắt. Hãy hiệu chỉnh lại hoặc đặt lại."
                    } else {
                        "Chưa hiệu chỉnh, đang dùng vị trí mặc định của cảm biến."
                    }),
                    ctl: Ctl::Button { title: tr("Đặt lại"), action: "resetCalib:" },
                },
            ];
            group(detail, target, top, cw, &rows)
        }
        _ => {
            // Giới thiệu: biểu tượng, tên, phiên bản, mô tả, rồi nhóm liên hệ/trợ giúp.
            let mid_x = PAD + cw / 2.0;
            let icon = msg(ns_app(), "applicationIconImage");
            if !icon.is_null() {
                let iv = msg_id(class("NSImageView"), "imageViewWithImage:", icon);
                msg_rect(iv, "setFrame:", Rect { x: mid_x - 48.0, y: top - 96.0, w: 96.0, h: 96.0 });
                msg_id(detail, "addSubview:", iv);
            }
            let mut y = top - 96.0 - 12.0;
            label_center(detail, APP_NAME, mid_x, cw, y - 28.0, 22.0, true, false);
            y -= 28.0;
            label_center(detail, crate::i18n::APP_TAGLINE, mid_x, cw, y - 18.0, 13.0, false, true);
            y -= 18.0;
            label_center(detail, &format!("{} {}", tr("Phiên bản"), env!("CARGO_PKG_VERSION")), mid_x, cw, y - 16.0, 11.0, false, true);
            y -= 16.0 + 8.0;
            let desc = tr("Biến màn hình cảm ứng USB rời thành thao tác native trên macOS: chạm, cuộn, pinch và vuốt nhiều ngón.");
            let dw = 420.0;
            let dh = lines_for(&desc, dw) * 14.0;
            wrap_label(detail, &desc, Rect { x: mid_x - dw / 2.0, y: y - dh, w: dw, h: dh }, 11.0, true, true);
            y -= dh + GAP;
            let rows = [
                RowSpec { title: tr("Liên hệ"), desc: CONTACT_EMAIL.to_string(), ctl: Ctl::Button { title: tr("Gửi email"), action: "openMail:" } },
                RowSpec {
                    title: tr("Hướng dẫn cử chỉ…").trim_end_matches('…').to_string(),
                    desc: tr("Xem các cử chỉ được hỗ trợ."),
                    ctl: Ctl::Button { title: tr("Mở"), action: "showHelp:" },
                },
                RowSpec {
                    title: tr("Khắc phục sự cố…").trim_end_matches('…').to_string(),
                    desc: tr("Kiểm tra trạng thái thiết bị và quyền của ứng dụng."),
                    ctl: Ctl::Button { title: tr("Mở"), action: "troubleshoot:" },
                },
            ];
            group(detail, target, y, cw, &rows)
        }
    };
    y_end
}

/// Dựng trang vào vùng cuộn của cửa sổ: đo chiều cao nội dung bằng một lượt dựng thử, rồi dựng thật.
unsafe fn show_page(idx: usize) {
    let scroll = DETAIL.load(Ordering::SeqCst);
    if scroll.is_null() || TARGET.load(Ordering::SeqCst).is_null() {
        return;
    }
    SELECTED.store(idx, Ordering::SeqCst);
    let dw = WIN_W - SIDEBAR_W;
    let probe = new_content(dw, 4000.0);
    let y_end = build_page(probe, idx, 4000.0);
    let h = (4000.0 - y_end + PAD).max(WIN_H);
    let page = new_content(dw, h);
    build_page(page, idx, h);
    msg_id(scroll, "setDocumentView:", page);
    // Cuộn về đầu trang (gốc view ở dưới-trái nên đầu trang là y = h - chiều cao vùng nhìn).
    let clip = msg(scroll, "contentView");
    let f: extern "C" fn(Id, Sel, Point) = transmute(imp());
    f(clip, sel("scrollToPoint:"), Point { x: 0.0, y: h - WIN_H });
    msg_id(scroll, "reflectScrolledClipView:", clip);
    if h > WIN_H + 0.5 {
        msg(scroll, "flashScrollers"); // báo hiệu còn nội dung bên dưới (chỉ có tác dụng với thanh cuộn kiểu overlay)
    }
}

/// Nhãn căn giữa quanh `mid_x`.
unsafe fn label_center(parent: Id, text: &str, mid_x: f64, cw: f64, y: f64, size: f64, bold: bool, secondary: bool) {
    let l = msg_id(class("NSTextField"), "labelWithString:", ns(text));
    msg_rect(l, "setFrame:", Rect { x: mid_x - cw / 2.0, y, w: cw, h: size + 5.0 });
    msg_id(l, "setFont:", font(size, bold));
    msg_int(l, "setAlignment:", TEXT_CENTER);
    if secondary {
        msg_id(l, "setTextColor:", color("secondaryLabelColor"));
    }
    msg_id(parent, "addSubview:", l);
}

// ----- nguồn dữ liệu / delegate của sidebar (NSTableView view-based) -----

extern "C" fn table_rows(_this: Id, _cmd: Sel, _table: Id) -> isize {
    PAGES.len() as isize
}

extern "C" fn table_view(_this: Id, _cmd: Sel, _table: Id, _col: Id, row: isize) -> Id {
    unsafe {
        let Some(&(symbol, name)) = PAGES.get(row as usize) else { return NIL };
        let cell = msg_rect(msg(class("NSView"), "alloc"), "initWithFrame:", Rect { x: 0.0, y: 0.0, w: SIDEBAR_W - 16.0, h: 32.0 });
        let img = msg_id2(class("NSImage"), "imageWithSystemSymbolName:accessibilityDescription:", ns(symbol), NIL);
        if !img.is_null() {
            let iv = msg_id(class("NSImageView"), "imageViewWithImage:", img);
            msg_rect(iv, "setFrame:", Rect { x: 8.0, y: 6.0, w: 20.0, h: 20.0 });
            msg_id(cell, "addSubview:", iv);
        }
        label(cell, &tr(name), Rect { x: 36.0, y: 7.0, w: SIDEBAR_W - 16.0 - 44.0, h: 18.0 }, 13.0, false, false);
        cell
    }
}

extern "C" fn table_selected(_this: Id, _cmd: Sel, _note: Id) {
    unsafe {
        let t = TABLE.load(Ordering::SeqCst);
        if t.is_null() {
            return;
        }
        let row = msg_ret_int(t, "selectedRow");
        if row >= 0 {
            show_page(row as usize);
        }
    }
}

extern "C" fn on_win_toggle(_this: Id, _cmd: Sel, sender: Id) {
    unsafe {
        let on = flip(tag_of(sender));
        msg_int(sender, "setState:", on as isize); // giữ công tắc khớp kết quả thật (vd. Login Items thất bại)
        if tag_of(sender) == T_PAUSE {
            show_page(SELECTED.load(Ordering::SeqCst)); // cập nhật dòng trạng thái
        }
    }
}

extern "C" fn on_pop_speed(_this: Id, _cmd: Sel, sender: Id) {
    set_speed(unsafe { msg_ret_int(sender, "indexOfSelectedItem") } as usize);
}

extern "C" fn on_pop_orient(_this: Id, _cmd: Sel, sender: Id) {
    set_orientation(unsafe { msg_ret_int(sender, "indexOfSelectedItem") } as usize);
}

extern "C" fn on_pop_scale(_this: Id, _cmd: Sel, sender: Id) {
    set_scale(unsafe { msg_ret_int(sender, "indexOfSelectedItem") } as usize);
    // Đổi cỡ làm thay đổi kích thước màn hình đích: vẽ lại trang để danh sách màn hình khớp.
    unsafe { show_page(SELECTED.load(Ordering::SeqCst)) };
}

extern "C" fn on_win_display(_this: Id, _cmd: Sel, sender: Id) {
    choose_display(unsafe { msg_ret_int(sender, "indexOfSelectedItem") });
}

extern "C" fn on_open_mail(_this: Id, _cmd: Sel, _sender: Id) {
    open_url(&format!("mailto:{CONTACT_EMAIL}"));
}

extern "C" fn on_win_close(_this: Id, _cmd: Sel, _sender: Id) {
    unsafe {
        let w = WINDOW.swap(ptr::null_mut(), Ordering::SeqCst);
        DETAIL.store(ptr::null_mut(), Ordering::SeqCst);
        TABLE.store(ptr::null_mut(), Ordering::SeqCst);
        if !w.is_null() {
            msg(w, "close");
        }
    }
}

extern "C" fn on_open_settings(_this: Id, _cmd: Sel, _sender: Id) {
    open_settings();
}

/// Mở cửa sổ cài đặt (dựng lại mỗi lần để phản ánh trạng thái và ngôn ngữ hiện tại).
pub fn open_settings() {
    refresh_lang();
    unsafe {
        let target = TARGET.load(Ordering::SeqCst);
        if target.is_null() {
            return;
        }
        on_win_close(NIL, ptr::null(), NIL);

        let win = msg(class("NSWindow"), "alloc");
        let f: extern "C" fn(Id, Sel, Rect, usize, usize, bool) -> Id = transmute(imp());
        let win = f(
            win,
            sel("initWithContentRect:styleMask:backing:defer:"),
            Rect { x: 0.0, y: 0.0, w: WIN_W, h: WIN_H },
            1 | 2 | (1 << 15), // titled | closable | fullSizeContentView
            2,                 // buffered
            false,
        );
        msg_bool(win, "setReleasedWhenClosed:", false);
        msg_id(win, "setTitle:", ns(&tr("{app} – Cài đặt")));
        msg_bool(win, "setTitlebarAppearsTransparent:", true);
        msg_int(win, "setTitleVisibility:", 1); // ẩn chữ tiêu đề, giữ 3 nút cửa sổ
        let content = new_content(WIN_W, WIN_H);
        msg_id(win, "setContentView:", content);

        // Vùng chi tiết (nền cửa sổ mặc định)
        let detail = msg_rect(msg(class("NSScrollView"), "alloc"), "initWithFrame:", Rect { x: SIDEBAR_W, y: 0.0, w: WIN_W - SIDEBAR_W, h: WIN_H });
        msg_bool(detail, "setHasVerticalScroller:", true);
        msg_bool(detail, "setAutohidesScrollers:", true); // trang ngắn thì không có thanh cuộn
        msg_bool(detail, "setDrawsBackground:", false);
        // Không để AppKit tự đệm theo titlebar: nếu có, trang ngắn vẫn "dư" ~28pt và cuộn được/hiện thanh cuộn.
        msg_bool(detail, "setAutomaticallyAdjustsContentInsets:", false);
        msg_insets(detail, "setContentInsets:", Insets { top: 0.0, left: 0.0, bottom: 0.0, right: 0.0 });
        // Kiểu thanh cuộn (luôn hiện / khi cuộn) giữ mặc định: theo cài đặt "Show scroll bars" của macOS.
        msg_id(content, "addSubview:", detail);
        DETAIL.store(detail, Ordering::SeqCst);

        // Sidebar: vibrancy kiểu source list
        let side = msg_rect(msg(class("NSVisualEffectView"), "alloc"), "initWithFrame:", Rect { x: 0.0, y: 0.0, w: SIDEBAR_W, h: WIN_H });
        msg_int(side, "setMaterial:", 7); // NSVisualEffectMaterialSidebar
        msg_int(side, "setBlendingMode:", 0); // behind window
        msg_int(side, "setState:", 1); // active
        msg_id(content, "addSubview:", side);

        let scroll = msg_rect(msg(class("NSScrollView"), "alloc"), "initWithFrame:", Rect { x: 0.0, y: 0.0, w: SIDEBAR_W, h: WIN_H - 52.0 });
        msg_bool(scroll, "setDrawsBackground:", false);
        msg_bool(scroll, "setHasVerticalScroller:", false);
        let table = msg_rect(msg(class("NSTableView"), "alloc"), "initWithFrame:", Rect { x: 0.0, y: 0.0, w: SIDEBAR_W, h: WIN_H - 52.0 });
        let col = msg_id(msg(class("NSTableColumn"), "alloc"), "initWithIdentifier:", ns("page"));
        msg_f64(col, "setWidth:", SIDEBAR_W - 16.0);
        msg_id(table, "addTableColumn:", col);
        msg_id(table, "setHeaderView:", NIL);
        msg_int(table, "setStyle:", 3); // NSTableViewStyleSourceList
        msg_f64(table, "setRowHeight:", 32.0);
        msg_id(table, "setBackgroundColor:", color("clearColor"));
        msg_id(table, "setDataSource:", target);
        msg_id(table, "setDelegate:", target);
        msg_id(scroll, "setDocumentView:", table);
        msg_id(side, "addSubview:", scroll);
        TABLE.store(table, Ordering::SeqCst);

        show_page(0);
        let idx = msg_int(class("NSIndexSet"), "indexSetWithIndex:", 0);
        let g: extern "C" fn(Id, Sel, Id, bool) = transmute(imp());
        g(table, sel("selectRowIndexes:byExtendingSelection:"), idx, false);

        msg(win, "center");
        msg_bool(ns_app(), "activateIgnoringOtherApps:", true);
        msg_id(win, "makeKeyAndOrderFront:", NIL);
        msg(win, "retain");
        WINDOW.store(win, Ordering::SeqCst);
    }
}

unsafe fn new_content(w: f64, h: f64) -> Id {
    msg_rect(msg(class("NSView"), "alloc"), "initWithFrame:", Rect { x: 0.0, y: 0.0, w, h })
}

// ---------- lớp phủ hiệu chỉnh vị trí chạm ----------

static CALIB_WIN: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

/// Hiện/cập nhật lớp phủ trên màn hình đích: dấu + tại `target` (toạ độ chuẩn hoá, gốc trên-trái) và hướng dẫn.
/// `bounds` = (x, y, w, h) theo toạ độ Quartz toàn cục; `main_h` = chiều cao màn hình chính (đổi sang toạ độ Cocoa).
pub fn calib_show(bounds: (f64, f64, f64, f64), main_h: f64, step: usize, total: usize, target: (f64, f64)) {
    unsafe {
        let (bx, by, bw, bh) = bounds;
        let mut win = CALIB_WIN.load(Ordering::SeqCst);
        if win.is_null() {
            let f: extern "C" fn(Id, Sel, Rect, usize, usize, bool) -> Id = transmute(imp());
            win = f(
                msg(class("NSWindow"), "alloc"),
                sel("initWithContentRect:styleMask:backing:defer:"),
                Rect { x: bx, y: main_h - (by + bh), w: bw, h: bh },
                0, // borderless
                2,
                false,
            );
            msg_bool(win, "setReleasedWhenClosed:", false);
            msg_int(win, "setLevel:", 1000);
            msg_bool(win, "setOpaque:", false);
            msg_id(win, "setBackgroundColor:", msg_f64(color("blackColor"), "colorWithAlphaComponent:", 0.88));
            msg_bool(win, "setIgnoresMouseEvents:", true);
            msg(win, "retain");
            CALIB_WIN.store(win, Ordering::SeqCst);
        }
        msg_id(win, "setBackgroundColor:", msg_f64(color("blackColor"), "colorWithAlphaComponent:", 0.88));
        let content = msg(win, "contentView");
        msg_id(content, "setSubviews:", msg(class("NSArray"), "array"));
        let white = color("whiteColor");
        let mk = |text: &str, r: Rect, size: f64, bold: bool| {
            let l = msg_id(class("NSTextField"), "labelWithString:", ns(text));
            msg_rect(l, "setFrame:", r);
            msg_id(l, "setFont:", font(size, bold));
            msg_id(l, "setTextColor:", white);
            msg_int(l, "setAlignment:", TEXT_CENTER);
            msg_id(content, "addSubview:", l);
        };
        // Dấu +: tâm tại target (trục y Cocoa hướng lên nên đảo lại)
        let (cx, cy) = (target.0 * bw, (1.0 - target.1) * bh);
        mk("+", Rect { x: cx - 50.0, y: cy - 50.0, w: 100.0, h: 100.0 }, 96.0, false);
        mk(&tr("Chạm vào dấu +"), Rect { x: 0.0, y: bh / 2.0 + 30.0, w: bw, h: 36.0 }, 28.0, true);
        mk(
            &tr("Bước {n}/{t} · chạm 2 ngón để hủy · không chạm được thì đợi 8 giây để bỏ qua")
                .replace("{n}", &(step + 1).to_string())
                .replace("{t}", &total.to_string()),
            Rect { x: 0.0, y: bh / 2.0 - 10.0, w: bw, h: 24.0 },
            16.0,
            false,
        );
        msg(win, "orderFrontRegardless");
    }
}

/// Thông báo kết quả trên lớp phủ (không dấu +), căn giữa màn hình đích.
pub fn calib_message(bounds: (f64, f64, f64, f64), main_h: f64, text: &str) {
    calib_show(bounds, main_h, 0, 0, (-1.0, -1.0));
    unsafe {
        let win = CALIB_WIN.load(Ordering::SeqCst);
        if win.is_null() {
            return;
        }
        let content = msg(win, "contentView");
        msg_id(content, "setSubviews:", msg(class("NSArray"), "array"));
        let l = msg_id(class("NSTextField"), "wrappingLabelWithString:", ns(text));
        let (bw, bh) = (bounds.2, bounds.3);
        msg_rect(l, "setFrame:", Rect { x: bw * 0.2, y: bh / 2.0 - 40.0, w: bw * 0.6, h: 80.0 });
        msg_id(l, "setFont:", font(28.0, true));
        msg_id(l, "setTextColor:", color("whiteColor"));
        msg_int(l, "setAlignment:", TEXT_CENTER);
        msg_id(content, "addSubview:", l);
    }
}

pub fn calib_close() {
    unsafe {
        let win = CALIB_WIN.swap(ptr::null_mut(), Ordering::SeqCst);
        if !win.is_null() {
            msg(win, "close");
        }
    }
}

extern "C" fn on_start_calib(_this: Id, _cmd: Sel, _sender: Id) {
    with_app(|a| a.start_calibration());
}

extern "C" fn on_reset_calib(_this: Id, _cmd: Sel, _sender: Id) {
    with_app(|a| a.reset_calibration());
    save_prefs();
    unsafe { show_page(SELECTED.load(Ordering::SeqCst)) };
}

// ---------- lớp phủ kiểm tra (vùng chạm, màu màn hình) ----------
// Dùng chung cửa sổ với lớp phủ hiệu chỉnh (CALIB_WIN).

static PROBE_IMG: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static PROBE_INFO: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> *const c_void;
    fn CGColorSpaceRelease(s: *const c_void);
    fn CGBitmapContextCreate(data: *mut c_void, w: usize, h: usize, bpc: usize, bpr: usize, space: *const c_void, info: u32) -> *mut c_void;
    fn CGBitmapContextCreateImage(ctx: *mut c_void) -> *const c_void;
    fn CGContextRelease(ctx: *mut c_void);
    fn CGImageRelease(img: *const c_void);
}

unsafe fn msg_rgba(o: Id, s: &str, r: f64, g: f64, b: f64, a: f64) -> Id {
    let f: extern "C" fn(Id, Sel, f64, f64, f64, f64) -> Id = transmute(imp());
    f(o, sel(s), r, g, b, a)
}

unsafe fn overlay_win(bounds: (f64, f64, f64, f64), main_h: f64) -> Id {
    calib_show(bounds, main_h, 0, 0, (-1.0, -1.0));
    let win = CALIB_WIN.load(Ordering::SeqCst);
    if !win.is_null() {
        let content = msg(win, "contentView");
        msg_id(content, "setSubviews:", msg(class("NSArray"), "array"));
    }
    PROBE_IMG.store(ptr::null_mut(), Ordering::SeqCst);
    PROBE_INFO.store(ptr::null_mut(), Ordering::SeqCst);
    win
}

/// Thông báo chữ lớn giữa lớp phủ.
pub fn overlay_message(bounds: (f64, f64, f64, f64), main_h: f64, text: &str) {
    calib_message(bounds, main_h, text);
}

/// Toàn màn hình một màu đặc (kiểm tra điểm ảnh chết).
pub fn overlay_color(bounds: (f64, f64, f64, f64), main_h: f64, r: u8, g: u8, b: u8) {
    unsafe {
        let win = overlay_win(bounds, main_h);
        if win.is_null() {
            return;
        }
        msg_id(win, "setBackgroundColor:", msg_rgba(class("NSColor"), "colorWithSRGBRed:green:blue:alpha:", r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0));
    }
}

/// Giao diện "Kiểm tra vùng chạm": ảnh bản đồ phủ kín, tiêu đề và dòng thông tin ở trên.
pub fn overlay_touch_ui(bounds: (f64, f64, f64, f64), main_h: f64) {
    unsafe {
        let win = overlay_win(bounds, main_h);
        if win.is_null() {
            return;
        }
        let (bw, bh) = (bounds.2, bounds.3);
        msg_id(win, "setBackgroundColor:", msg_rgba(class("NSColor"), "colorWithSRGBRed:green:blue:alpha:", 28.0 / 255.0, 28.0 / 255.0, 30.0 / 255.0, 1.0));
        let content = msg(win, "contentView");
        let iv = msg_rect(msg(class("NSImageView"), "alloc"), "initWithFrame:", Rect { x: 0.0, y: 0.0, w: bw, h: bh });
        msg_int(iv, "setImageScaling:", 1); // NSImageScaleAxesIndependently: kéo giãn phủ kín
        msg_id(content, "addSubview:", iv);
        PROBE_IMG.store(iv, Ordering::SeqCst);
        // Nền tối bán trong suốt phía sau chữ để đọc được trên vệt vàng.
        let bar = box_view(Rect { x: 0.0, y: bh - 180.0, w: bw, h: 180.0 }, 4);
        msg_int(bar, "setTitlePosition:", 0);
        msg_f64(bar, "setBorderWidth:", 0.0);
        msg_id(bar, "setFillColor:", msg_f64(color("blackColor"), "colorWithAlphaComponent:", 0.78));
        msg_id(content, "addSubview:", bar);
        let mk = |text: &str, y: f64, size: f64, bold: bool, col: Id| -> Id {
            let l = msg_id(class("NSTextField"), "labelWithString:", ns(text));
            msg_rect(l, "setFrame:", Rect { x: 0.0, y, w: bw, h: size + 8.0 });
            msg_id(l, "setFont:", font(size, bold));
            msg_id(l, "setTextColor:", col);
            msg_int(l, "setAlignment:", TEXT_CENTER);
            msg_id(content, "addSubview:", l);
            l
        };
        mk(&tr("Rê ngón tay khắp màn hình, kể cả sát bốn mép"), bh - 80.0, 28.0, true, color("whiteColor"));
        mk(&tr("Vàng = nhận chạm · chạm 4 ngón để kết thúc"), bh - 116.0, 16.0, false, color("secondaryLabelColor"));
        let info = mk("", bh - 150.0, 16.0, true, msg_rgba(class("NSColor"), "colorWithSRGBRed:green:blue:alpha:", 1.0, 0.84, 0.04, 1.0));
        msg(info, "retain");
        PROBE_INFO.store(info, Ordering::SeqCst);
    }
}

/// Cập nhật bản đồ (ảnh RGBA w×h, hàng đầu = mép trên) và dòng thông tin.
pub fn overlay_touch_update(w: usize, h: usize, rgba: &[u8], info: &str) {
    unsafe {
        let iv = PROBE_IMG.load(Ordering::SeqCst);
        if iv.is_null() || rgba.len() < w * h * 4 {
            return;
        }
        let space = CGColorSpaceCreateDeviceRGB();
        let mut buf = rgba.to_vec();
        let ctx = CGBitmapContextCreate(buf.as_mut_ptr() as *mut c_void, w, h, 8, w * 4, space, 1); // RGBA premultiplied last
        if !ctx.is_null() {
            let img = CGBitmapContextCreateImage(ctx);
            if !img.is_null() {
                let f: extern "C" fn(Id, Sel, *const c_void, Size) -> Id = transmute(imp());
                let ns_img = f(msg(class("NSImage"), "alloc"), sel("initWithCGImage:size:"), img, Size { w: 0.0, h: 0.0 });
                msg_id(iv, "setImage:", ns_img);
                CGImageRelease(img);
            }
            CGContextRelease(ctx);
        }
        CGColorSpaceRelease(space);
        let l = PROBE_INFO.load(Ordering::SeqCst);
        if !l.is_null() {
            msg_id(l, "setStringValue:", ns(info));
        }
    }
}

extern "C" fn on_start_touch_test(_this: Id, _cmd: Sel, _sender: Id) {
    with_app(|a| a.start_touch_test());
}

extern "C" fn on_start_screen_test(_this: Id, _cmd: Sel, _sender: Id) {
    with_app(|a| a.start_screen_test());
}

unsafe fn target_class() -> Id {
    let existing = class("TouchRelayMenuTarget");
    if !existing.is_null() {
        return existing;
    }
    let name = CString::new("TouchRelayMenuTarget").unwrap();
    let cls = objc_allocateClassPair(class("NSObject"), name.as_ptr(), 0);
    let types = CString::new("v@:@").unwrap();
    let handlers: [(&str, extern "C" fn(Id, Sel, Id)); 24] = [
        ("toggleItem:", on_toggle),
        ("setSpeed:", on_set_speed),
        ("setOrientation:", on_set_orientation),
        ("setScale:", on_set_scale),
        ("setDisplay:", on_set_display),
        ("menuNeedsUpdate:", on_menu_update),
        ("showHelp:", on_help),
        ("showAbout:", on_about),
        ("troubleshoot:", on_troubleshoot),
        ("openOptions:", on_open_options),
        ("openLog:", on_open_log),
        ("openSettings:", on_open_settings),
        ("winToggle:", on_win_toggle),
        ("popSpeed:", on_pop_speed),
        ("popOrient:", on_pop_orient),
        ("popScale:", on_pop_scale),
        ("openMail:", on_open_mail),
        ("startCalib:", on_start_calib),
        ("resetCalib:", on_reset_calib),
        ("startTouchTest:", on_start_touch_test),
        ("startScreenTest:", on_start_screen_test),
        ("winDisplay:", on_win_display),
        ("winClose:", on_win_close),
        ("quit:", on_quit),
    ];
    for (s, f) in handlers {
        class_addMethod(cls, sel(s), f as *const c_void, types.as_ptr());
    }
    // Nguồn dữ liệu / delegate của sidebar (kiểu trả về/tham số khác "v@:@").
    let t_rows = CString::new("q@:@").unwrap();
    class_addMethod(cls, sel("numberOfRowsInTableView:"), table_rows as *const c_void, t_rows.as_ptr());
    let t_view = CString::new("@@:@@q").unwrap();
    class_addMethod(cls, sel("tableView:viewForTableColumn:row:"), table_view as *const c_void, t_view.as_ptr());
    class_addMethod(cls, sel("tableViewSelectionDidChange:"), table_selected as *const c_void, types.as_ptr());
    objc_registerClassPair(cls);
    cls
}

unsafe fn add_item(menu: Id, target: Id, title: &str, action: Option<&str>, key: &str, state: Option<bool>) -> Id {
    let item = msg(class("NSMenuItem"), "alloc");
    let f: extern "C" fn(Id, Sel, Id, Sel, Id) -> Id = transmute(imp());
    let act = action.map(sel).unwrap_or(ptr::null());
    let item = f(
        item,
        sel("initWithTitle:action:keyEquivalent:"),
        ns(title),
        act,
        ns(key),
    );
    if action.is_some() {
        msg_id(item, "setTarget:", target);
    }
    if let Some(on) = state {
        msg_int(item, "setState:", on as isize);
    }
    msg_id(menu, "addItem:", item);
    item
}

unsafe fn separator(menu: Id) {
    msg_id(menu, "addItem:", msg(class("NSMenuItem"), "separatorItem"));
}

unsafe fn new_menu(title: &str) -> Id {
    let m = msg(class("NSMenu"), "alloc");
    msg_id(m, "initWithTitle:", ns(title))
}

/// Gắn submenu vào một item mới của `menu`.
unsafe fn add_submenu(menu: Id, target: Id, title: &str) -> Id {
    let item = add_item(menu, target, title, None, "", None);
    let sub = new_menu(title);
    msg_id(item, "setSubmenu:", sub);
    sub
}

unsafe fn radio_menu(menu: Id, target: Id, title: &str, labels: &[&str], action: &str, slot: &AtomicPtr<c_void>) {
    let sub = add_submenu(menu, target, &tr(title));
    for (i, label) in labels.iter().enumerate() {
        let it = add_item(sub, target, &tr(label), Some(action), "", Some(false));
        msg_int(it, "setTag:", i as isize);
    }
    slot.store(sub, Ordering::SeqCst);
}

unsafe fn toggle_item(menu: Id, target: Id, tag: isize, title: &str, key: &str) {
    let it = add_item(menu, target, &tr(title), Some("toggleItem:"), key, Some(false));
    msg_int(it, "setTag:", tag);
    TOGGLE_ITEMS[tag as usize].store(it, Ordering::SeqCst);
}

/// Dựng nội dung menu chính (gọi lúc khởi tạo và khi ngôn ngữ hệ thống đổi).
unsafe fn build_menu(menu: Id, target: Id) {
    let status = add_item(menu, target, &tr("●  Đang hoạt động"), None, "", None);
    msg(status, "retain");
    STATUS_ITEM.store(status, Ordering::SeqCst);
    let dev = add_item(menu, target, "", None, "", None);
    msg(dev, "retain");
    DEVICE_ITEM.store(dev, Ordering::SeqCst);
    set_devices(&[]);
    separator(menu);
    // Việc hay dùng ở cấp trên cùng.
    toggle_item(menu, target, T_PAUSE, "Tạm dừng", "p");
    toggle_item(menu, target, T_SNAP, "Chạm thông minh", "");
    toggle_item(menu, target, T_HIDE, "Ẩn con trỏ khi chạm", "");
    separator(menu);

    // Nhóm cài đặt dùng thường xuyên, gom vào submenu.
    let speed_labels: Vec<&str> = SPEEDS.iter().map(|s| s.0).collect();
    radio_menu(menu, target, "Tốc độ cuộn", &speed_labels, "setSpeed:", &SPEED_MENU);
    let sub = add_submenu(menu, target, &tr("Cuộn & cử chỉ"));
    toggle_item(sub, target, T_INVERT, "Đảo chiều cuộn", "");
    toggle_item(sub, target, T_MOMENTUM, "Cuộn quán tính", "");
    toggle_item(sub, target, T_PINCH, "Chụm / xoè để phóng to", "");
    toggle_item(sub, target, T_SWIPE, "Vuốt 3 ngón", "");
    toggle_item(sub, target, T_ONEFINGER, "Kéo 1 ngón để cuộn (kiểu iPad)", "");
    let sub = add_submenu(menu, target, &tr("Màn hình"));
    let disp = add_submenu(sub, target, &tr("Màn hình đích"));
    DISPLAY_MENU.store(disp, Ordering::SeqCst);
    radio_menu(sub, target, "Hướng màn hình", &ORIENTS, "setOrientation:", &ORIENT_MENU);
    radio_menu(sub, target, "Cỡ giao diện", &SCALES, "setScale:", &SCALE_MENU);

    // Công cụ: hiệu chỉnh, kiểm tra, trợ giúp, nâng cao.
    let tools = add_submenu(menu, target, &tr("Công cụ"));
    add_item(tools, target, &tr("Hiệu chỉnh vị trí chạm…"), Some("startCalib:"), "", None);
    add_item(tools, target, &tr("Kiểm tra vùng chạm…"), Some("startTouchTest:"), "", None);
    add_item(tools, target, &tr("Kiểm tra màn hình…"), Some("startScreenTest:"), "", None);
    separator(tools);
    add_item(tools, target, &tr("Hướng dẫn cử chỉ…"), Some("showHelp:"), "", None);
    add_item(tools, target, &tr("Khắc phục sự cố…"), Some("troubleshoot:"), "", None);
    separator(tools);
    add_item(tools, target, &tr("Sửa tuỳ chọn dòng lệnh…"), Some("openOptions:"), "", None);
    add_item(tools, target, &tr("Xem nhật ký"), Some("openLog:"), "", None);
    separator(menu);

    add_item(menu, target, &tr("Cài đặt…"), Some("openSettings:"), ",", None);
    if !class("SMAppService").is_null() {
        toggle_item(menu, target, T_LOGIN, "Mở cùng macOS", "");
    }
    add_item(menu, target, &tr("Giới thiệu {app}…"), Some("showAbout:"), "", None);
    separator(menu);
    add_item(menu, target, &tr("Thoát {app}"), Some("quit:"), "q", None);
}

/// Tạo icon trên thanh menu.
pub fn install(_prefs: &crate::Prefs) {
    refresh_lang();
    unsafe {
        let target = msg(msg(target_class(), "alloc"), "init");
        TARGET.store(target, Ordering::SeqCst);
        let menu = new_menu(APP_NAME);
        msg_bool(menu, "setAutoenablesItems:", false);
        msg_id(menu, "setDelegate:", target); // menuNeedsUpdate: đồng bộ ngôn ngữ + dấu chọn mỗi lần mở
        build_menu(menu, target);
        sync_menu();

        let bar = msg(class("NSStatusBar"), "systemStatusBar");
        let item = msg_f64(bar, "statusItemWithLength:", -1.0); // NSVariableStatusItemLength
        msg(item, "retain");
        let button = msg(item, "button");
        let img = msg_id2(
            class("NSImage"),
            "imageWithSystemSymbolName:accessibilityDescription:",
            ns("hand.point.up.left"),
            ns(APP_NAME),
        );
        if !button.is_null() {
            STATUS_BUTTON.store(button, Ordering::SeqCst);
            if !img.is_null() {
                msg_bool(img, "setTemplate:", true);
                msg_id(button, "setImage:", img);
            } else {
                msg_id(button, "setTitle:", ns("☝︎"));
            }
        }
        msg_id(item, "setMenu:", menu);
    }
}
