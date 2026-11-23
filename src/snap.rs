//! "Chạm thông minh": khi chạm gần một nút nhỏ nhưng không trúng, hút vào điều khiển gần nhất.
//! Dùng Accessibility API (AXUIElementCopyElementAtPosition) nên cần quyền Accessibility (đã cần sẵn).
//! Chỉ áp dụng cho tap (click trái), không áp dụng cho kéo/cuộn.

use crate::gesture::{Action, Button, P};

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
pub use mac::snap_point;

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
