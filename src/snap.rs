//! "Chạm thông minh": khi chạm gần một nút nhỏ nhưng không trúng, hút vào điều khiển gần nhất.
//! Dùng Accessibility API (AXUIElementCopyElementAtPosition) nên cần quyền Accessibility (đã cần sẵn).
//! Chỉ áp dụng cho tap (click trái), không áp dụng cho kéo/cuộn.

use crate::gesture::{Action, Button, P};
use std::sync::atomic::{AtomicBool, Ordering};

/// Kéo 1 ngón thông minh đang bật? (đọc trong closure của engine, đổi được lúc chạy)
pub static SMART_DRAG: AtomicBool = AtomicBool::new(true);

pub fn set_smart_drag(on: bool) {
    SMART_DRAG.store(on, Ordering::SeqCst);
}

pub fn smart_drag_enabled() -> bool {
    SMART_DRAG.load(Ordering::SeqCst)
}

/// Vai trò (AXRole) của các điều khiển bấm được.
pub const ACTIONABLE: [&str; 11] = [
    "AXButton",
    "AXCheckBox",
    "AXRadioButton",
    "AXPopUpButton",
    "AXMenuButton",
    "AXMenuItem",
    "AXMenuBarItem",
    "AXLink",
    "AXDisclosureTriangle",
    "AXComboBox",
    "AXIncrementor",
];

pub fn is_actionable(role: &str) -> bool {
    ACTIONABLE.contains(&role)
}

/// Vai trò của ô nhập văn bản (kể cả ô mật khẩu: AXTextField + subrole AXSecureTextField).
pub const TEXT_ROLES: [&str; 4] = ["AXTextField", "AXTextArea", "AXSearchField", "AXComboBox"];

pub fn is_text_role(role: &str) -> bool {
    TEXT_ROLES.contains(&role)
}

/// Phần tử đang focus: có phải ô nhập không, và khung của nó (toạ độ toàn cục, gốc trên-trái).
pub struct Focus {
    pub is_text: bool,
    pub frame: Option<(P, P)>,
    /// Con trỏ đang ở đầu câu (ô trống, sau xuống dòng, hoặc sau ". " "? " "! "); None nếu không đọc được.
    pub at_sentence_start: Option<bool>,
}

/// Văn bản trước con trỏ có kết thúc một câu không (để tự viết hoa chữ kế tiếp).
pub fn is_sentence_start(before: &str) -> bool {
    let t = before.trim_end_matches([' ', '\t']);
    if t.is_empty() || t.ends_with('\n') {
        return true;
    }
    // Phải có ít nhất một khoảng trắng sau dấu kết câu ("Hi." + chữ thì chưa phải câu mới).
    t.len() < before.len() && t.ends_with(['.', '?', '!'])
}

/// Vai trò mà kéo 1 ngón nên là KÉO CHUỘT (thanh trượt, thanh cuộn, đường chia, ô nhập 1 dòng...).
pub const DRAG_ROLES: [&str; 8] = [
    "AXSlider",
    "AXScrollBar",
    "AXValueIndicator",
    "AXSplitter",
    "AXHandle",
    "AXTextField",
    "AXDockItem",
    "AXColorWell",
];

/// Vai trò chứa nội dung cuộn được: kéo 1 ngón nên là CUỘN.
pub const SCROLL_ROLES: [&str; 8] = [
    "AXScrollArea",
    "AXWebArea",
    "AXTable",
    "AXOutline",
    "AXList",
    "AXBrowser",
    "AXTextArea",
    "AXCollection",
];

/// Phân loại ý định từ chuỗi vai trò, từ phần tử trong cùng ra ngoài (không kể AXApplication):
/// `Some(true)` = kéo chuột, `Some(false)` = cuộn, `None` = không biết (giữ mặc định).
/// Gặp vai trò kéo trước → kéo; gặp vùng cuộn trước → cuộn; hết chuỗi mà không có gì cuộn được → kéo
/// (cuộn ở đó vô ích, ví dụ thanh tiêu đề cửa sổ).
pub fn classify_drag_intent(chain: &[&str]) -> Option<bool> {
    let chain: Vec<&&str> = chain.iter().filter(|r| **r != "AXApplication").collect();
    if chain.is_empty() {
        return None;
    }
    for r in chain {
        if DRAG_ROLES.contains(r) {
            return Some(true);
        }
        if SCROLL_ROLES.contains(r) {
            return Some(false);
        }
    }
    Some(true)
}

/// Các điểm dò quanh `p`: vòng bán kính tăng dần, mỗi vòng 8 hướng.
pub fn probe_points(p: P, radius: f64) -> Vec<(f64, P)> {
    let mut out = vec![];
    let mut r = 5.0;
    while r <= radius + 0.01 {
        for i in 0..8 {
            let a = i as f64 * std::f64::consts::FRAC_PI_4;
            out.push((r, P::new(p.x + r * a.cos(), p.y + r * a.sin())));
        }
        r += 5.0;
    }
    out
}

/// Điểm sẽ click: nút nhỏ (<40pt) thì click vào tâm, nút lớn thì giữ điểm dò.
pub fn click_point(probe: P, frame: Option<(P, P)>) -> P {
    match frame {
        Some((pos, size)) if size.x < 40.0 && size.y < 40.0 && size.x > 0.0 && size.y > 0.0 => {
            P::new(pos.x + size.x / 2.0, pos.y + size.y / 2.0)
        }
        _ => probe,
    }
}

/// Tìm trong `acts` chuỗi click trái do tap sinh ra (MoveTo p → MouseDown trái p → MouseUp trái p),
/// trả về chỉ số MoveTo và điểm p.
pub fn find_tap_clicks(acts: &[Action]) -> Vec<(usize, P)> {
    let mut v = vec![];
    for i in 0..acts.len().saturating_sub(2) {
        if let (
            Action::MoveTo(p),
            Action::MouseDown { button: Button::Left, at: a, .. },
            Action::MouseUp { button: Button::Left, at: b, .. },
        ) = (&acts[i], &acts[i + 1], &acts[i + 2])
        {
            if p.dist(*a) < 0.01 && p.dist(*b) < 0.01 {
                v.push((i, *p));
            }
        }
    }
    v
}

/// Đổi vị trí của chuỗi click bắt đầu tại `i` sang `q`.
pub fn retarget(acts: &mut [Action], i: usize, q: P) {
    if let Action::MoveTo(p) = &mut acts[i] {
        *p = q;
    }
    if let Action::MouseDown { at, .. } = &mut acts[i + 1] {
        *at = q;
    }
    if let Action::MouseUp { at, .. } = &mut acts[i + 2] {
        *at = q;
    }
}

#[cfg(target_os = "macos")]
pub use mac::{drag_intent, focus_info, snap_point};

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use crate::ffi::*;
    use std::ffi::c_void;
    use std::ptr;

    type AxRef = *const c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXUIElementCreateSystemWide() -> AxRef;
        fn AXUIElementCopyElementAtPosition(app: AxRef, x: f32, y: f32, el: *mut AxRef) -> i32;
        fn AXUIElementCopyAttributeValue(el: AxRef, attr: CFStringRef, value: *mut CFTypeRef) -> i32;
        fn AXUIElementSetMessagingTimeout(el: AxRef, seconds: f32) -> i32;
        fn AXUIElementCreateApplication(pid: i32) -> AxRef;
        fn AXUIElementSetAttributeValue(el: AxRef, attr: CFStringRef, value: CFTypeRef) -> i32;
        fn AXValueGetValue(v: CFTypeRef, kind: u32, out: *mut c_void) -> bool;
    }

    const K_AX_CGPOINT: u32 = 1;
    const K_AX_CGSIZE: u32 = 2;

    unsafe fn attr(el: AxRef, name: &str) -> CFTypeRef {
        let k = CfStr::new(name);
        let mut v: CFTypeRef = ptr::null();
        if AXUIElementCopyAttributeValue(el, k.0, &mut v) != 0 {
            return ptr::null();
        }
        v
    }

    unsafe fn role_of(el: AxRef) -> Option<String> {
        let v = attr(el, "AXRole");
        if v.is_null() {
            return None;
        }
        let mut buf = [0 as std::ffi::c_char; 64];
        let ok = CFGetTypeID(v) == CFStringGetTypeID()
            && CFStringGetCString(v, buf.as_mut_ptr(), buf.len() as CFIndex, kCFStringEncodingUTF8) != 0;
        CFRelease(v);
        ok.then(|| std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
    }

    /// Văn bản đứng trước con trỏ (AXValue cắt theo AXSelectedTextRange, đơn vị UTF-16).
    unsafe fn text_before_caret(el: AxRef) -> Option<String> {
        let (v, r) = (attr(el, "AXValue"), attr(el, "AXSelectedTextRange"));
        let mut out = None;
        if !v.is_null() && !r.is_null() && CFGetTypeID(v) == CFStringGetTypeID() {
            let mut range = [0i64; 2]; // CFRange {location, length}
            if AXValueGetValue(r, 4, range.as_mut_ptr() as *mut c_void) {
                let mut buf = vec![0 as std::ffi::c_char; 8192];
                if CFStringGetCString(v, buf.as_mut_ptr(), buf.len() as CFIndex, kCFStringEncodingUTF8) != 0 {
                    let full = std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned();
                    let u: Vec<u16> = full.encode_utf16().collect();
                    let loc = (range[0].max(0) as usize).min(u.len());
                    out = Some(String::from_utf16_lossy(&u[..loc]));
                }
            }
        }
        if !v.is_null() {
            CFRelease(v);
        }
        if !r.is_null() {
            CFRelease(r);
        }
        out
    }

    unsafe fn frame_of(el: AxRef) -> Option<(P, P)> {
        let (pv, sv) = (attr(el, "AXPosition"), attr(el, "AXSize"));
        let mut pos = CGPoint { x: 0.0, y: 0.0 };
        let mut size = [0.0f64; 2];
        let ok = !pv.is_null()
            && !sv.is_null()
            && AXValueGetValue(pv, K_AX_CGPOINT, &mut pos as *mut CGPoint as *mut c_void)
            && AXValueGetValue(sv, K_AX_CGSIZE, size.as_mut_ptr() as *mut c_void);
        if !pv.is_null() {
            CFRelease(pv);
        }
        if !sv.is_null() {
            CFRelease(sv);
        }
        ok.then(|| (P::new(pos.x, pos.y), P::new(size[0], size[1])))
    }

    /// Điều khiển bấm được tại điểm `p`: (nó có bấm được không, khung của nó).
    unsafe fn probe(sys: AxRef, p: P) -> Option<(bool, Option<(P, P)>)> {
        let mut el: AxRef = ptr::null();
        if AXUIElementCopyElementAtPosition(sys, p.x as f32, p.y as f32, &mut el) != 0 || el.is_null() {
            return None;
        }
        let role = role_of(el);
        let actionable = role.as_deref().is_some_and(is_actionable);
        let frame = if actionable { frame_of(el) } else { None };
        CFRelease(el);
        Some((actionable, frame))
    }

    /// Dò vai trò từ phần tử dưới `p` lên tới ứng dụng, rồi phân loại kéo/cuộn (xem `classify_drag_intent`).
    pub fn drag_intent(p: P) -> Option<bool> {
        unsafe {
            if !AXIsProcessTrusted() {
                return None;
            }
            let sys = AXUIElementCreateSystemWide();
            if sys.is_null() {
                return None;
            }
            AXUIElementSetMessagingTimeout(sys, 0.05);
            let mut el: AxRef = ptr::null();
            let ok = AXUIElementCopyElementAtPosition(sys, p.x as f32, p.y as f32, &mut el) == 0 && !el.is_null();
            CFRelease(sys);
            if !ok {
                return None;
            }
            let mut roles: Vec<String> = vec![];
            let mut cur = el;
            for _ in 0..12 {
                if let Some(r) = role_of(cur) {
                    roles.push(r);
                }
                let parent = attr(cur, "AXParent");
                CFRelease(cur);
                if parent.is_null() {
                    cur = ptr::null();
                    break;
                }
                cur = parent;
            }
            if !cur.is_null() {
                CFRelease(cur);
            }
            let refs: Vec<&str> = roles.iter().map(|s| s.as_str()).collect();
            classify_drag_intent(&refs)
        }
    }

    /// Pid đã được bật "AXManualAccessibility" (Chrome/Electron chỉ lộ cây Accessibility của trang web khi có yêu cầu này).
    static ENABLED_PIDS: std::sync::Mutex<Vec<i32>> = std::sync::Mutex::new(Vec::new());

    unsafe fn enable_manual_accessibility(pid: i32) {
        let Ok(mut list) = ENABLED_PIDS.lock() else { return };
        if list.contains(&pid) {
            return;
        }
        if list.len() > 64 {
            list.clear();
        }
        list.push(pid);
        let app = AXUIElementCreateApplication(pid);
        if !app.is_null() {
            AXUIElementSetMessagingTimeout(app, 0.05);
            let k = CfStr::new("AXManualAccessibility");
            let _ = AXUIElementSetAttributeValue(app, k.0, kCFBooleanTrue);
            CFRelease(app);
        }
    }

    /// Phần tử đang focus của app đang ở trên cùng (`frontmost_pid`). None nếu không dò được (chưa cấp quyền,
    /// app không lộ Accessibility, hoặc hết thời gian chờ). Không gọi cho pid của chính TouchRelay (tự treo).
    pub fn focus_info(frontmost_pid: Option<i32>) -> Option<Focus> {
        unsafe {
            if !AXIsProcessTrusted() {
                return None;
            }
            if let Some(pid) = frontmost_pid {
                enable_manual_accessibility(pid);
            }
            let sys = AXUIElementCreateSystemWide();
            if sys.is_null() {
                return None;
            }
            AXUIElementSetMessagingTimeout(sys, 0.08);
            let el = attr(sys, "AXFocusedUIElement");
            CFRelease(sys);
            if el.is_null() {
                return None;
            }
            let is_text = role_of(el).as_deref().is_some_and(is_text_role);
            let frame = if is_text { frame_of(el) } else { None };
            let at_sentence_start = if is_text { text_before_caret(el).map(|t| is_sentence_start(&t)) } else { None };
            CFRelease(el);
            Some(Focus { is_text, frame, at_sentence_start })
        }
    }

    /// Điểm nên click thay cho `p`: giữ nguyên nếu đã trúng điều khiển bấm được hoặc không thấy gì gần đó.
    pub fn snap_point(p: P, radius: f64) -> P {
        unsafe {
            if !AXIsProcessTrusted() {
                return p;
            }
            let sys = AXUIElementCreateSystemWide();
            if sys.is_null() {
                return p;
            }
            AXUIElementSetMessagingTimeout(sys, 0.05); // app treo không được làm đơ cảm ứng
            let result = (|| {
                if let Some((true, _)) = probe(sys, p) {
                    return p; // đã trúng nút
                }
                let mut best: Option<(f64, P)> = None;
                for (r, q) in probe_points(p, radius) {
                    if best.is_some_and(|(br, _)| r > br) {
                        break; // đã có ứng viên ở vòng gần hơn
                    }
                    if let Some((true, frame)) = probe(sys, q) {
                        if best.is_none() {
                            best = Some((r, click_point(q, frame)));
                        }
                    }
                }
                best.map(|b| b.1).unwrap_or(p)
            })();
            CFRelease(sys);
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sentence_start() {
        assert!(is_sentence_start(""));
        assert!(is_sentence_start("Hello.\n"));
        assert!(is_sentence_start("Hello. "));
        assert!(is_sentence_start("Really? "));
        assert!(!is_sentence_start("Hello."));
        assert!(!is_sentence_start("Hello"));
        assert!(!is_sentence_start("Hello, "));
    }

    #[test]
    fn drag_intent_classification() {
        // Thanh trượt trong trang web: kéo (vai trò kéo gặp trước vùng cuộn)
        assert_eq!(classify_drag_intent(&["AXSlider", "AXWebArea", "AXScrollArea", "AXWindow", "AXApplication"]), Some(true));
        // Liên kết trong trang web: cuộn
        assert_eq!(classify_drag_intent(&["AXLink", "AXWebArea", "AXScrollArea", "AXWindow"]), Some(false));
        // Dòng trong danh sách/bảng: cuộn
        assert_eq!(classify_drag_intent(&["AXStaticText", "AXCell", "AXRow", "AXOutline", "AXScrollArea"]), Some(false));
        // Thanh tiêu đề cửa sổ: không có gì cuộn được -> kéo cửa sổ
        assert_eq!(classify_drag_intent(&["AXStaticText", "AXWindow", "AXApplication"]), Some(true));
        // Thanh cuộn nằm trong vùng cuộn: kéo (không phải cuộn)
        assert_eq!(classify_drag_intent(&["AXValueIndicator", "AXScrollBar", "AXScrollArea"]), Some(true));
        // Ô nhập 1 dòng trong trang: kéo để chọn chữ
        assert_eq!(classify_drag_intent(&["AXTextField", "AXWebArea"]), Some(true));
        // Vùng văn bản nhiều dòng: cuộn
        assert_eq!(classify_drag_intent(&["AXTextArea", "AXScrollArea"]), Some(false));
        // Không có thông tin: không biết
        assert_eq!(classify_drag_intent(&[]), None);
        assert_eq!(classify_drag_intent(&["AXApplication"]), None);
    }

    #[test]
    fn text_roles() {
        for r in ["AXTextField", "AXTextArea", "AXSearchField", "AXComboBox"] {
            assert!(is_text_role(r), "{r}");
        }
        for r in ["AXButton", "AXStaticText", "AXWebArea", "AXGroup", "AXSlider"] {
            assert!(!is_text_role(r), "{r}");
        }
    }

    #[test]
    fn probes_grow_outward() {
        let pts = probe_points(P::new(100.0, 100.0), 20.0);
        assert_eq!(pts.len(), 8 * 4); // bán kính 5,10,15,20
        assert!(pts.windows(2).all(|w| w[0].0 <= w[1].0));
        assert!(pts.iter().all(|(r, q)| (q.dist(P::new(100.0, 100.0)) - r).abs() < 1e-9));
    }

    #[test]
    fn small_buttons_click_center_large_keep_probe() {
        let probe = P::new(10.0, 10.0);
        let c = click_point(probe, Some((P::new(100.0, 50.0), P::new(12.0, 12.0))));
        assert_eq!((c.x, c.y), (106.0, 56.0));
        assert_eq!(click_point(probe, Some((P::new(0.0, 0.0), P::new(300.0, 40.0)))).x, 10.0);
        assert_eq!(click_point(probe, None).x, 10.0);
    }

    #[test]
    fn finds_and_retargets_tap_clicks_only() {
        let p = P::new(50.0, 60.0);
        let mut acts = vec![
            Action::MoveTo(p),
            Action::MouseDown { button: Button::Left, at: p, clicks: 1 },
            Action::MouseUp { button: Button::Left, at: p, clicks: 1 },
            // click phải và chuỗi kéo không bị đụng tới
            Action::MoveTo(p),
            Action::MouseDown { button: Button::Right, at: p, clicks: 1 },
            Action::MouseUp { button: Button::Right, at: p, clicks: 1 },
            Action::MouseDown { button: Button::Left, at: p, clicks: 1 },
            Action::Drag(P::new(70.0, 60.0)),
        ];
        let found = find_tap_clicks(&acts);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, 0);
        retarget(&mut acts, 0, P::new(51.0, 61.0));
        assert!(matches!(acts[1], Action::MouseDown { at, .. } if at.dist(P::new(51.0, 61.0)) < 1e-9));
        assert!(matches!(acts[2], Action::MouseUp { at, .. } if at.dist(P::new(51.0, 61.0)) < 1e-9));
    }
}
