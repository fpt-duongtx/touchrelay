//! Cỡ giao diện cho màn hình cảm ứng: đổi chế độ hiển thị (độ phân giải "looks like")
//! để nút, menu, Dock to hơn, dễ chạm hơn. Chỉ đổi trong phiên đăng nhập (không lưu vĩnh viễn)
//! và được khôi phục khi thoát TouchRelay.

use std::ffi::c_void;

/// Hệ số phóng to cho từng mức (0 = giữ nguyên).
pub const SCALE_FACTORS: [f64; 4] = [1.0, 4.0 / 3.0, 1.6, 2.0];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModeInfo {
    pub width: usize,
    pub height: usize,
    pub pixel_width: usize,
    pub refresh: f64,
}

/// Chọn chế độ có độ rộng logic gần `orig.width / factor` nhất, cùng tỉ lệ khung hình
/// và tần số quét với chế độ gốc; hoà thì ưu tiên nhiều pixel hơn (HiDPI cho nét hơn).
pub fn pick_mode(orig: ModeInfo, candidates: &[ModeInfo], factor: f64) -> Option<usize> {
    let want = orig.width as f64 / factor;
    let ratio = orig.width as f64 / orig.height as f64;
    candidates
        .iter()
        .enumerate()
        .filter(|(_, m)| m.height > 0 && ((m.width as f64 / m.height as f64) - ratio).abs() < 0.01)
        .filter(|(_, m)| (m.refresh - orig.refresh).abs() < 0.5)
        .min_by(|(_, a), (_, b)| {
            let da = (a.width as f64 - want).abs();
            let db = (b.width as f64 - want).abs();
            da.total_cmp(&db).then(b.pixel_width.cmp(&a.pixel_width))
        })
        .map(|(i, _)| i)
}

#[cfg(target_os = "macos")]
pub use mac::*;

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use crate::ffi::*;
    use crate::i18n::tr;
    use std::ptr;

    type ModeRef = *const c_void;
    type ArrayRef = *const c_void;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGDisplayCopyDisplayMode(d: CGDirectDisplayID) -> ModeRef;
        fn CGDisplayCopyAllDisplayModes(d: CGDirectDisplayID, opts: CFDictionaryRef) -> ArrayRef;
        fn CGDisplayModeGetWidth(m: ModeRef) -> usize;
        fn CGDisplayModeGetHeight(m: ModeRef) -> usize;
        fn CGDisplayModeGetPixelWidth(m: ModeRef) -> usize;
        fn CGDisplayModeGetRefreshRate(m: ModeRef) -> f64;
        fn CGDisplayModeIsUsableForDesktopGUI(m: ModeRef) -> bool;
        fn CGDisplayModeRetain(m: ModeRef) -> ModeRef;
        fn CGDisplayModeRelease(m: ModeRef);
        fn CGBeginDisplayConfiguration(cfg: *mut *mut c_void) -> i32;
        fn CGConfigureDisplayWithDisplayMode(
            cfg: *mut c_void,
            d: CGDirectDisplayID,
            m: ModeRef,
            opts: CFDictionaryRef,
        ) -> i32;
        fn CGCompleteDisplayConfiguration(cfg: *mut c_void, option: u32) -> i32;
        fn CGCancelDisplayConfiguration(cfg: *mut c_void) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFArrayGetCount(a: ArrayRef) -> CFIndex;
        fn CFArrayGetValueAtIndex(a: ArrayRef, i: CFIndex) -> *const c_void;
    }

    const K_CONFIGURE_FOR_SESSION: u32 = 1;

    unsafe fn info(m: ModeRef) -> ModeInfo {
        ModeInfo {
            width: CGDisplayModeGetWidth(m),
            height: CGDisplayModeGetHeight(m),
            pixel_width: CGDisplayModeGetPixelWidth(m),
            refresh: CGDisplayModeGetRefreshRate(m),
        }
    }

    unsafe fn apply(id: CGDirectDisplayID, m: ModeRef) -> Result<(), String> {
        let mut cfg: *mut c_void = ptr::null_mut();
        if CGBeginDisplayConfiguration(&mut cfg) != 0 {
            return Err(tr("không bắt đầu được cấu hình màn hình"));
        }
        if CGConfigureDisplayWithDisplayMode(cfg, id, m, ptr::null()) != 0 {
            CGCancelDisplayConfiguration(cfg);
            return Err(tr("không đặt được chế độ hiển thị"));
        }
        if CGCompleteDisplayConfiguration(cfg, K_CONFIGURE_FOR_SESSION) != 0 {
            return Err(tr("không áp dụng được chế độ hiển thị"));
        }
        Ok(())
    }

    /// Chế độ gốc của màn hình trước khi TouchRelay đổi cỡ (để khôi phục).
    pub struct ScaleState {
        orig: Option<(CGDirectDisplayID, ModeRef)>,
    }

    impl ScaleState {
        pub const fn new() -> Self {
            ScaleState { orig: None }
        }

        /// Đặt mức cỡ giao diện (0 = gốc) cho màn hình `id`.
        pub fn set(&mut self, id: CGDirectDisplayID, level: usize) -> Result<(), String> {
            unsafe {
                // Màn hình đích đổi: khôi phục màn hình cũ trước.
                if let Some((oid, _)) = self.orig {
                    if oid != id {
                        self.restore();
                    }
                }
                if level == 0 {
                    self.restore();
                    return Ok(());
                }
                let factor = *SCALE_FACTORS.get(level).ok_or_else(|| tr("không có chế độ phù hợp"))?;
                if self.orig.is_none() {
                    let cur = CGDisplayCopyDisplayMode(id);
                    if cur.is_null() {
                        return Err(tr("không đọc được chế độ hiện tại"));
                    }
                    self.orig = Some((id, cur));
                }
                let (_, orig) = self.orig.unwrap();
                let arr = CGDisplayCopyAllDisplayModes(id, ptr::null());
                if arr.is_null() {
                    return Err(tr("không liệt kê được chế độ hiển thị"));
                }
                let n = CFArrayGetCount(arr).max(0);
                let mut modes: Vec<ModeRef> = vec![];
                for i in 0..n {
                    let m = CFArrayGetValueAtIndex(arr, i);
                    if CGDisplayModeIsUsableForDesktopGUI(m) {
                        modes.push(m);
                    }
                }
                let infos: Vec<ModeInfo> = modes.iter().map(|&m| info(m)).collect();
                let res = match pick_mode(info(orig), &infos, factor) {
                    Some(i) => apply(id, modes[i]),
                    None => Err(tr("không có chế độ phù hợp")),
                };
                CFRelease(arr as CFTypeRef);
                res
            }
        }

        /// Trả về chế độ gốc (nếu đã đổi).
        pub fn restore(&mut self) {
            if let Some((id, m)) = self.orig.take() {
                unsafe {
                    let _ = apply(id, m);
                    CGDisplayModeRelease(m);
                }
            }
        }
    }

    // CGDisplayModeRetain chỉ dùng để tránh cảnh báo khi cần giữ thêm tham chiếu.
    #[allow(dead_code)]
    unsafe fn retain(m: ModeRef) -> ModeRef {
        CGDisplayModeRetain(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(w: usize, h: usize, px: usize, hz: f64) -> ModeInfo {
        ModeInfo {
            width: w,
            height: h,
            pixel_width: px,
            refresh: hz,
        }
    }

    fn sample() -> Vec<ModeInfo> {
        vec![
            m(2560, 1600, 2560, 120.0),
            m(1920, 1200, 1920, 120.0),
            m(1600, 1000, 1600, 120.0),
            m(1280, 800, 1280, 120.0),
            m(1280, 800, 2560, 120.0),
            m(1280, 800, 2560, 60.0),
            m(1920, 1080, 1920, 120.0),
        ]
    }

    #[test]
    fn picks_scaled_modes() {
        let orig = m(2560, 1600, 2560, 120.0);
        let s = sample();
        assert_eq!(s[pick_mode(orig, &s, 4.0 / 3.0).unwrap()].width, 1920);
        assert_eq!(s[pick_mode(orig, &s, 1.6).unwrap()].width, 1600);
        // 2x: ưu tiên chế độ HiDPI (nhiều pixel) cùng tần số quét
        let i = pick_mode(orig, &s, 2.0).unwrap();
        assert_eq!((s[i].width, s[i].pixel_width, s[i].refresh), (1280, 2560, 120.0));
    }

    #[test]
    fn ignores_other_aspect_ratio() {
        let orig = m(2560, 1600, 2560, 120.0);
        let s = vec![m(1920, 1080, 1920, 120.0)];
        assert_eq!(pick_mode(orig, &s, 4.0 / 3.0), None);
    }
}
