//! Đa ngôn ngữ: khoá là câu tiếng Việt, bản dịch tiếng Anh nằm trong bảng EN.
//! Ngôn ngữ theo hệ thống (macOS "Preferred Languages"): bắt đầu bằng "vi" thì giữ tiếng Việt,
//! còn lại dùng tiếng Anh. Chuỗi không có trong bảng sẽ giữ nguyên.

use std::sync::atomic::{AtomicBool, Ordering};

/// Tên hiển thị của app (đổi ở một chỗ khi đổi tên).
pub const APP_NAME: &str = "TouchRelay";
pub const APP_TAGLINE: &str = "Touch Relay for Mac";
pub const CONTACT_EMAIL: &str = "duongtx1505@icloud.com";

static VI: AtomicBool = AtomicBool::new(true);

pub fn set_vi(v: bool) {
    VI.store(v, Ordering::SeqCst);
}

pub fn is_vi() -> bool {
    VI.load(Ordering::SeqCst)
}

/// Dịch một chuỗi; `{app}` được thay bằng tên app.
pub fn tr(vi: &str) -> String {
    let s = if is_vi() {
        vi
    } else {
        EN.iter().find(|(k, _)| *k == vi).map(|(_, e)| *e).unwrap_or(vi)
    };
    s.replace("{app}", APP_NAME)
}

pub const EN: &[(&str, &str)] = &[
    // --- menu ---
    ("●  Đang hoạt động", "●  Active"),
    ("⏸  Đang tạm dừng", "⏸  Paused"),
    ("    Chưa thấy màn hình cảm ứng – hãy cắm cáp USB", "    No touchscreen found – plug in its USB cable"),
    ("Cài đặt…", "Settings…"),
    ("Tạm dừng", "Pause"),
    ("Tốc độ cuộn", "Scroll speed"),
    ("Rất chậm", "Very slow"),
    ("Chậm", "Slow"),
    ("Vừa", "Medium"),
    ("Nhanh (mặc định)", "Fast (default)"),
    ("Nhanh", "Fast"),
    ("Rất nhanh", "Very fast"),
    ("Đảo chiều cuộn", "Reverse scroll direction"),
    ("Ẩn con trỏ khi chạm", "Hide pointer while touching"),
    ("Hướng màn hình", "Screen orientation"),
    ("Ngang", "Landscape"),
    ("Dọc (xoay phải 90°)", "Portrait (rotated right)"),
    ("Dọc (xoay trái 90°)", "Portrait (rotated left)"),
    ("Cỡ giao diện", "Interface size"),
    ("Gốc", "Native"),
    ("Lớn hơn (1.3×)", "Larger (1.3×)"),
    ("Lớn (1.6×)", "Large (1.6×)"),
    ("Rất lớn (2×, nét)", "Extra large (2×, sharp)"),
    ("Màn hình đích", "Target display"),
    ("Tự động (màn hình rời)", "Automatic (external display)"),
    ("Màn hình", "Display"),
    (" (MacBook)", " (built-in)"),
    ("Cử chỉ", "Gestures"),
    ("Cuộn quán tính", "Momentum scrolling"),
    ("Chụm / xoè để phóng to", "Pinch to zoom"),
    ("Vuốt 3 ngón", "Three-finger swipe"),
    ("Kéo 1 ngón để cuộn (kiểu iPad)", "One-finger drag to scroll (iPad style)"),
    ("Mở cùng macOS", "Open at login"),
    ("Hướng dẫn cử chỉ…", "Gesture guide…"),
    ("Khắc phục sự cố…", "Troubleshooting…"),
    ("Nâng cao", "Advanced"),
    ("Sửa tuỳ chọn dòng lệnh…", "Edit command-line options…"),
    ("Xem nhật ký", "Show log"),
    ("Giới thiệu {app}…", "About {app}…"),
    ("Thoát {app}", "Quit {app}"),
    // --- cửa sổ cài đặt ---
    ("{app} – Cài đặt", "{app} – Settings"),
    ("Cuộn & cử chỉ", "Scrolling & gestures"),
    ("Màn hình & chung", "Display & general"),
    ("Chụm / xoè để zoom", "Pinch to zoom"),
    ("Kéo 1 ngón để cuộn", "One-finger scroll"),
    ("Cỡ giao diện (màn hình cảm ứng)", "Interface size (touchscreen)"),
    ("Hướng dẫn", "Gesture guide"),
    ("Sự cố…", "Troubleshoot…"),
    ("Giới thiệu", "About"),
    ("Đóng", "Close"),
    ("Cuộn nhanh hoặc chậm quá? Chỉnh ở \"Cài đặt…\" → Tốc độ cuộn.", "Scrolling too fast or too slow? Adjust it in \"Settings…\" → Scroll speed."),
    // --- mô tả cài đặt ---
    ("Tốc độ nội dung trượt khi bạn kéo ngón tay.", "How fast content moves as you drag your finger."),
    (
        "Mặc định nội dung đi theo ngón tay (kiểu iPad). Bật để đảo hướng.",
        "By default content follows your finger (iPad style). Turn on to reverse it.",
    ),
    (
        "Nội dung tiếp tục trượt và giảm dần sau khi nhấc ngón.",
        "Content keeps gliding and slows down after you lift your finger.",
    ),
    (
        "Chụm hoặc xoè 2 ngón để phóng to/thu nhỏ (Safari, Preview, Maps…).",
        "Pinch with two fingers to zoom in or out (Safari, Preview, Maps…).",
    ),
    (
        "Vuốt 3 ngón để chuyển Space, mở Mission Control và App Exposé.",
        "Swipe with three fingers to switch Spaces or open Mission Control and App Exposé.",
    ),
    (
        "Kéo 1 ngón để cuộn như iPad. Giữ yên ~0.3s rồi kéo để dùng như chuột (thanh trượt, chọn chữ).",
        "Drag with one finger to scroll like an iPad. Hold still for ~0.3s, then drag to act like a mouse (sliders, text selection).",
    ),
    (
        "Chọn nếu màn hình cảm ứng được xoay dọc, để vị trí chạm khớp đúng.",
        "Choose this if the touchscreen is rotated, so touches land in the right place.",
    ),
    (
        "Phóng to giao diện hệ thống để nút dễ chạm hơn. Chỉ mức 2× là sắc nét; 1.3× và 1.6× có thể hơi mờ do màn hình không có độ phân giải trung gian. Tự khôi phục khi thoát.",
        "Enlarge the system interface so buttons are easier to tap. Only 2× is sharp; 1.3× and 1.6× can look soft because the display has no intermediate resolution. Restores automatically when you quit.",
    ),
    (
        "Màn hình nhận thao tác chạm. Tự động = màn hình rời đầu tiên.",
        "The display that receives touches. Automatic picks the first external display.",
    ),
    (
        "Ẩn con trỏ khi dùng cảm ứng, hiện lại khi bạn dùng chuột hoặc trackpad.",
        "Hide the pointer while using touch; it returns when you use a mouse or trackpad.",
    ),
    ("Tự chạy khi đăng nhập macOS.", "Start automatically when you log in to macOS."),
    (
        "Tạm ngừng xử lý cảm ứng (chạm sẽ không có tác dụng).",
        "Temporarily stop handling touch (touches will do nothing).",
    ),
    // --- hộp thoại ---
    ("Cử chỉ trên màn hình cảm ứng", "Touchscreen gestures"),
    (
        "☝︎  Chạm 1 ngón: click\n☝︎  Chạm 2 lần nhanh: double-click\n☝︎  Kéo 1 ngón: cuộn trang\n☝︎  Giữ yên 1 ngón ~0.3s rồi kéo: kéo chuột (thanh trượt, chọn chữ, kéo cửa sổ)\n✌︎  Chạm 2 ngón: click phải\n✌︎  Kéo 2 ngón: cuộn (có quán tính)\n✌︎  Chụm / xoè 2 ngón: phóng to, thu nhỏ\n✋  Vuốt 3 ngón trái/phải: chuyển Space\n✋  Vuốt 3 ngón lên/xuống: Mission Control / App Exposé\n🖐  Chạm 2 lần bằng 3 ngón, hoặc giữ yên 3 ngón 3 giây: hiện/ẩn bàn phím ảo\n\n{tip}",
        "☝︎  Tap with 1 finger: click\n☝︎  Double tap: double-click\n☝︎  Drag with 1 finger: scroll\n☝︎  Hold 1 finger still for ~0.3s, then drag: mouse drag (sliders, text selection, windows)\n✌︎  Tap with 2 fingers: right-click\n✌︎  Drag with 2 fingers: scroll (with momentum)\n✌︎  Pinch / spread: zoom out / in\n✋  Swipe 3 fingers left/right: switch Space\n✋  Swipe 3 fingers up/down: Mission Control / App Exposé\n🖐  Tap twice with 3 fingers, or hold them still for 3 seconds: show/hide the on-screen keyboard\n\n{tip}",
    ),
    ("Khắc phục sự cố", "Troubleshooting"),
    ("Đang hoạt động", "Active"),
    ("Đang tạm dừng", "Paused"),
    (
        "Trạng thái: {state}\n{devices}\n\nNếu chạm không có tác dụng:\n1. Kiểm tra cáp USB của màn hình cảm ứng.\n2. Bật Input Monitoring và Accessibility cho {app}.\n3. Thoát rồi mở lại {app}.",
        "Status: {state}\n{devices}\n\nIf touch does nothing:\n1. Check the touchscreen's USB cable.\n2. Turn on Input Monitoring and Accessibility for {app}.\n3. Quit and reopen {app}.",
    ),
    ("Mở cài đặt quyền", "Open permission settings"),
    ("Thiết bị: {d}", "Device: {d}"),
    ("Chưa thấy màn hình cảm ứng (kiểm tra cáp USB)", "No touchscreen found (check the USB cable)"),
    ("Màn hình đích: {w}×{h}", "Target display: {w}×{h}"),
    ("Không đổi được \"Mở cùng macOS\"", "Couldn't change \"Open at login\""),
    (
        "Hãy chép {app}.app vào thư mục Applications rồi thử lại. Có thể bật thủ công trong System Settings → General → Login Items.",
        "Copy {app}.app to the Applications folder and try again. You can also enable it manually in System Settings → General → Login Items.",
    ),
    ("Không đổi được cỡ giao diện", "Couldn't change the interface size"),
    ("Không tìm thấy màn hình đích.", "Target display not found."),
    ("không bắt đầu được cấu hình màn hình", "couldn't start the display configuration"),
    ("không đặt được chế độ hiển thị", "couldn't set the display mode"),
    ("không áp dụng được chế độ hiển thị", "couldn't apply the display mode"),
    ("không đọc được chế độ hiện tại", "couldn't read the current display mode"),
    ("không liệt kê được chế độ hiển thị", "couldn't list display modes"),
    ("không có chế độ phù hợp", "no suitable display mode"),
    // --- quyền & lỗi khởi động ---
    ("Cần quyền Input Monitoring", "Input Monitoring permission needed"),
    (
        "Bấm \"Mở cài đặt\", bật công tắc cho {who} trong mục Input Monitoring, rồi mở lại {app}.",
        "Click \"Open Settings\", turn on {who} under Input Monitoring, then reopen {app}.",
    ),
    ("Cần quyền Accessibility", "Accessibility permission needed"),
    (
        "Bấm \"Mở cài đặt\", bật công tắc cho {who} trong mục Accessibility, rồi mở lại {app}.",
        "Click \"Open Settings\", turn on {who} under Accessibility, then reopen {app}.",
    ),
    ("Mở cài đặt", "Open Settings"),
    ("Không tìm thấy màn hình đích", "Target display not found"),
    (
        "Kiểm tra lại tuỳ chọn --display (dùng `touchrelay --list` để xem danh sách).",
        "Check the --display option (use `touchrelay --list` to see the list).",
    ),
    ("File tuỳ chọn có lỗi", "Options file has an error"),
    // --- giới thiệu ---
    (
        "Biến màn hình cảm ứng USB rời thành thao tác native trên macOS: chạm, cuộn, pinch và vuốt nhiều ngón.",
        "Turns an external USB touchscreen into native macOS touch: tap, scroll, pinch and multi-finger swipes.",
    ),
    ("Liên hệ", "Contact"),
    ("Chung", "General"),
    ("Bàn phím", "Keyboard"),
    ("Tự hiện khi chạm vào ô nhập", "Show automatically when tapping a text field"),
    ("Chạm 2 lần bằng 3 ngón, hoặc giữ yên 3 ngón 3 giây, để hiện/ẩn bàn phím bất cứ lúc nào.", "Tap twice with 3 fingers, or hold 3 fingers still for 3 seconds, to show or hide the keyboard at any time."),
    ("Ngôn ngữ", "Language"),
    ("Theo hệ thống", "System default"),
    ("Chọn ngôn ngữ của TouchRelay, hoặc theo ngôn ngữ của hệ điều hành.", "Choose TouchRelay's language, or follow the operating system's."),
    ("Độ nhạy vuốt nhiều ngón", "Multi-finger swipe sensitivity"),
    ("Quãng phải vuốt bằng 3–4 ngón để chuyển desktop. Chọn \"Khó\" nếu hay bị chuyển nhầm khi nhích tay.", "How far you must swipe with 3–3 fingers to switch desktops. Choose \"Hard\" if small nudges switch desktops by mistake."),
    ("Nhạy", "Sensitive"),
    ("Khó (vuốt xa hơn)", "Hard (swipe farther)"),
    ("Cỡ bàn phím", "Keyboard size"),
    ("Chọn kích thước phím cho vừa ngón tay và màn hình.", "Choose a key size that suits your fingers and your screen."),
    ("Nhỏ", "Small"),
    ("Vừa (mặc định)", "Medium (default)"),
    ("Lớn", "Large"),
    ("Bàn phím ảo", "On-screen keyboard"),
    ("Hiện bàn phím ảo", "Show on-screen keyboard"),
    ("Tự hiện bàn phím nổi khi bạn chạm vào ô nhập văn bản (cần quyền Accessibility). Chạm 2 lần bằng 3 ngón (hoặc giữ yên 3 ngón 3 giây) để hiện/ẩn thủ công. Kéo thanh trên cùng để di chuyển; gõ tiếng Việt dùng bộ gõ của hệ thống.", "Shows a floating keyboard when you tap a text field (needs the Accessibility permission). Tap twice with 3 fingers (or hold 3 fingers still for 3 seconds) to show or hide it manually. Drag the top bar to move it; Vietnamese typing uses the system input method."),
    ("Kéo thông minh", "Smart drag"),
    ("Kéo 1 ngón tự chọn: kéo chuột khi chạm thanh trượt, thanh cuộn, thanh tiêu đề cửa sổ hoặc ô nhập; cuộn khi chạm nội dung cuộn được (cần quyền Accessibility).", "One-finger drag chooses for you: mouse drag when you start on a slider, scroll bar, window title bar or text field; scroll when you start on scrollable content (needs the Accessibility permission)."),
    ("Công cụ", "Tools"),
    ("Kiểm tra vùng chạm", "Touch area test"),
    ("Kiểm tra vùng chạm…", "Touch area test…"),
    ("Rê ngón tay khắp màn hình: vùng nhận chạm hiện màu vàng, vùng tối là chỗ cảm biến không nhận. Kết quả dùng để kéo vùng chạm ra sát mép.", "Sweep a finger across the screen: areas that register touch turn yellow, dark areas are where the sensor doesn't. The result is used to stretch the touch area to the screen edges."),
    ("Kiểm tra màn hình", "Screen test"),
    ("Kiểm tra màn hình…", "Screen test…"),
    ("Hiện lần lượt các màu đặc để tìm điểm ảnh chết hoặc sáng không đều. Chạm để đổi màu.", "Shows solid colors in turn to spot dead pixels or uneven brightness. Tap to change the color."),
    ("Rê ngón tay khắp màn hình, kể cả sát bốn mép", "Sweep your finger across the whole screen, including all four edges"),
    ("Vàng = nhận chạm · chạm 3 ngón để kết thúc", "Yellow = touch detected · tap with 3 fingers to finish"),
    ("Vùng nhận chạm: {info}", "Touch range: {info}"),
    ("Đã lưu vùng nhận chạm ({info}). Dải sát mép giờ chạm tới được.", "Touch range saved ({info}). The strip along the edges is now reachable."),
    ("Chưa đủ dữ liệu: hãy rê ngón tay khắp màn hình rồi thử lại.", "Not enough data: sweep your finger across the whole screen and try again."),
    ("Chạm để đổi màu · chạm 3 ngón để thoát", "Tap to change the color · tap with 3 fingers to exit"),
    ("Chạm thông minh", "Smart tap"),
    ("Khi chạm trượt một nút nhỏ, tự chọn nút gần nhất trong vòng ~20 điểm (cần quyền Accessibility).", "When a tap just misses a small button, pick the nearest control within ~20 points (needs the Accessibility permission)."),
    ("Hiệu chỉnh vị trí chạm", "Touch calibration"),
    ("Hiệu chỉnh vị trí chạm…", "Touch calibration…"),
    ("Nếu chạm bị lệch, chạm lần lượt vào 9 dấu + trên màn hình để căn chỉnh lại.", "If touches land off target, tap the 9 + marks on the screen one by one to realign."),
    ("Bắt đầu", "Start"),
    ("Đặt lại hiệu chỉnh", "Reset calibration"),
    ("Đang dùng hiệu chỉnh riêng. Đặt lại để về mặc định.", "A custom calibration is in use. Reset to go back to the default."),
    ("Chưa hiệu chỉnh, đang dùng vị trí mặc định của cảm biến.", "Not calibrated; using the sensor's default positions."),
    ("Đặt lại", "Reset"),
    ("Hiệu chỉnh cũ được lưu cho độ phân giải khác nên đang tắt. Hãy hiệu chỉnh lại hoặc đặt lại.", "The saved calibration was made for a different resolution, so it is switched off. Recalibrate or reset it."),
    ("Chạm vào dấu +", "Tap the + mark"),
    ("Bước {n}/{t} · chạm 2 ngón để hủy · không chạm được thì đợi 8 giây để bỏ qua", "Step {n}/{t} · tap with 2 fingers to cancel · if you can't tap it, wait 8 seconds to skip"),
    ("Đã hủy hiệu chỉnh.", "Calibration cancelled."),
    ("Đã hiệu chỉnh xong. Bạn thử chạm lại nhé.", "Calibration complete. Try tapping again."),
    ("Hiệu chỉnh không thành công: các điểm chạm chưa nhất quán. Hãy thử lại và chạm chính giữa dấu +.", "Calibration failed: the taps were inconsistent. Try again and tap the exact centre of each + mark."),
    ("Cuộn", "Scrolling"),
    ("Phiên bản", "Version"),
    ("Gửi email", "Send email"),
    ("Mở", "Open"),
    ("Xem các cử chỉ được hỗ trợ.", "See the supported gestures."),
    ("Kiểm tra trạng thái thiết bị và quyền của ứng dụng.", "Check the device status and the app's permissions."),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_and_translated() {
        for (i, (k, e)) in EN.iter().enumerate() {
            assert!(!e.is_empty(), "bản dịch rỗng: {k}");
            assert!(EN.iter().skip(i + 1).all(|(k2, _)| k2 != k), "khoá trùng: {k}");
        }
    }

    #[test]
    fn english_and_vietnamese() {
        set_vi(false);
        assert_eq!(tr("Tạm dừng"), "Pause");
        assert_eq!(tr("Thoát {app}"), format!("Quit {APP_NAME}"));
        assert_eq!(tr("không có trong bảng"), "không có trong bảng");
        set_vi(true);
        assert_eq!(tr("Tạm dừng"), "Tạm dừng");
        assert_eq!(tr("Thoát {app}"), format!("Thoát {APP_NAME}"));
    }
}
