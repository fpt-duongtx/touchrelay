//! Đổi nguồn nhập (ngôn ngữ bàn phím) qua Text Input Services của macOS: chọn nguồn kế tiếp trong danh sách
//! bàn phím/bộ gõ đã bật. Không cần mã phím (phím chuyển nguồn nhập thật tuỳ cài đặt từng máy: 🌐/Fn, Ctrl+Space…).

/// Chỉ số nguồn kế tiếp (vòng lại) sau nguồn có id `current`; nếu không tìm thấy thì lấy nguồn đầu.
pub fn next_index(ids: &[String], current: &str) -> Option<usize> {
    if ids.is_empty() {
        return None;
    }
    Some(match ids.iter().position(|i| i == current) {
        Some(i) => (i + 1) % ids.len(),
        None => 0,
    })
}

/// Mã ngắn hiển thị trên phím: "en" -> "EN", "vi" -> "VI", "zh-Hans" -> "ZH".
pub fn abbreviate(lang: &str) -> String {
    let code: String = lang.chars().take_while(|c| c.is_ascii_alphabetic()).take(2).collect();
    if code.is_empty() {
        "?".to_string()
    } else {
        code.to_uppercase()
    }
}

#[cfg(target_os = "macos")]
pub use mac::{current_abbrev, switch_next};

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use crate::ffi::*;
    use std::ffi::c_void;
    use std::ptr;

    type SourceRef = *const c_void;

    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        static kTISPropertyInputSourceID: CFStringRef;
        static kTISPropertyInputSourceCategory: CFStringRef;
        static kTISPropertyInputSourceIsSelectCapable: CFStringRef;
        static kTISPropertyInputSourceLanguages: CFStringRef;
        static kTISCategoryKeyboardInputSource: CFStringRef;
        fn TISCreateInputSourceList(props: CFDictionaryRef, include_all_installed: u8) -> *const c_void;
        fn TISCopyCurrentKeyboardInputSource() -> SourceRef;
        fn TISGetInputSourceProperty(src: SourceRef, key: CFStringRef) -> *const c_void;
        fn TISSelectInputSource(src: SourceRef) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFArrayGetCount(a: *const c_void) -> CFIndex;
        fn CFArrayGetValueAtIndex(a: *const c_void, i: CFIndex) -> *const c_void;
        fn CFEqual(a: *const c_void, b: *const c_void) -> u8;
        fn CFBooleanGetValue(b: *const c_void) -> u8;
    }

    unsafe fn to_string(s: *const c_void) -> Option<String> {
        if s.is_null() || CFGetTypeID(s) != CFStringGetTypeID() {
            return None;
        }
        let mut buf = [0 as std::ffi::c_char; 256];
        if CFStringGetCString(s, buf.as_mut_ptr(), buf.len() as CFIndex, kCFStringEncodingUTF8) == 0 {
            return None;
        }
        Some(std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
    }

    unsafe fn id_of(src: SourceRef) -> Option<String> {
        to_string(TISGetInputSourceProperty(src, kTISPropertyInputSourceID))
    }

    unsafe fn abbrev_of(src: SourceRef) -> String {
        let langs = TISGetInputSourceProperty(src, kTISPropertyInputSourceLanguages);
        let first = if !langs.is_null() && CFArrayGetCount(langs) > 0 {
            to_string(CFArrayGetValueAtIndex(langs, 0))
        } else {
            None
        };
        abbreviate(&first.unwrap_or_default())
    }

    /// Mã ngôn ngữ của nguồn nhập hiện tại (vd. "EN", "VI").
    pub fn current_abbrev() -> String {
        unsafe {
            let cur = TISCopyCurrentKeyboardInputSource();
            if cur.is_null() {
                return "?".to_string();
            }
            let a = abbrev_of(cur);
            CFRelease(cur);
            a
        }
    }

    /// Chuyển sang nguồn nhập kế tiếp (bàn phím/bộ gõ đã bật, chọn được). Trả về mã ngôn ngữ mới.
    pub fn switch_next() -> Option<String> {
        unsafe {
            let list = TISCreateInputSourceList(ptr::null(), 0);
            if list.is_null() {
                return None;
            }
            let mut sources: Vec<(SourceRef, String)> = vec![];
            for i in 0..CFArrayGetCount(list).max(0) {
                let src = CFArrayGetValueAtIndex(list, i);
                let cat = TISGetInputSourceProperty(src, kTISPropertyInputSourceCategory);
                let selectable = TISGetInputSourceProperty(src, kTISPropertyInputSourceIsSelectCapable);
                if cat.is_null()
                    || CFEqual(cat, kTISCategoryKeyboardInputSource) == 0
                    || selectable.is_null()
                    || CFBooleanGetValue(selectable) == 0
                {
                    continue;
                }
                if let Some(id) = id_of(src) {
                    sources.push((src, id));
                }
            }
            let cur = TISCopyCurrentKeyboardInputSource();
            let cur_id = if cur.is_null() { String::new() } else { id_of(cur).unwrap_or_default() };
            if !cur.is_null() {
                CFRelease(cur);
            }
            let ids: Vec<String> = sources.iter().map(|s| s.1.clone()).collect();
            let result = next_index(&ids, &cur_id).and_then(|i| {
                let src = sources[i].0;
                (TISSelectInputSource(src) == 0).then(|| abbrev_of(src))
            });
            CFRelease(list);
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn cycles_through_sources_and_wraps() {
        let l = ids(&["us", "vi-telex", "jp"]);
        assert_eq!(next_index(&l, "us"), Some(1));
        assert_eq!(next_index(&l, "vi-telex"), Some(2));
        assert_eq!(next_index(&l, "jp"), Some(0)); // vòng lại
        assert_eq!(next_index(&l, "không-có"), Some(0));
        assert_eq!(next_index(&ids(&["only"]), "only"), Some(0));
        assert_eq!(next_index(&[], "x"), None);
    }

    #[test]
    fn abbreviates_language_codes() {
        assert_eq!(abbreviate("en"), "EN");
        assert_eq!(abbreviate("vi"), "VI");
        assert_eq!(abbreviate("zh-Hans"), "ZH");
        assert_eq!(abbreviate("pt_BR"), "PT");
        assert_eq!(abbreviate(""), "?");
        assert_eq!(abbreviate("123"), "?");
    }
}
