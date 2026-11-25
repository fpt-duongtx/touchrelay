//! Phần chạy trên macOS: tìm & chiếm (seize) màn hình cảm ứng qua IOKit,
//! đọc raw HID report, đưa qua gesture engine rồi post CGEvent.

use crate::ffi::*;
use crate::gesture::{Action, Button, Engine, GesturePhase, Key, MomentumPhase, ScrollPhase, Touch, P};
use crate::calib::{fit_calib, Calib};
use crate::coverage::Canvas;
use crate::hid_desc;
use crate::i18n::tr;
use crate::menubar;
use crate::touch::{FrameAssembler, TouchLayout, PAGE_DIGITIZER, USAGE_TOUCH_SCREEN};
use crate::Options;
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

static STOP: AtomicBool = AtomicBool::new(false);

// ============================================================
// Map toạ độ chuẩn hoá -> toạ độ global của màn hình đích
// ============================================================
struct Mapper {
    bounds: CGRect,
    swap_xy: bool,
    flip_x: bool,
    flip_y: bool,
    /// Hiệu chỉnh vị trí chạm do người dùng chạy (chưa kể vùng nhận chạm).
    calib_base: Option<Calib>,
    /// Vùng cảm biến thật sự nhận chạm [min x, max x, min y, max y] đo từ "Kiểm tra vùng chạm".
    extent: Option<[f64; 4]>,
    /// Độ phân giải (điểm) của màn hình đích lúc hiệu chỉnh. Khác độ phân giải hiện tại thì hiệu chỉnh
    /// tự tắt: màn hình cảm ứng có thể đổi vùng chạm theo chế độ hiển thị (vd. 16:9 ↔ 16:10).
    calib_size: Option<(u32, u32)>,
    /// Ánh xạ hiệu dụng = calib_base (hoặc đồng nhất) + extent; áp dụng sau xoay/lật.
    calib: Option<Calib>,
}

impl Mapper {
    /// Toạ độ cảm biến sau khi xoay/lật, trước hiệu chỉnh.
    fn oriented(&self, mut x: f64, mut y: f64) -> (f64, f64) {
        if self.swap_xy {
            std::mem::swap(&mut x, &mut y);
        }
        if self.flip_x {
            x = 1.0 - x;
        }
        if self.flip_y {
            y = 1.0 - y;
        }
        (x, y)
    }

    fn current_size(&self) -> (u32, u32) {
        (self.bounds.size.width.round() as u32, self.bounds.size.height.round() as u32)
    }

    /// Hiệu chỉnh đã lưu nhưng đang tắt vì độ phân giải màn hình đích đã đổi.
    fn calib_stale(&self) -> bool {
        (self.calib_base.is_some() || self.extent.is_some()) && self.calib_size.is_some_and(|s| s != self.current_size())
    }

    fn refresh_calib(&mut self) {
        if self.calib_stale() {
            self.calib = None;
            return;
        }
        self.calib = match (&self.calib_base, self.extent) {
            (Some(b), Some(e)) => Some(b.with_extents(e)),
            (Some(b), None) => Some(b.clone()),
            (None, Some(e)) => Some(Calib::identity().with_extents(e)),
            (None, None) => None,
        };
    }

    /// Đổi sang hiệu chỉnh/phạm vi mới: ghi nhận độ phân giải hiện tại; nếu độ phân giải khác với
    /// dữ liệu cũ thì bỏ dữ liệu cũ (không còn đúng cho chế độ này).
    fn adopt_size(&mut self) {
        let size = self.current_size();
        if self.calib_size.is_some_and(|s| s != size) {
            self.calib_base = None;
            self.extent = None;
        }
        self.calib_size = Some(size);
    }

    fn set_calib(&mut self, c: Option<Calib>) {
        if c.is_some() {
            self.adopt_size();
        }
        self.calib_base = c;
        if self.calib_base.is_none() && self.extent.is_none() {
            self.calib_size = None;
        }
        self.refresh_calib();
    }

    fn set_extent(&mut self, e: Option<[f64; 4]>) {
        if e.is_some() {
            self.adopt_size();
        }
        self.extent = e;
        if self.calib_base.is_none() && self.extent.is_none() {
            self.calib_size = None;
        }
        self.refresh_calib();
    }

    /// Toạ độ màn hình chuẩn hoá 0..1 (gốc trên-trái) sau xoay/lật + hiệu chỉnh, KHÔNG kẹp vào [0,1].
    fn normalized(&self, x: f64, y: f64) -> (f64, f64) {
        let (x, y) = self.oriented(x, y);
        match &self.calib {
            Some(c) => c.apply(x, y),
            None => (x, y),
        }
    }

    fn to_screen(&self, x: f64, y: f64) -> P {
        let (x, y) = self.normalized(x, y);
        let (x, y) = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
        let b = self.bounds;
        P::new(
            b.origin.x + (x * b.size.width).min(b.size.width - 1.0),
            b.origin.y + (y * b.size.height).min(b.size.height - 1.0),
        )
    }
}

// ============================================================
// Sink: Action -> CGEvent
// ============================================================
struct Sink {
    rem: (f64, f64),
    scroll_scale: f64,
    invert_scroll: bool,
    debug: bool,
    /// Tâm các điểm chạm gần nhất; dùng làm vị trí con trỏ khi cuộn.
    hint: Option<P>,
    /// Sự kiện chuột chờ phát (giãn cách như chuột thật để menu/popup nhận đúng thứ tự).
    queue: std::collections::VecDeque<(f64, Action)>,
    last_due: f64,
    move_due: f64,
    down_due: f64,
    prev_move: bool,
    hide_cursor: bool,
    cursor_hidden: bool,
    /// Vị trí con trỏ do chính app đặt gần nhất (để phân biệt với chuột/trackpad thật).
    last_pos: Option<P>,
    last_touch_t: f64,
}

fn pt(p: P) -> CGPoint {
    CGPoint { x: p.x, y: p.y }
}

unsafe fn post(e: CGEventRef) {
    if !e.is_null() {
        CGEventPost(kCGHIDEventTap, e);
        CFRelease(e as CFTypeRef);
    }
}

/// Phát một phím (nhấn rồi nhả) với cờ bổ trợ, như bàn phím vật lý. Dùng mã phím nên bộ gõ hệ thống (Telex/VNI...) hoạt động.
pub(crate) fn post_key(code: u16, flags: u64) {
    unsafe {
        for down in [true, false] {
            let e = CGEventCreateKeyboardEvent(ptr::null(), code, down);
            if !e.is_null() {
                CGEventSetFlags(e, flags);
            }
            post(e);
        }
    }
}

unsafe fn mouse(t: u32, at: P, button: u32, clicks: i64) {
    let e = CGEventCreateMouseEvent(ptr::null(), t, pt(at), button);
    if !e.is_null() {
        // Chạm KHÔNG bao giờ mang phím bổ trợ: sau vuốt 3 ngón (Ctrl+mũi tên) cờ Ctrl có thể còn kẹt trong trạng thái
        // phiên, khiến click trái thành Ctrl-click = mở context menu.
        CGEventSetFlags(e, 0);
        if clicks > 0 {
            CGEventSetIntegerValueField(e, kCGMouseEventClickState, clicks);
        }
    }
    post(e);
}

unsafe fn gesture(hid_type: i64, phase: i64, zoom: Option<f64>, at: P) {
    let e = CGEventCreate(ptr::null());
    if e.is_null() {
        return;
    }
    CGEventSetType(e, kCGSEventGesture);
    CGEventSetIntegerValueField(e, kCGSEventTypeField, kCGSEventGesture as i64);
    CGEventSetIntegerValueField(e, kCGEventGestureHIDType, hid_type);
    if let Some(z) = zoom {
        CGEventSetDoubleValueField(e, kCGEventGestureZoomValue, z);
        CGEventSetIntegerValueField(e, kCGEventGesturePhase, phase);
    }
    CGEventSetLocation(e, pt(at));
    post(e);
}

impl Sink {
    /// Xếp lịch phát một action. Click = MoveTo → (15ms) MouseDown → (40ms) MouseUp,
    /// vì menu ngữ cảnh bỏ qua click phát liền trong cùng một vòng lặp.
    fn apply(&mut self, a: &Action) {
        let now = unsafe { CFAbsoluteTimeGetCurrent() };
        let mut due = now.max(self.last_due);
        match a {
            Action::MouseDown { .. } => {
                if self.prev_move {
                    due = due.max(self.move_due + 0.015);
                }
                self.down_due = due;
            }
            Action::MouseUp { .. } => due = due.max(self.down_due + 0.04),
            _ => {}
        }
        self.prev_move = matches!(a, Action::MoveTo(_));
        if self.prev_move {
            self.move_due = due;
        }
        self.last_due = due;
        if due <= now && self.queue.is_empty() {
            self.exec(a);
        } else {
            self.queue.push_back((due, a.clone()));
        }
    }

    /// Phát các action đã đến hạn (gọi từ timer 120Hz).
    fn pump(&mut self, now: f64) {
        while self.queue.front().is_some_and(|(t, _)| *t <= now) {
            if let Some((_, a)) = self.queue.pop_front() {
                self.exec(&a);
            }
        }
    }

    /// Phát hết ngay (khi thoát).
    fn flush(&mut self) {
        while let Some((_, a)) = self.queue.pop_front() {
            self.exec(&a);
        }
    }

    fn hide_cursor_now(&mut self) {
        if !self.hide_cursor || self.cursor_hidden {
            return;
        }
        unsafe {
            // Cho phép app nền ẩn con trỏ (API riêng; nếu thất bại thì con trỏ chỉ không ẩn).
            let cid = CGSMainConnectionID();
            CGSSetConnectionProperty(cid, cid, CfStr::new("SetsCursorInBackground").0, kCFBooleanTrue);
            CGDisplayHideCursor(CGMainDisplayID());
        }
        self.cursor_hidden = true;
    }

    fn show_cursor_now(&mut self) {
        if self.cursor_hidden {
            unsafe { CGDisplayShowCursor(CGMainDisplayID()) };
            self.cursor_hidden = false;
        }
    }

    /// Chuột/trackpad thật vừa di chuyển (con trỏ lệch khỏi vị trí app đặt) -> hiện lại con trỏ.
    fn check_real_pointer(&mut self, now: f64) {
        if !self.cursor_hidden || !self.queue.is_empty() || now - self.last_touch_t < 0.25 {
            return;
        }
        let Some(p) = self.last_pos else { return };
        let cur = unsafe {
            let e = CGEventCreate(ptr::null());
            if e.is_null() {
                return;
            }
            let l = CGEventGetLocation(e);
            CFRelease(e as CFTypeRef);
            P::new(l.x, l.y)
        };
        if cur.dist(p) > 4.0 {
            self.show_cursor_now();
        }
    }

    fn exec(&mut self, a: &Action) {
        match a {
            Action::MoveTo(p) | Action::Drag(p) => self.last_pos = Some(*p),
            Action::MouseDown { at, .. } | Action::MouseUp { at, .. } => self.last_pos = Some(*at),
            _ => {}
        }
        if self.debug {
            eprintln!("  -> {a:?}");
        }
        unsafe {
            match *a {
                Action::MoveTo(p) => mouse(kCGEventMouseMoved, p, kCGMouseButtonLeft, 0),
                Action::Drag(p) => mouse(kCGEventLeftMouseDragged, p, kCGMouseButtonLeft, 0),
                Action::MouseDown { button, at, clicks } => match button {
                    Button::Left => mouse(kCGEventLeftMouseDown, at, kCGMouseButtonLeft, clicks),
                    Button::Right => mouse(kCGEventRightMouseDown, at, kCGMouseButtonRight, clicks),
                },
                Action::MouseUp { button, at, clicks } => match button {
                    Button::Left => mouse(kCGEventLeftMouseUp, at, kCGMouseButtonLeft, clicks),
                    Button::Right => mouse(kCGEventRightMouseUp, at, kCGMouseButtonRight, clicks),
                },
                Action::Scroll {
                    dx,
                    dy,
                    phase,
                    momentum,
                } => {
                    // Kiểu "natural": nội dung đi theo ngón tay.
                    // Wheel dương = nội dung đi xuống/sang phải = ngón đi xuống/sang phải.
                    let sign = if self.invert_scroll { -1.0 } else { 1.0 };
                    let wy = self.rem.1 + dy * self.scroll_scale * sign;
                    let wx = self.rem.0 + dx * self.scroll_scale * sign;
                    let (iy, ix) = (wy.round(), wx.round());
                    self.rem = (wx - ix, wy - iy);
                    // Cuộn đi vào cửa sổ dưới con trỏ: đưa con trỏ tới tâm hai ngón khi bắt đầu.
                    if matches!(phase, ScrollPhase::Began) {
                        if let Some(h) = self.hint {
                            mouse(kCGEventMouseMoved, h, kCGMouseButtonLeft, 0);
                            self.last_pos = Some(h);
                        }
                    }
                    let e = CGEventCreateScrollWheelEvent2(
                        ptr::null(),
                        kCGScrollEventUnitPixel,
                        2,
                        iy as i32,
                        ix as i32,
                        0,
                    );
                    if e.is_null() {
                        return;
                    }
                    if let Some(h) = self.hint {
                        CGEventSetLocation(e, pt(h));
                    }
                    CGEventSetIntegerValueField(e, kCGScrollWheelEventIsContinuous, 1);
                    let sp = match phase {
                        ScrollPhase::None => 0,
                        ScrollPhase::Began => 1,
                        ScrollPhase::Changed => 2,
                        ScrollPhase::Ended => 4,
                    };
                    let mp = match momentum {
                        MomentumPhase::None => 0,
                        MomentumPhase::Begin => 1,
                        MomentumPhase::Continue => 2,
                        MomentumPhase::End => 3,
                    };
                    CGEventSetIntegerValueField(e, kCGScrollWheelEventScrollPhase, sp);
                    CGEventSetIntegerValueField(e, kCGScrollWheelEventMomentumPhase, mp);
                    post(e);
                    if matches!(phase, ScrollPhase::Ended) || matches!(momentum, MomentumPhase::End) {
                        self.rem = (0.0, 0.0);
                    }
                }
                Action::Magnify { delta, phase, at } => match phase {
                    GesturePhase::Began => {
                        gesture(kIOHIDEventTypeGestureStarted, 0, None, at);
                        gesture(kIOHIDEventTypeZoom, 1, Some(delta), at);
                    }
                    GesturePhase::Changed => gesture(kIOHIDEventTypeZoom, 2, Some(delta), at),
                    GesturePhase::Ended => {
                        gesture(kIOHIDEventTypeZoom, 4, Some(0.0), at);
                        gesture(kIOHIDEventTypeGestureEnded, 0, None, at);
                    }
                },
                Action::ToggleKeyboard => {} // do App xử lý (cần trạng thái bàn phím), không phải Sink
                Action::CtrlKey(k) => {
                    let code: u16 = match k {
                        Key::Left => 123,
                        Key::Right => 124,
                        Key::Down => 125,
                        Key::Up => 126,
                    };
                    let flags = kCGEventFlagMaskControl | kCGEventFlagMaskSecondaryFn | kCGEventFlagMaskNumericPad;
                    for down in [true, false] {
                        let e = CGEventCreateKeyboardEvent(ptr::null(), code, down);
                        if !e.is_null() {
                            CGEventSetFlags(e, flags);
                        }
                        post(e);
                    }
                }
            }
        }
    }
}

// ============================================================
// App state (chỉ truy cập từ main run loop)
// ============================================================
pub(crate) struct App {
    pub(crate) engine: Engine,
    sink: Sink,
    map: Mapper,
    debug: bool,
    pub(crate) paused: bool,
    /// Đang chạy dưới dạng TouchRelay.app (có menu bar).
    ui: bool,
    /// Mô tả ngắn thiết bị/màn hình đích, hiện ở mục Khắc phục sự cố.
    pub(crate) devices: Vec<String>,
    display: Option<usize>,
    ticks: u32,
    scale: crate::display::ScaleState,
    pub(crate) scale_level: usize,
    snap: bool,
    /// Bàn phím ảo: bật tự hiện, đã bị người dùng đóng tay (chờ tới khi rời ô nhập mới tự hiện lại),
    /// và lịch kiểm tra ô nhập sau một cú chạm (thời điểm, vị trí chạm).
    keyboard: bool,
    swipe_level: u8,
    kbd_dismissed: bool,
    kbd_check: Option<(f64, P)>,
    probe: Option<Probe>,
    calib_run: Option<CalibRun>,
    /// Thời điểm tự đóng lớp phủ hiệu chỉnh (đang hiện thông báo kết quả).
    overlay_until: Option<f64>,
}

/// Vị trí các dấu + (toạ độ màn hình chuẩn hoá 0..1, gốc trên-trái): hai đường chéo, thụt 3%.
/// Số giây chờ chạm trước khi tự bỏ qua một dấu.
const CALIB_SKIP_SECS: f64 = 8.0;

const CALIB_TARGETS: [(f64, f64); 9] = [
    // Hai đường chéo, thụt 6%: mỗi giá trị trục có ≥2 mẫu. (Thụt 3% thì ILITEK không nhận chạm ở góc dưới-phải.)
    (0.06, 0.06),
    (0.28, 0.28),
    (0.5, 0.5),
    (0.72, 0.72),
    (0.94, 0.94),
    (0.06, 0.94),
    (0.28, 0.72),
    (0.72, 0.28),
    (0.94, 0.06),
];

fn extent_info(e: [f64; 4]) -> String {
    format!("x {:.1}–{:.1}% · y {:.1}–{:.1}%", e[0] * 100.0, e[1] * 100.0, e[2] * 100.0, e[3] * 100.0)
}

/// Màu đặc cho "Kiểm tra màn hình" (tìm điểm ảnh chết / sáng không đều).
const PROBE_COLORS: [(u8, u8, u8); 6] = [(255, 255, 255), (0, 0, 0), (255, 0, 0), (0, 255, 0), (0, 0, 255), (128, 128, 128)];

/// Công cụ kiểm tra chạy trên lớp phủ của màn hình đích; mọi thao tác chạm chỉ dùng để đo, không click.
enum Probe {
    /// Vẽ vàng nơi cảm biến ghi nhận chạm.
    Touch(TouchTest),
    /// Chạm để đổi màu đặc (`idx` = None: màn hình hướng dẫn); `down` = đang có ngón chạm.
    Colors { idx: Option<usize>, down: bool },
}

struct TouchTest {
    canvas: Canvas,
    /// Vị trí chuẩn hoá lần trước của mỗi ngón (None = ngoài màn hình), để nối nét.
    last: Vec<(u32, Option<(f64, f64)>)>,
    /// [min x, max x, min y, max y] của toạ độ cảm biến (sau xoay/lật, trước hiệu chỉnh) đã đạt tới.
    ext: Option<[f64; 4]>,
    dirty: bool,
    last_draw: f64,
}

struct CalibRun {
    samples: Vec<((f64, f64), (f64, f64))>,
    idx: usize,
    /// Các vị trí cảm biến (đã xoay/lật) của lần chạm đang diễn ra.
    acc: Vec<(f64, f64)>,
    /// Thời điểm bắt đầu bước hiện tại (để tự bỏ qua dấu không chạm được).
    step_t: f64,
}

impl App {
    /// Bắt đầu hiệu chỉnh: hiện lớp phủ trên màn hình đích, các lần chạm sau đó chỉ dùng để đo.
    pub(crate) fn start_calibration(&mut self) {
        for a in self.engine.reset() {
            self.sink.apply(&a);
        }
        self.sink.show_cursor_now();
        self.calib_run = Some(CalibRun { samples: vec![], idx: 0, acc: vec![], step_t: unsafe { CFAbsoluteTimeGetCurrent() } });
        self.show_calib_step();
    }

    pub(crate) fn reset_calibration(&mut self) {
        self.map.set_calib(None);
        self.map.set_extent(None);
    }

    pub(crate) fn is_calibrated(&self) -> bool {
        self.map.calib.is_some()
    }

    pub(crate) fn calib_stale(&self) -> bool {
        self.map.calib_stale()
    }

    fn show_calib_step(&self) {
        let Some(run) = &self.calib_run else { return };
        let b = self.map.bounds;
        let main_h = unsafe { CGDisplayBounds(CGMainDisplayID()).size.height };
        menubar::calib_show(
            (b.origin.x, b.origin.y, b.size.width, b.size.height),
            main_h,
            run.idx,
            CALIB_TARGETS.len(),
            CALIB_TARGETS[run.idx.min(CALIB_TARGETS.len() - 1)],
        );
    }

    /// Hiện kết quả ngay trên màn hình cảm ứng (không dùng hộp thoại modal vì nó hiện ở màn hình khác
    /// và chặn mọi thao tác chạm), tự đóng sau vài giây.
    fn calib_finish(&mut self, text: String) {
        let b = self.map.bounds;
        let main_h = unsafe { CGDisplayBounds(CGMainDisplayID()).size.height };
        menubar::calib_message((b.origin.x, b.origin.y, b.size.width, b.size.height), main_h, &text);
        self.overlay_until = Some(unsafe { CFAbsoluteTimeGetCurrent() } + 3.0);
    }

    fn overlay_geometry(&self) -> ((f64, f64, f64, f64), f64) {
        let b = self.map.bounds;
        let main_h = unsafe { CGDisplayBounds(CGMainDisplayID()).size.height };
        ((b.origin.x, b.origin.y, b.size.width, b.size.height), main_h)
    }

    fn begin_probe(&mut self, probe: Probe) {
        for a in self.engine.reset() {
            self.sink.apply(&a);
        }
        self.sink.show_cursor_now();
        self.calib_run = None;
        let (bounds, main_h) = self.overlay_geometry();
        match &probe {
            Probe::Touch(_) => menubar::overlay_touch_ui(bounds, main_h),
            Probe::Colors { .. } => menubar::overlay_message(bounds, main_h, &tr("Chạm để đổi màu · chạm 3 ngón để thoát")),
        }
        self.probe = Some(probe);
    }

    /// Kiểm tra vùng chạm: rê ngón khắp màn hình, vùng nhận chạm hiện vàng.
    pub(crate) fn start_touch_test(&mut self) {
        if self.probe.is_some() {
            self.stop_probe();
            return;
        }
        self.begin_probe(Probe::Touch(TouchTest {
            canvas: Canvas::new(640, 400),
            last: vec![],
            ext: None,
            dirty: true,
            last_draw: 0.0,
        }));
    }

    /// Kiểm tra màn hình: các màu đặc lần lượt.
    pub(crate) fn start_screen_test(&mut self) {
        if self.probe.is_some() {
            self.stop_probe();
            return;
        }
        self.begin_probe(Probe::Colors { idx: None, down: false });
    }

    fn stop_probe(&mut self) {
        let Some(probe) = self.probe.take() else { return };
        let (bounds, main_h) = self.overlay_geometry();
        let text = match probe {
            Probe::Touch(t) => match t.ext {
                Some(e) if e[1] - e[0] > 0.6 && e[3] - e[2] > 0.6 => {
                    self.map.set_extent(Some(e));
                    self.prefs().save();
                    eprintln!("Vùng nhận chạm: {e:?}");
                    tr("Đã lưu vùng nhận chạm ({info}). Dải sát mép giờ chạm tới được.").replace("{info}", &extent_info(e))
                }
                _ => tr("Chưa đủ dữ liệu: hãy rê ngón tay khắp màn hình rồi thử lại."),
            },
            Probe::Colors { .. } => String::new(),
        };
        if text.is_empty() {
            menubar::calib_close();
        } else {
            menubar::overlay_message(bounds, main_h, &text);
            self.overlay_until = Some(unsafe { CFAbsoluteTimeGetCurrent() } + 3.5);
        }
    }

    /// Nhận một frame cảm ứng khi đang chạy công cụ kiểm tra.
    fn probe_frame(&mut self, tips: &[(u32, f64, f64)]) {
        if tips.len() >= 3 {
            self.stop_probe();
            return;
        }
        let normalized: Vec<(u32, f64, f64, (f64, f64))> = tips
            .iter()
            .map(|&(id, x, y)| {
                let (ox, oy) = self.map.oriented(x, y);
                (id, ox, oy, self.map.normalized(x, y))
            })
            .collect();
        let (bounds, main_h) = self.overlay_geometry();
        let Some(probe) = self.probe.as_mut() else { return };
        match probe {
            Probe::Touch(t) => {
                let mut new_last = vec![];
                for &(id, ox, oy, n) in &normalized {
                    let prev = t.last.iter().find(|l| l.0 == id).and_then(|l| l.1);
                    t.canvas.stroke(prev, Some(n), 4.0);
                    new_last.push((id, Some(n)));
                    let e = t.ext.get_or_insert([ox, ox, oy, oy]);
                    e[0] = e[0].min(ox);
                    e[1] = e[1].max(ox);
                    e[2] = e[2].min(oy);
                    e[3] = e[3].max(oy);
                }
                t.last = new_last;
                t.dirty = !normalized.is_empty() || t.dirty;
            }
            Probe::Colors { idx, down } => {
                if !tips.is_empty() {
                    *down = true;
                } else if *down {
                    *down = false;
                    let next = idx.map(|i| (i + 1) % PROBE_COLORS.len()).unwrap_or(0);
                    *idx = Some(next);
                    let (r, g, b) = PROBE_COLORS[next];
                    menubar::overlay_color(bounds, main_h, r, g, b);
                }
            }
        }
    }

    /// Gọi từ timer: vẽ lại bản đồ vùng chạm (tối đa ~30 lần/giây).
    fn probe_tick(&mut self, now: f64) {
        if let Some(Probe::Touch(t)) = self.probe.as_mut() {
            if t.dirty && now - t.last_draw > 0.033 {
                t.dirty = false;
                t.last_draw = now;
                let info = t
                    .ext
                    .map(|e| tr("Vùng nhận chạm: {info}").replace("{info}", &extent_info(e)))
                    .unwrap_or_default();
                menubar::overlay_touch_update(t.canvas.w, t.canvas.h, &t.canvas.px, &info);
            }
        }
    }

    /// Nhận một frame cảm ứng khi đang hiệu chỉnh. Mỗi lần nhấc ngón = một mẫu đo.
    fn calibration_frame(&mut self, tips: &[(f64, f64)]) {
        let oriented: Vec<(f64, f64)> = tips.iter().map(|&(x, y)| self.map.oriented(x, y)).collect();
        let Some(run) = self.calib_run.as_mut() else { return };
        if oriented.len() >= 2 {
            self.calib_run = None;
            self.calib_finish(tr("Đã hủy hiệu chỉnh."));
            return;
        }
        if let Some(&p) = oriented.first() {
            run.acc.push(p);
            return;
        }
        if run.acc.is_empty() {
            return;
        }
        // Nhấc ngón: lấy trung bình các vị trí của lần chạm làm mẫu.
        let n = run.acc.len() as f64;
        let avg = (run.acc.iter().map(|p| p.0).sum::<f64>() / n, run.acc.iter().map(|p| p.1).sum::<f64>() / n);
        run.acc.clear();
        run.samples.push((avg, CALIB_TARGETS[run.idx]));
        self.calib_advance();
    }

    /// Sang dấu tiếp theo (sau khi đo xong hoặc bỏ qua); hết dấu thì tính kết quả.
    fn calib_advance(&mut self) {
        let Some(run) = self.calib_run.as_mut() else { return };
        run.idx += 1;
        run.step_t = unsafe { CFAbsoluteTimeGetCurrent() };
        run.acc.clear();
        if run.idx < CALIB_TARGETS.len() {
            self.show_calib_step();
            return;
        }
        let samples = std::mem::take(&mut run.samples);
        self.calib_run = None;
        for (i, (raw, tgt)) in samples.iter().enumerate() {
            eprintln!(
                "Hiệu chỉnh mẫu {}: cảm biến=({:.4}, {:.4}) đích=({:.2}, {:.2})",
                i + 1,
                raw.0,
                raw.1,
                tgt.0,
                tgt.1
            );
        }
        match fit_calib(&samples) {
            Some(fit) if fit.max_error < 0.05 => {
                self.map.set_calib(Some(fit.calib.clone()));
                self.prefs().save();
                eprintln!("Hiệu chỉnh: {:?}, sai số lớn nhất {:.3}", fit.calib, fit.max_error);
                self.calib_finish(tr("Đã hiệu chỉnh xong. Bạn thử chạm lại nhé."));
            }
            other => {
                eprintln!(
                    "Hiệu chỉnh thất bại, sai số lớn nhất: {:?}",
                    other.map(|f| f.max_error)
                );
                self.calib_finish(tr("Hiệu chỉnh không thành công: các điểm chạm chưa nhất quán. Hãy thử lại và chạm chính giữa dấu +."));
            }
        }
    }

    /// Gọi từ timer: dấu nào 8 giây không có chạm (vùng cảm biến không nhận) thì bỏ qua để khỏi kẹt.
    fn calib_tick(&mut self, now: f64) {
        let stale = self.calib_run.as_ref().is_some_and(|r| r.acc.is_empty() && now - r.step_t > CALIB_SKIP_SECS);
        if stale {
            if let Some(r) = &self.calib_run {
                eprintln!("Hiệu chỉnh: bỏ qua dấu {} (không nhận được chạm)", r.idx + 1);
            }
            self.calib_advance();
        }
    }

    /// Cài đặt hiện tại (để menu lưu lại).
    pub(crate) fn prefs(&self) -> crate::Prefs {
        let c = &self.engine.cfg;
        crate::Prefs {
            scroll_speed: self.sink.scroll_scale,
            invert_scroll: self.sink.invert_scroll,
            one_finger_scroll: c.one_finger_scroll,
            momentum: c.momentum,
            pinch: c.pinch,
            swipe: c.three_finger_swipe,
            orientation: self.orientation(),
            display: self.display,
            hide_cursor: self.sink.hide_cursor,
            snap: self.snap,
            smart_drag: crate::snap::smart_drag_enabled(),
            keyboard: self.keyboard,
            kbd_size: menubar::kbd_size(),
            swipe_level: self.swipe_level,
            language: menubar::language_mode(),
            calib: self.map.calib_base.as_ref().map(|c| c.to_vec()),
            extent: self.map.extent.map(|e| e.to_vec()),
            calib_size: self.map.calib_size,
        }
    }

    pub(crate) fn device_summary(&self) -> String {
        let b = self.map.bounds;
        let d = if self.devices.is_empty() {
            tr("Chưa thấy màn hình cảm ứng (kiểm tra cáp USB)")
        } else {
            self.devices.join(", ")
        };
        format!(
            "{}\n{}",
            tr("Thiết bị: {d}").replace("{d}", &d),
            tr("Màn hình đích: {w}×{h}")
                .replace("{w}", &b.size.width.to_string())
                .replace("{h}", &b.size.height.to_string())
        )
    }

    pub(crate) fn orientation(&self) -> u8 {
        let m = &self.map;
        match (m.swap_xy, m.flip_x, m.flip_y) {
            (true, true, false) => 1,
            (true, false, true) => 2,
            _ => 0,
        }
    }

    pub(crate) fn set_orientation(&mut self, o: u8) {
        self.map.swap_xy = o != 0;
        self.map.flip_x = o == 1;
        self.map.flip_y = o == 2;
    }

    /// Đổi cỡ giao diện màn hình đích (0 = gốc). Trả về chuỗi rỗng nếu thành công, ngược lại là lỗi.
    pub(crate) fn set_scale(&mut self, level: usize) -> String {
        let Some(id) = target_display_id(self.display) else {
            return tr("Không tìm thấy màn hình đích.");
        };
        match self.scale.set(id, level) {
            Ok(()) => {
                self.scale_level = level;
                if let Some(b) = target_bounds(self.display) {
                    self.map.bounds = b;
            self.map.refresh_calib();
            self.sync_swipe();
                    self.map.refresh_calib();
            self.sync_swipe();
                    self.sync_swipe();
                }
                String::new()
            }
            Err(e) => e,
        }
    }

    pub(crate) fn set_display(&mut self, d: Option<usize>) {
        self.display = d;
        if let Some(b) = target_bounds(d) {
            self.map.bounds = b;
            self.map.refresh_calib();
            self.sync_swipe();
        }
    }

    pub(crate) fn display_choice(&self) -> Option<usize> {
        self.display
    }

    /// Quãng vuốt nhiều ngón theo kích thước màn hình đích và độ nhạy: 7% / 12% / 18% chiều rộng (ngang)
    /// và chiều cao (dọc). Màn hình 2560 rộng: ~180 / ~307 / ~460 điểm.
    pub(crate) fn sync_swipe(&mut self) {
        let f = [0.07, 0.12, 0.18][self.swipe_level.min(2) as usize];
        let b = self.map.bounds;
        self.engine.cfg.swipe_distance_x = b.size.width * f;
        self.engine.cfg.swipe_distance_y = b.size.height * f;
    }

    pub(crate) fn set_swipe_level(&mut self, level: u8) {
        self.swipe_level = level.min(2);
        self.sync_swipe();
    }

    pub(crate) fn toggle_keyboard(&mut self) -> bool {
        self.keyboard = !self.keyboard;
        if !self.keyboard {
            menubar::kbd_hide();
        }
        self.keyboard
    }

    /// Đưa một hành động của engine tới nơi xử lý: ToggleKeyboard là việc của App (cần trạng thái bàn phím),
    /// còn lại tới Sink. Phải dùng cho MỌI đường phát ra hành động (frame cảm ứng và timer `tick`):
    /// cử chỉ giữ 3 ngón do `tick` sinh ra, nếu đưa thẳng vào Sink sẽ bị bỏ rơi.
    pub(crate) fn dispatch(&mut self, a: Action) {
        if a == Action::ToggleKeyboard {
            if self.debug {
                eprintln!("  -> ToggleKeyboard (cử chỉ 3 ngón), bàn phím đang hiện: {}", menubar::kbd_visible());
            }
            self.toggle_keyboard_by_gesture();
        } else {
            self.sink.apply(&a);
        }
    }

    /// Chạm 2 lần bằng 3 ngón: đang hiện thì ẩn (và không tự hiện lại tới khi rời ô nhập), đang ẩn thì hiện.
    fn toggle_keyboard_by_gesture(&mut self) {
        if !self.keyboard {
            return;
        }
        if menubar::kbd_visible() {
            menubar::kbd_hide();
            self.kbd_dismissed = true;
        } else {
            self.show_keyboard_now();
        }
    }

    /// Người dùng đóng bàn phím bằng nút ✕: không tự hiện lại cho tới khi focus rời khỏi ô nhập.
    pub(crate) fn kbd_dismissed_by_user(&mut self) {
        self.kbd_dismissed = true;
    }

    /// Hiện bàn phím ngay (từ menu), bất kể có đang focus ô nhập hay không.
    pub(crate) fn show_keyboard_now(&mut self) {
        self.kbd_dismissed = false;
        let (bounds, main_h) = self.overlay_geometry();
        menubar::kbd_show(bounds, main_h, None);
    }

    /// Sau một cú chạm: nếu đang focus vào ô nhập thì hiện bàn phím, rời ô nhập thì ẩn.
    fn kbd_tick(&mut self, now: f64) {
        let Some((at, tap)) = self.kbd_check else { return };
        if now < at {
            return;
        }
        self.kbd_check = None;
        // Chạm vào chính bàn phím: giữ nguyên, đừng dò (tránh treo AX vào cửa sổ của mình).
        if menubar::point_in_own_windows(tap) {
            return;
        }
        let pid = menubar::frontmost_pid();
        let focus = if pid == Some(std::process::id() as i32) { None } else { crate::snap::focus_info(pid) };
        match focus {
            Some(f) if f.is_text => {
                if self.keyboard && menubar::kbd_visible() {
                    menubar::kbd_set_autocap(f.at_sentence_start == Some(true));
                }
                if self.keyboard && !self.kbd_dismissed && !menubar::kbd_visible() {
                    let (bounds, main_h) = self.overlay_geometry();
                    let avoid = f.frame.map(|(p, s)| (p.x, p.y, s.x, s.y));
                    menubar::kbd_show(bounds, main_h, avoid);
                    menubar::kbd_set_autocap(f.at_sentence_start == Some(true));
                }
            }
            Some(_) => {
                self.kbd_dismissed = false;
                if menubar::kbd_visible() {
                    menubar::kbd_hide();
                }
            }
            None => {}
        }
    }

    pub(crate) fn toggle_smart_drag(&mut self) -> bool {
        let on = !crate::snap::smart_drag_enabled();
        crate::snap::set_smart_drag(on);
        on
    }

    pub(crate) fn toggle_snap(&mut self) -> bool {
        self.snap = !self.snap;
        self.snap
    }

    pub(crate) fn toggle_hide_cursor(&mut self) -> bool {
        self.sink.hide_cursor = !self.sink.hide_cursor;
        if !self.sink.hide_cursor {
            self.sink.show_cursor_now();
        }
        self.sink.hide_cursor
    }

    pub(crate) fn set_scroll_speed(&mut self, v: f64) {
        self.sink.scroll_scale = v;
    }

    pub(crate) fn toggle_invert_scroll(&mut self) -> bool {
        self.sink.invert_scroll = !self.sink.invert_scroll;
        self.sink.invert_scroll
    }

    pub(crate) fn set_paused(&mut self, paused: bool) {
        if paused && !self.paused {
            for a in self.engine.reset() {
                self.sink.apply(&a);
            }
        }
        self.paused = paused;
        if paused {
            self.sink.show_cursor_now();
        }
        eprintln!("{}", if paused { "Tạm dừng." } else { "Tiếp tục." });
    }
}

static APP_PTR: AtomicPtr<App> = AtomicPtr::new(ptr::null_mut());

/// Truy cập App từ callback menu (luôn chạy trên main thread, cùng run loop).
pub(crate) fn with_app<R: Default>(f: impl FnOnce(&mut App) -> R) -> R {
    let p = APP_PTR.load(Ordering::SeqCst);
    if p.is_null() {
        return R::default();
    }
    f(unsafe { &mut *p })
}

/// Yêu cầu thoát: timer sẽ nhả mọi nút đang giữ rồi mới thoát.
pub(crate) fn request_quit() {
    STOP.store(true, Ordering::SeqCst);
}

struct DevCtx {
    name: String,
    layout: TouchLayout,
    asm: FrameAssembler,
    buf: Vec<u8>,
    app: *mut App,
    reports_seen: u64,
    bad_reports: u32,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}

extern "C" fn on_report(
    ctx: *mut c_void,
    result: IOReturn,
    _sender: *mut c_void,
    _rtype: u32,
    _rid: u32,
    report: *mut u8,
    len: CFIndex,
) {
    if ctx.is_null() || report.is_null() || result != kIOReturnSuccess || len <= 0 {
        return;
    }
    let d = unsafe { &mut *(ctx as *mut DevCtx) };
    let app = unsafe { &mut *d.app };
    let data = unsafe { std::slice::from_raw_parts(report, len as usize) };
    d.reports_seen += 1;
    if d.reports_seen == 1 {
        eprintln!("[{}] nhận report đầu tiên ({} byte): {}", d.name, data.len(), hex(data));
    } else if app.debug {
        eprintln!("[{}] raw: {}", d.name, hex(data));
    }
    let Some(parsed) = d.layout.parse_report(data) else {
        if d.bad_reports < 5 {
            d.bad_reports += 1;
            eprintln!(
                "[{}] ⚠️ report không khớp layout (id=0x{:02x}, {} byte, layout có id {:?})",
                d.name,
                data.first().copied().unwrap_or(0),
                data.len(),
                d.layout.reports.keys().collect::<Vec<_>>()
            );
        }
        return;
    };
    if app.debug {
        eprintln!(
            "  parsed: count={:?} {:?}",
            parsed.contact_count,
            parsed.contacts.iter().filter(|c| c.tip).collect::<Vec<_>>()
        );
    }
    let Some(frame) = d.asm.push(parsed) else { return };
    if app.probe.is_some() {
        let tips: Vec<(u32, f64, f64)> = frame.iter().map(|c| (c.id, c.x, c.y)).collect();
        app.probe_frame(&tips);
        return;
    }
    if app.calib_run.is_some() {
        let tips: Vec<(f64, f64)> = frame.iter().map(|c| (c.x, c.y)).collect();
        app.calibration_frame(&tips);
        return;
    }
    if app.paused {
        return;
    }

    let now = unsafe { CFAbsoluteTimeGetCurrent() };
    let touches: Vec<Touch> = frame
        .iter()
        .map(|c| Touch {
            id: c.id,
            p: app.map.to_screen(c.x, c.y),
        })
        .collect();
    if app.debug {
        let s: Vec<String> = touches
            .iter()
            .map(|t| format!("#{}({:.0},{:.0})", t.id, t.p.x, t.p.y))
            .collect();
        eprintln!("[{}] t={:.3} frame: {}", d.name, now, s.join(" "));
    }
    if !touches.is_empty() {
        app.sink.last_touch_t = now;
        app.sink.hide_cursor_now();
        let n = touches.len() as f64;
        app.sink.hint = Some(P {
            x: touches.iter().map(|t| t.p.x).sum::<f64>() / n,
            y: touches.iter().map(|t| t.p.y).sum::<f64>() / n,
        });
    }
    let mut acts = app.engine.update(&touches, now);
    if app.snap {
        // Chạm thông minh: tap trượt nút nhỏ thì hút vào điều khiển gần nhất (vòng dò bán kính 20pt).
        for (i, p) in crate::snap::find_tap_clicks(&acts) {
            // Điểm nằm trong cửa sổ của chính TouchRelay (bàn phím, Cài đặt): dò AX sẽ tự treo tới hết timeout.
            if menubar::point_in_own_windows(p) {
                continue;
            }
            let q = crate::snap::snap_point(p, 20.0);
            if q.dist(p) > 0.5 {
                if app.debug {
                    eprintln!("  snap: ({:.0},{:.0}) -> ({:.0},{:.0})", p.x, p.y, q.x, q.y);
                }
                crate::snap::retarget(&mut acts, i, q);
            }
        }
    }
    if app.keyboard {
        if let Some((_, q)) = crate::snap::find_tap_clicks(&acts).first() {
            app.kbd_check = Some((now + 0.2, *q));
        }
    }
    for a in acts {
        app.dispatch(a);
    }
}

extern "C" fn on_timer(_t: CFRunLoopTimerRef, info: *mut c_void) {
    let app = unsafe { &mut *(info as *mut App) };
    if let Some(t) = app.overlay_until {
        if unsafe { CFAbsoluteTimeGetCurrent() } >= t {
            app.overlay_until = None;
            menubar::calib_close();
        }
    }
    if STOP.load(Ordering::SeqCst) {
        for a in app.engine.reset() {
            app.sink.apply(&a);
        }
        app.sink.flush();
        app.sink.show_cursor_now();
        app.scale.restore();
        eprintln!("Đã thoát.");
        if app.ui {
            menubar::terminate();
        } else {
            unsafe { CFRunLoopStop(CFRunLoopGetCurrent()) };
        }
        return;
    }
    // Mỗi ~1 giây: màn hình rời có thể bật/tắt/đổi vị trí sau khi cảm ứng đã nhận.
    app.ticks = app.ticks.wrapping_add(1);
    if app.ticks % 120 == 0 {
        if let Some(b) = target_bounds(app.display) {
            let old = app.map.bounds;
            if (b.origin.x, b.origin.y, b.size.width, b.size.height)
                != (old.origin.x, old.origin.y, old.size.width, old.size.height)
            {
                app.map.bounds = b;
            app.map.refresh_calib();
            app.sync_swipe();
                app.map.refresh_calib();
            app.sync_swipe();
                app.sync_swipe();
                if app.ui {
                    menubar::rebuild_displays();
                }
                eprintln!(
                    "Màn hình đích đổi: {}x{} tại ({}, {})",
                    b.size.width, b.size.height, b.origin.x, b.origin.y
                );
            }
        }
    }
    let now = unsafe { CFAbsoluteTimeGetCurrent() };
    app.kbd_tick(now);
    app.calib_tick(now);
    app.probe_tick(now);
    app.sink.pump(now);
    app.sink.check_real_pointer(now);
    for a in app.engine.tick(now) {
        app.dispatch(a);
    }
}

extern "C" fn on_signal(_sig: i32) {
    STOP.store(true, Ordering::SeqCst);
}

extern "C" {
    fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
}

// ============================================================
// Liệt kê thiết bị / màn hình
// ============================================================
unsafe fn matching_dict(page: i32, usage: i32) -> CFMutableDictionaryRef {
    let dict = CFDictionaryCreateMutable(
        kCFAllocatorDefault,
        0,
        &kCFTypeDictionaryKeyCallBacks,
        &kCFTypeDictionaryValueCallBacks,
    );
    let (kp, ku) = (CfStr::new("DeviceUsagePage"), CfStr::new("DeviceUsage"));
    let (np, nu) = (cf_number_i32(page), cf_number_i32(usage));
    CFDictionarySetValue(dict, kp.0, np);
    CFDictionarySetValue(dict, ku.0, nu);
    CFRelease(np);
    CFRelease(nu);
    dict
}

unsafe fn copy_devices(page: i32, usage: i32) -> Vec<IOHIDDeviceRef> {
    let m = IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone);
    if m.is_null() {
        return vec![];
    }
    let dict = matching_dict(page, usage);
    IOHIDManagerSetDeviceMatching(m, dict);
    CFRelease(dict as CFTypeRef);

    let set = IOHIDManagerCopyDevices(m);
    if set.is_null() {
        return vec![];
    }
    let n = CFSetGetCount(set).max(0) as usize;
    let mut vals: Vec<*const c_void> = vec![ptr::null(); n];
    CFSetGetValues(set, vals.as_mut_ptr());
    let out = vals
        .into_iter()
        .map(|v| {
            CFRetain(v);
            v as IOHIDDeviceRef
        })
        .collect();
    CFRelease(set);
    // Cố ý giữ manager sống suốt chương trình.
    out
}

unsafe fn displays() -> Vec<(CGDirectDisplayID, CGRect, bool)> {
    let mut ids = [0u32; 16];
    let mut n = 0u32;
    if CGGetActiveDisplayList(16, ids.as_mut_ptr(), &mut n) != 0 {
        return vec![];
    }
    ids[..n as usize]
        .iter()
        .map(|&id| (id, CGDisplayBounds(id), CGDisplayIsBuiltin(id) != 0))
        .collect()
}

unsafe fn describe(dev: IOHIDDeviceRef) -> (String, i64, i64) {
    let name = device_prop_string(dev, "Product").unwrap_or_else(|| "(không tên)".into());
    let vid = device_prop_i64(dev, "VendorID").unwrap_or(0);
    let pid = device_prop_i64(dev, "ProductID").unwrap_or(0);
    (name, vid, pid)
}

unsafe fn layout_of(dev: IOHIDDeviceRef) -> Result<TouchLayout, String> {
    let raw = device_prop_data(dev, "ReportDescriptor").ok_or("không đọc được ReportDescriptor")?;
    let d = hid_desc::parse(&raw)?;
    TouchLayout::from_descriptor(&d).ok_or_else(|| "descriptor không có collection Finger với X/Y".into())
}

fn list() {
    unsafe {
        println!("Màn hình:");
        for (i, (id, b, builtin)) in displays().iter().enumerate() {
            println!(
                "  [{i}] id={id} {}x{} tại ({}, {}){}",
                b.size.width,
                b.size.height,
                b.origin.x,
                b.origin.y,
                if *builtin { "  (màn hình MacBook)" } else { "" }
            );
        }
        println!("\nThiết bị cảm ứng (HID Digitizer / Touch Screen):");
        let devs = copy_devices(PAGE_DIGITIZER as i32, USAGE_TOUCH_SCREEN as i32);
        if devs.is_empty() {
            println!("  (không tìm thấy)");
        }
        for dev in devs {
            let (name, vid, pid) = describe(dev);
            match layout_of(dev) {
                Ok(l) => println!(
                    "  {name}  vid=0x{vid:04x} pid=0x{pid:04x}  slot/report={} report IDs={:?} device-mode={}",
                    l.max_slots(),
                    l.reports.keys().collect::<Vec<_>>(),
                    l.device_mode.is_some()
                ),
                Err(e) => println!("  {name}  vid=0x{vid:04x} pid=0x{pid:04x}  -> {e}"),
            }
        }
    }
}

/// Một số màn hình cần Device Mode = 2 (multi-input) mới gửi multitouch.
/// Đọc-sửa-ghi một feature field (giữ nguyên phần còn lại của report).
unsafe fn write_feature(dev: IOHIDDeviceRef, l: &TouchLayout, f: &hid_desc::Field, value: u32, what: &str) {
    let bits = l
        .feature_report_bits
        .get(&f.report_id)
        .copied()
        .unwrap_or(f.bit_offset + f.bit_size);
    let id_bytes = if l.uses_report_ids { 1 } else { 0 };
    let mut buf = vec![0u8; id_bytes + bits.div_ceil(8) as usize];
    let mut len = buf.len() as CFIndex;
    let _ = IOHIDDeviceGetReport(
        dev,
        kIOHIDReportTypeFeature,
        f.report_id as CFIndex,
        buf.as_mut_ptr(),
        &mut len,
    );
    if l.uses_report_ids {
        buf[0] = f.report_id;
    }
    hid_desc::insert(&mut buf[id_bytes..], f, value as i32);
    let r = IOHIDDeviceSetReport(
        dev,
        kIOHIDReportTypeFeature,
        f.report_id as CFIndex,
        buf.as_ptr(),
        buf.len() as CFIndex,
    );
    eprintln!(
        "   Set {what}={value} (feature report 0x{:02x}) -> IOReturn 0x{:08x}",
        f.report_id, r as u32
    );
}

/// Bật chế độ đa điểm: Device Mode (0x52) = 2 và Contact Count Maximum (0x55) = tối đa.
unsafe fn set_device_mode(dev: IOHIDDeviceRef, l: &TouchLayout) {
    if let Some(f) = &l.device_mode {
        write_feature(dev, l, f, 2, "Device Mode");
    }
    if let Some(f) = &l.contact_count_max {
        write_feature(dev, l, f, f.logical_max.max(1) as u32, "Contact Count Maximum");
    }
}

// ============================================================
// Hot-plug: mở / đóng thiết bị khi cắm / rút
// ============================================================
struct Hot {
    app: *mut App,
    opts: Options,
    rl: CFRunLoopRef,
    /// (vid, pid) các màn hình cảm ứng đang mở.
    open: Vec<(i64, i64)>,
    /// (vid, pid) các interface chuột đã chặn.
    seized: Vec<(i64, i64)>,
}

unsafe fn watch(
    page: i32,
    usage: i32,
    added: IOHIDDeviceCallback,
    removed: Option<IOHIDDeviceCallback>,
    ctx: *mut c_void,
    rl: CFRunLoopRef,
) {
    let m = IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone);
    if m.is_null() {
        return;
    }
    let dict = matching_dict(page, usage);
    IOHIDManagerSetDeviceMatching(m, dict);
    CFRelease(dict as CFTypeRef);
    IOHIDManagerRegisterDeviceMatchingCallback(m, added, ctx);
    if let Some(r) = removed {
        IOHIDManagerRegisterDeviceRemovalCallback(m, r, ctx);
    }
    IOHIDManagerScheduleWithRunLoop(m, rl, kCFRunLoopCommonModes);
    // Cố ý giữ manager sống suốt chương trình.
}

/// Nhãn các màn hình đang bật, dùng cho menu "Màn hình đích".
pub(crate) fn display_labels() -> Vec<String> {
    unsafe { displays() }
        .iter()
        .enumerate()
        .map(|(i, (_, b, builtin))| {
            format!(
                "{} {}: {}×{}{}",
                tr("Màn hình"),
                i + 1,
                b.size.width,
                b.size.height,
                if *builtin { tr(" (MacBook)") } else { String::new() }
            )
        })
        .collect()
}

fn target_display_id(display: Option<usize>) -> Option<CGDirectDisplayID> {
    let ds = unsafe { displays() };
    match display {
        Some(i) => ds.get(i).copied(),
        None => ds.iter().find(|d| !d.2).or(ds.first()).copied(),
    }
    .map(|d| d.0)
}

fn target_bounds(display: Option<usize>) -> Option<CGRect> {
    let ds = unsafe { displays() };
    let t = match display {
        Some(i) => ds.get(i).copied(),
        None => ds.iter().find(|d| !d.2).or(ds.first()).copied(),
    };
    t.map(|d| d.1)
}

extern "C" fn on_touch_added(ctx: *mut c_void, _r: IOReturn, _s: *mut c_void, dev: IOHIDDeviceRef) {
    unsafe {
        let h = &mut *(ctx as *mut Hot);
        let opts = h.opts.clone();
        let (name, vid, pid) = describe(dev);
        if opts.vid.is_some_and(|v| v != vid) || opts.pid.is_some_and(|p| p != pid) {
            return;
        }
        let layout = match layout_of(dev) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Bỏ qua {name}: {e}");
                return;
            }
        };
        let open_opts = if opts.no_seize {
            kIOHIDOptionsTypeNone
        } else {
            kIOHIDOptionsTypeSeizeDevice
        };
        let r = IOHIDDeviceOpen(dev, open_opts);
        if r != kIOReturnSuccess {
            eprintln!(
                "Không mở được {name} (IOReturn 0x{:08x}). Có app khác đang chiếm thiết bị?",
                r as u32
            );
            return;
        }
        if opts.debug {
            eprintln!("Layout {name}: {:#?}", layout.reports);
        }
        if let Some(raw) = device_prop_data(dev, "ReportDescriptor") {
            eprintln!("   ReportDescriptor ({} byte): {}", raw.len(), hex(&raw));
        }
        set_device_mode(dev, &layout);

        let max_in = device_prop_i64(dev, "MaxInputReportSize").unwrap_or(64).max(8) as usize;
        eprintln!(
            "✅ {name} (vid=0x{vid:04x} pid=0x{pid:04x}) – tối đa {} ngón/report",
            layout.max_slots()
        );
        let dctx = Box::into_raw(Box::new(DevCtx {
            name: name.clone(),
            layout,
            asm: FrameAssembler::default(),
            reports_seen: 0,
            bad_reports: 0,
            buf: vec![0u8; max_in],
            app: h.app,
        }));
        IOHIDDeviceScheduleWithRunLoop(dev, h.rl, kCFRunLoopCommonModes);
        IOHIDDeviceRegisterInputReportCallback(
            dev,
            (*dctx).buf.as_mut_ptr(),
            (*dctx).buf.len() as CFIndex,
            on_report,
            dctx as *mut c_void,
        );
        h.open.push((vid, pid));

        // Màn hình đích có thể vừa được cắm cùng lúc: tính lại.
        let app = &mut *h.app;
        if let Some(b) = target_bounds(app.display) {
            app.map.bounds = b;
            app.map.refresh_calib();
            app.sync_swipe();
            eprintln!(
                "Màn hình đích: {}x{} tại ({}, {})",
                b.size.width, b.size.height, b.origin.x, b.origin.y
            );
        }
        app.devices.push(name);
        if app.ui {
            menubar::set_devices(&app.devices);
            menubar::rebuild_displays();
        }

        // Chiếm luôn interface "chuột" ảo của cùng màn hình (nếu đã xuất hiện).
        if !opts.no_seize && !opts.keep_mouse_interface {
            for m in copy_devices(0x01, 0x02) {
                seize_mouse(h, m);
            }
        }
    }
}

unsafe fn seize_mouse(h: &mut Hot, dev: IOHIDDeviceRef) {
    let (name, vid, pid) = describe(dev);
    if !h.open.contains(&(vid, pid)) || h.seized.contains(&(vid, pid)) {
        return;
    }
    let r = IOHIDDeviceOpen(dev, kIOHIDOptionsTypeSeizeDevice);
    if r == kIOReturnSuccess {
        h.seized.push((vid, pid));
        eprintln!("   (đã chặn interface chuột giả lập: {name})");
    } else {
        eprintln!("   (KHÔNG chặn được interface chuột {name}: IOReturn 0x{:08x})", r as u32);
    }
}

extern "C" fn on_mouse_added(ctx: *mut c_void, _r: IOReturn, _s: *mut c_void, dev: IOHIDDeviceRef) {
    unsafe { seize_mouse(&mut *(ctx as *mut Hot), dev) }
}

extern "C" fn on_touch_removed(ctx: *mut c_void, _r: IOReturn, _s: *mut c_void, dev: IOHIDDeviceRef) {
    unsafe {
        let h = &mut *(ctx as *mut Hot);
        let (name, vid, pid) = describe(dev);
        if let Some(i) = h.open.iter().position(|&x| x == (vid, pid)) {
            h.open.remove(i);
        }
        h.seized.retain(|&x| x != (vid, pid));
        eprintln!("⏏️  Đã rút {name}");
        let app = &mut *h.app;
        // Nhả mọi nút/cử chỉ đang dở.
        for a in app.engine.reset() {
            app.sink.apply(&a);
        }
        app.sink.show_cursor_now();
        if let Some(i) = app.devices.iter().position(|n| *n == name) {
            app.devices.remove(i);
        }
        if app.ui {
            menubar::set_devices(&app.devices);
        }
    }
}

// ============================================================
// Entry
// ============================================================
pub fn run(opts: Options) -> i32 {
    if opts.list {
        list();
        return 0;
    }
    let ui = opts.app_mode;
    let who = if ui {
        crate::i18n::APP_NAME
    } else {
        "Terminal (hoặc app chạy touchrelay)"
    };
    // Báo lỗi: ra stderr (tiếng Việt, cho nhà phát triển), và hiện hộp thoại theo ngôn ngữ hệ thống nếu đang chạy dạng .app.
    let fail = |title: &str, text: &str, settings_url: Option<&str>| -> i32 {
        let plain = text.replace("{who}", who).replace("{app}", crate::i18n::APP_NAME);
        eprintln!("⚠️  {title}\n   {plain}");
        if ui {
            let body = tr(text).replace("{who}", who);
            match settings_url {
                Some(u) => {
                    if menubar::alert_buttons(&tr(title), &body, &[&tr("Mở cài đặt"), &tr("Đóng")]) == 0 {
                        menubar::open_url(u);
                    }
                }
                None => menubar::alert(&tr(title), &body),
            }
        }
        1
    };
    menubar::set_language_mode(opts.language); // trước mọi hộp thoại, kể cả thông báo thiếu quyền
    if ui {
        menubar::init_app();
    }
    unsafe {
        // ---- Quyền ----
        if !IOHIDRequestAccess(kIOHIDRequestTypeListenEvent) {
            return fail(
                "Cần quyền Input Monitoring",
                "Bấm \"Mở cài đặt\", bật công tắc cho {who} trong mục Input Monitoring, rồi mở lại {app}.",
                Some("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"),
            );
        }
        if !CGPreflightPostEventAccess() {
            CGRequestPostEventAccess();
            return fail(
                "Cần quyền Accessibility",
                "Bấm \"Mở cài đặt\", bật công tắc cho {who} trong mục Accessibility, rồi mở lại {app}.",
                Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"),
            );
        }

        // ---- Chọn màn hình đích ----
        let ds = displays();
        let target = match opts.display {
            Some(i) => ds.get(i).copied(),
            None => ds.iter().find(|d| !d.2).or(ds.first()).copied(),
        };
        let Some((did, bounds, _)) = target else {
            return fail(
                "Không tìm thấy màn hình đích",
                "Kiểm tra lại tuỳ chọn --display (dùng `touchrelay --list` để xem danh sách).",
                None,
            );
        };
        eprintln!(
            "Màn hình đích: id={did} {}x{} tại ({}, {})",
            bounds.size.width, bounds.size.height, bounds.origin.x, bounds.origin.y
        );

        let app = Box::into_raw(Box::new(App {
            engine: Engine::new(opts.gesture.clone()),
            sink: Sink {
                rem: (0.0, 0.0),
                scroll_scale: opts.scroll_scale,
                invert_scroll: opts.invert_scroll,
                debug: opts.debug,
                hint: None,
                queue: Default::default(),
                last_due: 0.0,
                move_due: 0.0,
                down_due: 0.0,
                prev_move: false,
                hide_cursor: opts.hide_cursor,
                cursor_hidden: false,
                last_pos: None,
                last_touch_t: 0.0,
            },
            map: Mapper {
                bounds,
                swap_xy: opts.swap_xy,
                flip_x: opts.flip_x,
                flip_y: opts.flip_y,
                calib_base: opts.calib.as_deref().and_then(Calib::from_slice),
                extent: opts.extent.as_deref().and_then(|e| <[f64; 4]>::try_from(e).ok()),
                calib_size: opts.calib_size,
                calib: None,
            },
            debug: opts.debug,
            paused: false,
            ui,
            devices: vec![],
            display: opts.display,
            ticks: 0,
            scale: crate::display::ScaleState::new(),
            scale_level: 0,
            snap: opts.snap,
            keyboard: opts.keyboard,
            swipe_level: opts.swipe_level,
            kbd_dismissed: false,
            kbd_check: None,
            probe: None,
            calib_run: None,
            overlay_until: None,
        }));
        APP_PTR.store(app, Ordering::SeqCst);
        crate::snap::set_smart_drag(opts.smart_drag);
        menubar::kbd_set_size(opts.kbd_size);
        (*app).engine.intent_probe = Some(Box::new(|p| {
            if !crate::snap::smart_drag_enabled() {
                None
            } else if menubar::point_in_own_windows(p) {
                Some(true) // kéo cửa sổ của mình (thanh kéo bàn phím...), không dò AX
            } else {
                crate::snap::drag_intent(p)
            }
        }));
        (*app).map.refresh_calib();
        (*app).sync_swipe();

        // ---- Theo dõi thiết bị cảm ứng (hỗ trợ cắm/rút nóng) ----
        // Dùng common modes để vẫn nhận cảm ứng khi đang mở menu.
        let rl = CFRunLoopGetCurrent();
        let hot = Box::into_raw(Box::new(Hot {
            app,
            opts: opts.clone(),
            rl,
            open: vec![],
            seized: vec![],
        })) as *mut c_void;
        watch(PAGE_DIGITIZER as i32, USAGE_TOUCH_SCREEN as i32, on_touch_added, Some(on_touch_removed), hot, rl);
        if !opts.no_seize && !opts.keep_mouse_interface {
            // Interface "chuột" ảo có thể xuất hiện sau (hoặc trước) interface cảm ứng.
            watch(0x01, 0x02, on_mouse_added, None, hot, rl);
        }
        eprintln!("Đang chờ màn hình cảm ứng… (cắm/rút lúc nào cũng được)");

        // ---- Timer 120Hz cho long-press & quán tính ----
        let mut tctx = CFRunLoopTimerContext {
            version: 0,
            info: app as *mut c_void,
            retain: None,
            release: None,
            copyDescription: None,
        };
        let timer = CFRunLoopTimerCreate(
            kCFAllocatorDefault,
            CFAbsoluteTimeGetCurrent() + 0.01,
            1.0 / 120.0,
            0,
            0,
            on_timer,
            &mut tctx,
        );
        CFRunLoopAddTimer(rl, timer, kCFRunLoopCommonModes);

        signal(2, on_signal); // SIGINT
        signal(15, on_signal); // SIGTERM

        if ui {
            menubar::install(&with_app(|a| a.prefs()));
            menubar::set_debug_ui(opts.debug);
            if opts.open_settings {
                menubar::open_settings();
                if let Some(n) = opts.settings_page {
                    menubar::select_page(n);
                }
                if let Some(path) = &opts.snapshot {
                    menubar::schedule_snapshot(path, 0.8);
                }
            }
            if opts.show_keyboard {
                with_app(|a| a.show_keyboard_now());
            }
            eprintln!("Đang chạy (menu bar).");
            menubar::run_app();
        } else {
            eprintln!("Đang chạy. Ctrl+C để thoát.");
            CFRunLoopRun();
        }
    }
    0
}
