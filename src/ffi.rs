//! FFI thuần tới CoreFoundation, IOKit và CoreGraphics (không dùng crate ngoài).
#![allow(non_upper_case_globals, non_snake_case, dead_code)]

use std::ffi::{c_char, c_void};

pub type CFTypeRef = *const c_void;
pub type CFAllocatorRef = *const c_void;
pub type CFStringRef = *const c_void;
pub type CFNumberRef = *const c_void;
pub type CFDictionaryRef = *const c_void;
pub type CFMutableDictionaryRef = *mut c_void;
pub type CFSetRef = *const c_void;
pub type CFDataRef = *const c_void;
pub type CFRunLoopRef = *mut c_void;
pub type CFRunLoopTimerRef = *mut c_void;
pub type CFIndex = isize;
pub type CFTypeID = usize;
pub type CFOptionFlags = usize;
pub type Boolean = u8;

pub type IOHIDManagerRef = *mut c_void;
pub type IOHIDDeviceRef = *mut c_void;
pub type IOReturn = i32;
pub type IOOptionBits = u32;

pub type CGEventRef = *mut c_void;
pub type CGEventSourceRef = *const c_void;
pub type CGDirectDisplayID = u32;

pub const kCFStringEncodingUTF8: u32 = 0x0800_0100;
pub const kCFNumberSInt32Type: CFIndex = 3;
pub const kCFNumberSInt64Type: CFIndex = 4;

pub const kIOReturnSuccess: IOReturn = 0;
pub const kIOHIDOptionsTypeNone: IOOptionBits = 0;
pub const kIOHIDOptionsTypeSeizeDevice: IOOptionBits = 1;
pub const kIOHIDReportTypeInput: u32 = 0;
pub const kIOHIDReportTypeFeature: u32 = 2;
pub const kIOHIDRequestTypeListenEvent: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

/// Chỉ cần địa chỉ của các struct callback toàn cục, nên khai báo opaque.
#[repr(C)]
pub struct Opaque {
    _p: [u8; 0],
}

#[repr(C)]
pub struct CFRunLoopTimerContext {
    pub version: CFIndex,
    pub info: *mut c_void,
    pub retain: Option<extern "C" fn(*const c_void) -> *const c_void>,
    pub release: Option<extern "C" fn(*const c_void)>,
    pub copyDescription: Option<extern "C" fn(*const c_void) -> CFStringRef>,
}

pub type IOHIDDeviceCallback = extern "C" fn(ctx: *mut c_void, result: IOReturn, sender: *mut c_void, dev: IOHIDDeviceRef);
pub type IOHIDReportCallback = extern "C" fn(
    context: *mut c_void,
    result: IOReturn,
    sender: *mut c_void,
    report_type: u32,
    report_id: u32,
    report: *mut u8,
    report_length: CFIndex,
);

pub type CFRunLoopTimerCallBack = extern "C" fn(timer: CFRunLoopTimerRef, info: *mut c_void);

#[cfg_attr(target_os = "macos", link(name = "CoreFoundation", kind = "framework"))]
extern "C" {
    pub static kCFAllocatorDefault: CFAllocatorRef;
    pub static kCFRunLoopDefaultMode: CFStringRef;
    pub static kCFRunLoopCommonModes: CFStringRef;
    pub static kCFBooleanTrue: CFTypeRef;
    pub static kCFTypeDictionaryKeyCallBacks: Opaque;
    pub static kCFTypeDictionaryValueCallBacks: Opaque;

    pub fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;

    pub fn CFStringCreateWithCString(alloc: CFAllocatorRef, s: *const c_char, enc: u32) -> CFStringRef;
    pub fn CFStringGetTypeID() -> CFTypeID;
    pub fn CFStringGetCString(s: CFStringRef, buf: *mut c_char, size: CFIndex, enc: u32) -> Boolean;

    pub fn CFNumberCreate(alloc: CFAllocatorRef, the_type: CFIndex, value: *const c_void) -> CFNumberRef;
    pub fn CFNumberGetTypeID() -> CFTypeID;
    pub fn CFNumberGetValue(n: CFNumberRef, the_type: CFIndex, value: *mut c_void) -> Boolean;

    pub fn CFDataGetTypeID() -> CFTypeID;
    pub fn CFDataGetLength(d: CFDataRef) -> CFIndex;
    pub fn CFDataGetBytePtr(d: CFDataRef) -> *const u8;

    pub fn CFDictionaryCreateMutable(
        alloc: CFAllocatorRef,
        capacity: CFIndex,
        key_cb: *const Opaque,
        value_cb: *const Opaque,
    ) -> CFMutableDictionaryRef;
    pub fn CFDictionarySetValue(d: CFMutableDictionaryRef, key: *const c_void, value: *const c_void);

    pub fn CFSetGetCount(s: CFSetRef) -> CFIndex;
    pub fn CFSetGetValues(s: CFSetRef, values: *mut *const c_void);

    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopRun();
    pub fn CFRunLoopStop(rl: CFRunLoopRef);
    pub fn CFRunLoopTimerCreate(
        alloc: CFAllocatorRef,
        fire_date: f64,
        interval: f64,
        flags: CFOptionFlags,
        order: CFIndex,
        callout: CFRunLoopTimerCallBack,
        context: *mut CFRunLoopTimerContext,
    ) -> CFRunLoopTimerRef;
    pub fn CFRunLoopAddTimer(rl: CFRunLoopRef, timer: CFRunLoopTimerRef, mode: CFStringRef);
    pub fn CFAbsoluteTimeGetCurrent() -> f64;
}

#[cfg_attr(target_os = "macos", link(name = "IOKit", kind = "framework"))]
extern "C" {
    pub fn IOHIDManagerCreate(alloc: CFAllocatorRef, options: IOOptionBits) -> IOHIDManagerRef;
    pub fn IOHIDManagerSetDeviceMatching(m: IOHIDManagerRef, matching: CFDictionaryRef);
    pub fn IOHIDManagerCopyDevices(m: IOHIDManagerRef) -> CFSetRef;
    pub fn IOHIDManagerRegisterDeviceMatchingCallback(m: IOHIDManagerRef, cb: IOHIDDeviceCallback, ctx: *mut c_void);
    pub fn IOHIDManagerRegisterDeviceRemovalCallback(m: IOHIDManagerRef, cb: IOHIDDeviceCallback, ctx: *mut c_void);
    pub fn IOHIDManagerScheduleWithRunLoop(m: IOHIDManagerRef, rl: CFRunLoopRef, mode: CFStringRef);

    pub fn IOHIDDeviceOpen(d: IOHIDDeviceRef, options: IOOptionBits) -> IOReturn;
    pub fn IOHIDDeviceClose(d: IOHIDDeviceRef, options: IOOptionBits) -> IOReturn;
    pub fn IOHIDDeviceGetProperty(d: IOHIDDeviceRef, key: CFStringRef) -> CFTypeRef;
    pub fn IOHIDDeviceScheduleWithRunLoop(d: IOHIDDeviceRef, rl: CFRunLoopRef, mode: CFStringRef);
    pub fn IOHIDDeviceRegisterInputReportCallback(
        d: IOHIDDeviceRef,
        report: *mut u8,
        report_length: CFIndex,
        callback: IOHIDReportCallback,
        context: *mut c_void,
    );
    pub fn IOHIDDeviceGetReport(
        d: IOHIDDeviceRef,
        report_type: u32,
        report_id: CFIndex,
        report: *mut u8,
        report_length: *mut CFIndex,
    ) -> IOReturn;
    pub fn IOHIDDeviceSetReport(
        d: IOHIDDeviceRef,
        report_type: u32,
        report_id: CFIndex,
        report: *const u8,
        report_length: CFIndex,
    ) -> IOReturn;

    /// macOS 10.15+: hỏi quyền Input Monitoring.
    pub fn IOHIDRequestAccess(request_type: u32) -> bool;
}

// ---- CoreGraphics ----
pub const kCGHIDEventTap: u32 = 0;

pub const kCGEventLeftMouseDown: u32 = 1;
pub const kCGEventLeftMouseUp: u32 = 2;
pub const kCGEventRightMouseDown: u32 = 3;
pub const kCGEventRightMouseUp: u32 = 4;
pub const kCGEventMouseMoved: u32 = 5;
pub const kCGEventLeftMouseDragged: u32 = 6;
pub const kCGMouseButtonLeft: u32 = 0;
pub const kCGMouseButtonRight: u32 = 1;

pub const kCGScrollEventUnitPixel: u32 = 0;

// Event fields (CGEventTypes.h)
pub const kCGMouseEventClickState: u32 = 1;
pub const kCGScrollWheelEventIsContinuous: u32 = 88;
pub const kCGScrollWheelEventScrollPhase: u32 = 99;
pub const kCGScrollWheelEventMomentumPhase: u32 = 123;

pub const kCGEventFlagMaskControl: u64 = 0x0004_0000;
pub const kCGEventFlagMaskNumericPad: u64 = 0x0020_0000;
pub const kCGEventFlagMaskSecondaryFn: u64 = 0x0080_0000;

// ---- Gesture event (API riêng, không công khai – reverse engineered) ----
pub const kCGSEventTypeField: u32 = 55;
pub const kCGSEventGesture: u32 = 29;
pub const kCGEventGestureHIDType: u32 = 110;
pub const kCGEventGestureZoomValue: u32 = 113;
pub const kCGEventGesturePhase: u32 = 132;
pub const kIOHIDEventTypeZoom: i64 = 8;
pub const kIOHIDEventTypeGestureStarted: i64 = 61;
pub const kIOHIDEventTypeGestureEnded: i64 = 62;

#[cfg_attr(target_os = "macos", link(name = "CoreGraphics", kind = "framework"))]
extern "C" {
    pub fn CGEventCreate(source: CGEventSourceRef) -> CGEventRef;
    pub fn CGEventCreateMouseEvent(source: CGEventSourceRef, t: u32, pos: CGPoint, button: u32) -> CGEventRef;
    pub fn CGEventCreateScrollWheelEvent2(
        source: CGEventSourceRef,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> CGEventRef;
    pub fn CGEventCreateKeyboardEvent(source: CGEventSourceRef, keycode: u16, key_down: bool) -> CGEventRef;
    pub fn CGEventSetType(e: CGEventRef, t: u32);
    pub fn CGEventSetLocation(e: CGEventRef, p: CGPoint);
    pub fn CGEventSetFlags(e: CGEventRef, flags: u64);
    pub fn CGEventSetIntegerValueField(e: CGEventRef, field: u32, value: i64);
    pub fn CGEventSetDoubleValueField(e: CGEventRef, field: u32, value: f64);
    pub fn CGEventPost(tap: u32, e: CGEventRef);

    pub fn CGGetActiveDisplayList(max: u32, displays: *mut CGDirectDisplayID, count: *mut u32) -> i32;
    pub fn CGDisplayBounds(d: CGDirectDisplayID) -> CGRect;
    pub fn CGDisplayIsBuiltin(d: CGDirectDisplayID) -> u32;
    pub fn CGMainDisplayID() -> CGDirectDisplayID;
    pub fn CGDisplayHideCursor(d: CGDirectDisplayID) -> i32;
    pub fn CGDisplayShowCursor(d: CGDirectDisplayID) -> i32;
    pub fn CGEventGetLocation(e: CGEventRef) -> CGPoint;
    // API riêng: cho phép app nền ẩn con trỏ.
    pub fn CGSMainConnectionID() -> i32;
    pub fn CGSSetConnectionProperty(cid: i32, target: i32, key: CFStringRef, value: CFTypeRef) -> i32;

    /// macOS 10.15+: quyền Accessibility để post event.
    pub fn CGPreflightPostEventAccess() -> bool;
    pub fn CGRequestPostEventAccess() -> bool;
}

// ---- Helpers ----

pub struct CfStr(pub CFStringRef);

impl CfStr {
    pub fn new(s: &str) -> Self {
        let c = std::ffi::CString::new(s).unwrap();
        CfStr(unsafe { CFStringCreateWithCString(kCFAllocatorDefault, c.as_ptr(), kCFStringEncodingUTF8) })
    }
}

impl Drop for CfStr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) }
        }
    }
}

pub fn cf_number_i32(v: i32) -> CFNumberRef {
    unsafe {
        CFNumberCreate(
            kCFAllocatorDefault,
            kCFNumberSInt32Type,
            &v as *const i32 as *const c_void,
        )
    }
}

pub unsafe fn device_prop_i64(dev: IOHIDDeviceRef, key: &str) -> Option<i64> {
    let k = CfStr::new(key);
    let v = IOHIDDeviceGetProperty(dev, k.0);
    if v.is_null() || CFGetTypeID(v) != CFNumberGetTypeID() {
        return None;
    }
    let mut out: i64 = 0;
    if CFNumberGetValue(v, kCFNumberSInt64Type, &mut out as *mut i64 as *mut c_void) != 0 {
        Some(out)
    } else {
        None
    }
}

pub unsafe fn device_prop_string(dev: IOHIDDeviceRef, key: &str) -> Option<String> {
    let k = CfStr::new(key);
    let v = IOHIDDeviceGetProperty(dev, k.0);
    if v.is_null() || CFGetTypeID(v) != CFStringGetTypeID() {
        return None;
    }
    let mut buf = [0 as c_char; 256];
    if CFStringGetCString(v, buf.as_mut_ptr(), buf.len() as CFIndex, kCFStringEncodingUTF8) == 0 {
        return None;
    }
    Some(std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
}

pub unsafe fn device_prop_data(dev: IOHIDDeviceRef, key: &str) -> Option<Vec<u8>> {
    let k = CfStr::new(key);
    let v = IOHIDDeviceGetProperty(dev, k.0);
    if v.is_null() || CFGetTypeID(v) != CFDataGetTypeID() {
        return None;
    }
    let len = CFDataGetLength(v);
    let ptr = CFDataGetBytePtr(v);
    if ptr.is_null() || len <= 0 {
        return None;
    }
    Some(std::slice::from_raw_parts(ptr, len as usize).to_vec())
}
