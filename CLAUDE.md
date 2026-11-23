# touchrelay: ghi chú bàn giao cho Claude Code

Trả lời người dùng bằng **tiếng Việt**.

## Mục tiêu
App Rust cho macOS biến màn hình cảm ứng USB rời (chuẩn HID Digitizer) thành thao tác native: tap = click, giữ = click phải, kéo 1 ngón = cuộn (mặc định) hoặc kéo chuột, 2 ngón = cuộn pixel có phase + quán tính, pinch = magnify, vuốt 3 ngón = Ctrl+mũi tên. Không dùng crate ngoài; IOKit/CoreFoundation/CoreGraphics/ObjC runtime gọi qua FFI thuần.

## Trạng thái (đã chạy được trên máy thật)
Chạm, cuộn 1 ngón, hot-plug (rút/cắm cáp) đều hoạt động với ILITEK-TP. Người dùng cuối dùng `TouchRelay.app` (menu bar), không phải đọc log.

### Máy của người dùng
- MacBook Pro Apple Silicon (arm64), zsh, Rust qua rustup (`. "$HOME/.cargo/env"` nếu thiếu `cargo`).
- Project: `~/Projects/touchmac`. App cài ở `/Applications/TouchRelay.app`, bundle id `local.touchrelay`.
- Màn hình MacBook id=1 1800x1169 tại (0,0); màn hình rời id=3 2560x1600. **Vị trí màn hình rời đổi theo lần sắp xếp** (từng là (1800,0), hiện (-2560,-244)); app luôn đọc từ CoreGraphics lúc chạy và theo dõi thay đổi mỗi giây.
- Thiết bị: **ILITEK-TP vid=0x222a pid=0x0001**, 10 slot/report, report ID 4, kèm 1 interface chuột giả lập cùng VID/PID.

## Bài học quan trọng (đã mất nhiều thời gian mới tìm ra)
1. **ILITEK không gửi report đa điểm cho tới khi host ghi feature report "Contact Count Maximum" (0x0D/0x55, ở đây report ID 6) = 10.** Không có thì interface Digitizer im lặng (và interface chuột lo việc làm con trỏ nhảy ở màn hình chính). `set_device_mode` trong `src/macos.rs` ghi trường này (và Device Mode 0x52 nếu có) từ descriptor, không hardcode. Đừng gỡ.
2. **Sự kiện cuộn phải có vị trí.** Event tạo bằng `CGEventCreateScrollWheelEvent2` đi vào cửa sổ dưới con trỏ hiện tại. `Sink.hint` giữ tâm các điểm chạm; khi bắt đầu cuộn app `MoveTo` tới đó và gắn `CGEventSetLocation`.
3. **Sink giãn cách sự kiện chuột** (MoveTo → 15ms → MouseDown → 40ms → MouseUp) bằng hàng đợi `queue`/`pump` chạy theo timer 120Hz. Click phát liền trong một vòng lặp bị menu ngữ cảnh bỏ qua (lỗi: sau click phải bằng 2 ngón, chạm 1 ngón không đóng được menu). Nếu lỗi vẫn còn, kiểm tra thứ tự/thời gian ở đây trước.
4. **Hot-plug** dùng `IOHIDManagerRegisterDeviceMatchingCallback`/`RemovalCallback` + `ScheduleWithRunLoop`, **không cần `IOHIDManagerOpen`**; mở thiết bị bằng `IOHIDDeviceOpen(seize)` ngay trong callback. Interface chuột giả lập được chặn bằng manager thứ hai (`on_mouse_added`) và cũng khi cảm ứng vừa mở.
5. **Màn hình đích có thể chưa bật lúc cảm ứng xuất hiện** (sau khi cắm lại) → timer tính lại `target_bounds` mỗi ~1 giây.
6. **Chữ ký:** ký ad-hoc làm macOS coi mỗi bản build là app mới và mất quyền TCC. `scripts/make-app.sh` tự dùng chứng chỉ cục bộ **"Touchmac Local Signing"** (trong keychain đăng nhập) nếu có, giữ quyền qua các lần build. Nếu thiếu chứng chỉ, quay về ad-hoc và cần `tccutil reset ListenEvent local.touchrelay; tccutil reset Accessibility local.touchrelay` (luôn kèm bundle id, không reset toàn cục) rồi cấp lại quyền.
7. **Kéo 1 ngón = cuộn** (mặc định). Giữ yên ≥0.3s (`drag_hold_time`) → engine nhấn giữ nút chuột trái ngay (MouseDown), sau đó kéo = kéo chuột (thanh trượt, chọn chữ), nhả không kéo = click. Click phải = chạm 2 ngón (long-press click phải chỉ còn khi tắt kéo-1-ngón-cuộn), vì long-press trước đây làm mở context menu khi người dùng giữ để kéo.
8. Claude Code chạy lệnh trong shell riêng nên **không có quyền Input Monitoring/Accessibility**. Muốn chạy `./scripts/dev.sh` cần mở trong Terminal panel của app (`run_in_terminal`) và người dùng phải chạm màn hình trong lúc chạy. Lệnh trong `run_in_terminal` chỉ được chứa ASCII.

## Cài đặt người dùng
- Menu bar (gọn): trạng thái + thiết bị; Tạm dừng, Chạm thông minh, Ẩn con trỏ; Tốc độ cuộn ▸, Cuộn & cử chỉ ▸, Màn hình ▸ (Màn hình đích ▸, Hướng ▸, Cỡ giao diện ▸), Công cụ ▸ (Hiệu chỉnh, Kiểm tra vùng chạm, Kiểm tra màn hình, Hướng dẫn, Khắc phục sự cố, Sửa tuỳ chọn, Xem nhật ký); Cài đặt…, Mở cùng macOS, Giới thiệu, Thoát. Thêm mục mới: đặt đúng nhóm, không thêm ở cấp trên cùng.
- Lưu ở `~/Library/Application Support/touchrelay/settings.txt` (`Prefs` trong `src/main.rs`, chuyển thành cờ dòng lệnh đặt trước `options.txt`, nên `options.txt` ghi đè được).
- Cờ `--settings` mở luôn cửa sổ cài đặt khi khởi động (chỉ chạy được từ .app/Terminal có quyền; shell của Claude không có quyền Screen Recording nên không chụp được ảnh giao diện, nhờ người dùng chụp).
- Log khi chạy dạng app: `~/Library/Logs/touchrelay.log`.

## Kiến trúc
```
src/main.rs      CLI + Prefs (settings.txt) ; mở từ .app: nối cờ từ Prefs rồi options.txt, ghi stderr ra log
src/hid_desc.rs  Parser HID Report Descriptor -> Field{report_id, bit_offset, bit_size, usage, logical min/max, collection}
src/touch.rs     TouchLayout (collection Finger 0x0D/0x22 dưới Touch Screen 0x0D/0x04, device_mode, contact_count_max),
                 parse_report(raw), FrameAssembler (parallel + hybrid). Test dùng descriptor/report ILITEK thật.
src/gesture.rs   Engine state machine thuần Rust: update(touches, now) / tick(now) -> Vec<Action>
src/macos.rs     Mapper (xoay/lật), Sink (Action -> CGEvent, hàng đợi giãn cách), hot-plug (Hot, watch, on_touch_added/removed),
                 timer 120Hz, App state, quyền (IOHIDRequestAccess / CGPreflightPostEventAccess)
src/menubar.rs   NSStatusItem/NSMenu/NSAlert/NSWindow qua objc_msgSend. Menu có delegate (menuNeedsUpdate: -> sync_menu) nên dấu chọn luôn khớp trạng thái.
                 Cửa sổ "Cài đặt…" (open_settings): sidebar 6 trang (Chung, Cuộn, Cử chỉ, Màn hình, Công cụ, Giới thiệu); vùng chi tiết là NSScrollView, `show_page` dựng thử trang để đo chiều cao rồi dựng thật (nội dung dài thì cuộn được), control native cỡ lớn (NSSwitch, NSSegmentedControl, NSPopUpButton), mô tả cho từng mục, nền NSVisualEffectView, nút dưới dùng bezel Glass (macOS 26+).
                 Công tắc dùng chung `flip(tag)` với menu. "Giới thiệu" = hộp About chuẩn (orderFrontStandardAboutPanelWithOptions) kèm email liên hệ.
src/i18n.rs      Đa ngôn ngữ: khoá là câu tiếng Việt, bảng EN dịch sang tiếng Anh; `tr()` theo ngôn ngữ hệ thống (AppleLanguages đầu tiên bắt đầu bằng "vi" = tiếng Việt, còn lại = tiếng Anh).
                 Mọi chuỗi hiển thị cho người dùng phải qua `tr()` và có trong bảng EN (test kiểm tra trùng khoá). Menu tự dựng lại khi ngôn ngữ đổi (menuNeedsUpdate:).
                 `APP_NAME`/`CONTACT_EMAIL` đặt ở đây: đổi tên app ở đây + bundle id/ Info.plist/ make-app.sh/ tên chứng chỉ (quyền macOS sẽ phải cấp lại).
src/display.rs   Cỡ giao diện màn hình cảm ứng: đổi chế độ hiển thị (1.3×/1.6×/2×, chọn cùng tỉ lệ+tần số quét) bằng CGConfigureDisplayWithDisplayMode
                 kCGConfigureForSession; khôi phục khi thoát (không lưu qua các lần mở). pick_mode có test.
src/ffi.rs       FFI CF/IOKit/CG
scripts/make-app.sh  build + bundle + ký + (INSTALL=1) cài /Applications
scripts/dev.sh       tắt app, build, chạy --debug N giây, ghi dev.log
```

## Hiệu chỉnh vị trí chạm
`src/calib.rs`: ánh xạ **từng đoạn theo mỗi trục** (piecewise-linear, có test), từ toạ độ cảm biến sau xoay/lật → toạ độ màn hình chuẩn hoá. Affine KHÔNG đủ: ILITEK phi tuyến ở rìa (dấu 8% trên đọc 0.048, tâm 0.503, 92% dưới đọc 0.98). Luồng: menu/Cài đặt → Màn hình → "Hiệu chỉnh vị trí chạm" → lớp phủ đen trên màn hình đích (`menubar::calib_show`) hiện 9 dấu + trên hai đường chéo, thụt 6% (thụt 3% thì ILITEK không nhận chạm ở góc dưới-phải; dấu nào 8s không có chạm thì tự bỏ qua để khỏi kẹt) (mỗi giá trị trục có ≥2 mẫu); mỗi lần nhấc ngón là một mẫu (trung bình các frame); chạm 2 ngón để hủy; sai số tối đa >0.05 thì báo thất bại. Lưu `calib=` (danh sách số) trong settings.txt, nạp bằng cờ `--calib`. Kết quả hiện ngay trên lớp phủ (`calib_message`, tự đóng sau 3s) chứ KHÔNG dùng NSAlert modal: modal hiện ở màn hình MacBook và chặn mọi thao tác chạm. Click ở vị trí trung bình các frame sau frame đầu (`tap_point` trong gesture.rs) vì frame đầu khi ngón mới chạm lệch nhất.

## Công cụ kiểm tra (lớp phủ trên màn hình đích)
- **Kiểm tra vùng chạm** (`Probe::Touch`, `src/coverage.rs`): canvas 640×400 vẽ vệt vàng nơi cảm biến ghi nhận (toạ độ sau hiệu chỉnh, KHÔNG kẹp, điểm ngoài [0,1] bỏ qua). Chạm 4 ngón để kết thúc; ghi nhận min/max toạ độ cảm biến (sau xoay/lật, trước hiệu chỉnh) → lưu `extent=` và `Calib::with_extents` thêm nút (min→0, max→1) để **dải sát mép không nhận chạm (vd. menu bar) vẫn chạm tới được**. "Đặt lại hiệu chỉnh" xoá cả extent.
- **Kiểm tra màn hình** (`Probe::Colors`): màu đặc lần lượt (trắng, đen, đỏ, lục, lam, xám) để tìm điểm ảnh chết; chạm để đổi, 4 ngón để thoát.
- Cả hai dùng chung cửa sổ với lớp phủ hiệu chỉnh (`CALIB_WIN` trong menubar.rs). Khi đang chạy, chạm không sinh click.

## Lưu ý AppKit trên arm64
`NSTextAlignment` dùng giá trị kiểu iOS trên Apple Silicon (giữa = 1, phải = 2), x86_64 thì giữa = 2: dùng hằng `TEXT_CENTER` trong menubar.rs, đừng hardcode.

## Hiệu chỉnh gắn với độ phân giải
Màn hình cảm ứng có thể đổi vùng chạm theo chế độ hiển thị (người dùng thấy: đổi 16:9 ↔ 16:10 thì vùng chạm lệch tới khi rút/cắm lại cáp). `Mapper.calib_size` ghi độ phân giải (điểm) của màn hình đích lúc hiệu chỉnh/đo vùng chạm (`calib_size=WxH` trong settings.txt, cờ `--calib-size`); khác độ phân giải hiện tại thì `calib_stale()` → hiệu chỉnh + extent tự TẮT (Cài đặt hiện cảnh báo) cho tới khi hiệu chỉnh lại hoặc quay về độ phân giải cũ. Dữ liệu cũ không có `calib_size` được coi là luôn áp dụng. Phát hiện: cảm biến ILITEK khai báo vùng vật lý 16:9 (3093×1739) nên trên màn 16:10 chỉ phủ ~90% chiều cao (4.3%–94.2%); chế độ 16:9 có viền đen khớp vùng chạm.

## Chạm thông minh (snap)
`src/snap.rs`: tap trái trượt nút nhỏ thì dùng Accessibility (`AXUIElementCopyElementAtPosition`, timeout 50ms) dò quanh điểm chạm (bán kính 5–20pt, 8 hướng); nếu điểm chạm chưa trúng điều khiển bấm được mà có điều khiển ở gần thì click vào tâm nó (nút <40pt) hoặc điểm dò. Chỉ áp dụng cho chuỗi MoveTo+MouseDown trái+MouseUp do tap sinh ra (`find_tap_clicks`), không đụng kéo/cuộn/click phải. Cần quyền Accessibility; tắt bằng menu hoặc `--no-snap`. Màn hình ILITEK 2560×1600 chỉ có chế độ gốc và 2× HiDPI sắc nét; 1.3×/1.6× bị mờ (không có framebuffer trung gian), nên snap là cách chính để chạm nút nhỏ mà không phóng to.

## Con trỏ
`Sink` ẩn con trỏ khi có chạm (`CGDisplayHideCursor` + API riêng `CGSSetConnectionProperty("SetsCursorInBackground")` để app nền ẩn được) và hiện lại khi chuột/trackpad thật làm con trỏ lệch >4px khỏi vị trí app đặt, khi tạm dừng, rút thiết bị hoặc thoát. Tắt bằng menu "Ẩn con trỏ khi chạm" / cờ `--keep-cursor`. Chưa kiểm chứng trên máy thật.

## Touch native của macOS 27 (đã nghiên cứu 2026-11-23)
- SDK 27.2 có `NSScreen.touchCapabilities`, `NSScrollView.touchScrollingEnabled`, `NSGestureRecognizer.allowedTouchTypes` mặc định `.direct`, MacBook Pro OLED M5 Pro/Max có touch. Theo bài của dodecaidr (dẫn TN3212): touch trực tiếp **chỉ nhận được qua gesture recognizer trong chính app**, không đi qua `NSEvent`/event monitor toàn cục; chưa thấy API công khai nào để *phát* touch hay đọc touch toàn cục.
- Kết luận: không map được ILITEK vào API touch native của Apple; tiếp tục phát chuột/cuộn/gesture event như hiện nay. **Đã kiểm chứng (2026-11-23, macOS 27.2.1):** khi tắt TouchRelay, `NSScreen.touchCapabilities` = 0 cho mọi màn hình, kể cả màn hình ILITEK → macOS không tự nhận USB touchscreen bên thứ ba là touch native; TouchRelay vẫn cần thiết. (Thử bằng script Swift nhỏ in `NSScreen.screens` + `touchCapabilities`.)

## Giả định còn chưa kiểm chứng
- Pinch dùng gesture event riêng (không công khai): CGEventType 29, field 55/110/113/132, kIOHIDEventTypeZoom=8, GestureStarted/Ended 61/62.
- Hướng "Dọc" (swap_xy + flip_x hoặc flip_y) tính theo toán học, chưa thử trên màn hình thật xoay dọc; nếu ngược thì đổi hai lựa chọn.
- Màn hình touch hãng khác: parser đọc descriptor nên thường chạy ngay; cần kiểm tra xem có cần feature report bật chế độ như ILITEK không.

## Lệnh hay dùng
```bash
cargo test                     # 16 test (descriptor, frame, gesture, ILITEK thật)
cargo clippy --all-targets
./target/release/touchrelay --list
./scripts/dev.sh 20 [cờ thêm]  # chạy trong Terminal panel, vừa chạy vừa chạm
INSTALL=1 ./scripts/make-app.sh
tail -f ~/Library/Logs/touchrelay.log
```

## Repo công khai (GitHub: fpt-duongtx/touchrelay, MIT)
- `README.md` (EN) + `README.vi.md`, `DISCLAIMER.md`, `LICENSE`, trang web tĩnh trong `docs/` (GitHub Pages: nguồn = nhánh main, thư mục /docs; EN/VI tự theo trình duyệt), `.github/workflows/` (ci.yml chạy test; release.yml dựng bản ad-hoc universal + sha256 khi đẩy tag `v*`).
- Chủ sở hữu chưa có Developer ID trả phí → bản tải sẵn chỉ ký ad-hoc, README/DISCLAIMER đã nói rõ. Đừng đưa email cá nhân vào file công khai (README/site) trừ khi người dùng cho phép rõ ràng; kênh liên hệ mặc định là GitHub Issues. (Email hiện còn trong `src/i18n.rs` `CONTACT_EMAIL` cho hộp About.)
- Xem trước trang: `cd docs && python3 -m http.server 8765` rồi mở http://127.0.0.1:8765/ (xem dạng file `data:` sẽ không tải CSS/JS).
