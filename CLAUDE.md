# touchrelay: ghi chú bàn giao cho Claude Code

Trả lời người dùng bằng **tiếng Việt**.

## Mục tiêu
App Rust cho macOS biến màn hình cảm ứng USB rời (chuẩn HID Digitizer) thành thao tác native: tap = click, giữ = click phải, kéo 1 ngón = cuộn (mặc định) hoặc kéo chuột, 2 ngón = cuộn pixel có phase + quán tính, pinch = magnify, vuốt 3 ngón = Ctrl+mũi tên. Không dùng crate ngoài; IOKit/CoreFoundation/CoreGraphics/ObjC runtime gọi qua FFI thuần.

## Trạng thái (đã chạy được trên máy thật)
Chạm, cuộn 1 ngón, hot-plug (rút/cắm cáp) đều hoạt động với ILITEK-TP. Người dùng cuối dùng `TouchRelay.app` (menu bar), không phải đọc log.

### Máy của người dùng
- MacBook Pro Apple Silicon (arm64), zsh, Rust qua rustup (`. "$HOME/.cargo/env"` nếu thiếu `cargo`).
- Project: `~/Projects/touchmac`. App cài ở `/Applications/TouchRelay.app`, bundle id `io.github.fpt-duongtx.touchrelay`.
- Màn hình MacBook id=1 1800x1169 tại (0,0); màn hình rời id=3 2560x1600. **Vị trí màn hình rời đổi theo lần sắp xếp** (từng là (1800,0), hiện (-2560,-244)); app luôn đọc từ CoreGraphics lúc chạy và theo dõi thay đổi mỗi giây.
- Thiết bị: **ILITEK-TP vid=0x222a pid=0x0001**, 10 slot/report, report ID 4, kèm 1 interface chuột giả lập cùng VID/PID.

## Bài học quan trọng (đã mất nhiều thời gian mới tìm ra)
1. **ILITEK không gửi report đa điểm cho tới khi host ghi feature report "Contact Count Maximum" (0x0D/0x55, ở đây report ID 6) = 10.** Không có thì interface Digitizer im lặng (và interface chuột lo việc làm con trỏ nhảy ở màn hình chính). `set_device_mode` trong `src/macos.rs` ghi trường này (và Device Mode 0x52 nếu có) từ descriptor, không hardcode. Đừng gỡ.
2. **Sự kiện cuộn phải có vị trí.** Event tạo bằng `CGEventCreateScrollWheelEvent2` đi vào cửa sổ dưới con trỏ hiện tại. `Sink.hint` giữ tâm các điểm chạm; khi bắt đầu cuộn app `MoveTo` tới đó và gắn `CGEventSetLocation`.
3. **Sink giãn cách sự kiện chuột** (MoveTo → 15ms → MouseDown → 40ms → MouseUp) bằng hàng đợi `queue`/`pump` chạy theo timer 120Hz. Click phát liền trong một vòng lặp bị menu ngữ cảnh bỏ qua (lỗi: sau click phải bằng 2 ngón, chạm 1 ngón không đóng được menu). Nếu lỗi vẫn còn, kiểm tra thứ tự/thời gian ở đây trước.
4. **Hot-plug** dùng `IOHIDManagerRegisterDeviceMatchingCallback`/`RemovalCallback` + `ScheduleWithRunLoop`, **không cần `IOHIDManagerOpen`**; mở thiết bị bằng `IOHIDDeviceOpen(seize)` ngay trong callback. Interface chuột giả lập được chặn bằng manager thứ hai (`on_mouse_added`) và cũng khi cảm ứng vừa mở.
5. **Màn hình đích có thể chưa bật lúc cảm ứng xuất hiện** (sau khi cắm lại) → timer tính lại `target_bounds` mỗi ~1 giây.
6. **Chữ ký:** ký ad-hoc làm macOS coi mỗi bản build là app mới và mất quyền TCC. `scripts/make-app.sh` tự dùng chứng chỉ cục bộ **"Touchmac Local Signing"** (trong keychain đăng nhập) nếu có, giữ quyền qua các lần build. Nếu thiếu chứng chỉ, quay về ad-hoc và cần `tccutil reset ListenEvent io.github.fpt-duongtx.touchrelay; tccutil reset Accessibility io.github.fpt-duongtx.touchrelay` (luôn kèm bundle id, không reset toàn cục) rồi cấp lại quyền.
7. **Kéo 1 ngón = cuộn** (mặc định). Giữ yên ≥0.3s (`drag_hold_time`) → engine nhấn giữ nút chuột trái ngay (MouseDown), sau đó kéo = kéo chuột (thanh trượt, chọn chữ), nhả không kéo = click. Click phải = chạm 2 ngón (long-press click phải chỉ còn khi tắt kéo-1-ngón-cuộn), vì long-press trước đây làm mở context menu khi người dùng giữ để kéo.
8. Claude Code chạy lệnh trong shell riêng nên **không có quyền Input Monitoring/Accessibility**. Muốn chạy `./scripts/dev.sh` cần mở trong Terminal panel của app (`run_in_terminal`) và người dùng phải chạm màn hình trong lúc chạy. Lệnh trong `run_in_terminal` chỉ được chứa ASCII.

## Cài đặt người dùng
- Menu bar (gom theo mục đích): trạng thái + thiết bị; công tắc nhanh (Tạm dừng, Chạm thông minh, Ẩn con trỏ); Bàn phím ảo ▸ (tự hiện, hiện ngay, cỡ); Cuộn & cử chỉ ▸ (tốc độ, đảo chiều, quán tính, 1 ngón cuộn, kéo thông minh, chụm, vuốt 3 ngón, độ nhạy vuốt); Màn hình ▸ (đích, hướng, cỡ giao diện); Công cụ ▸ (hiệu chỉnh, kiểm tra, hướng dẫn, sự cố, tuỳ chọn, log); rồi Cài đặt…, Ngôn ngữ ▸, Mở cùng macOS, Giới thiệu, Thoát. Cửa sổ Cài đặt có 6 trang cùng cấu trúc (Chung, Cuộn & cử chỉ, Bàn phím, Màn hình, Công cụ, Giới thiệu). Mục mới phải vào đúng nhóm, không thêm ở cấp trên cùng.
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
src/i18n.rs      Đa ngôn ngữ: khoá là câu tiếng Việt, bảng EN dịch sang tiếng Anh; `tr()` theo ngôn ngữ hệ thống, hoặc theo lựa chọn của người dùng (`LANG_MODE` trong menubar.rs: 0 theo hệ thống, 1 English, 2 Tiếng Việt; Prefs `language`, cờ `--lang system|en|vi`; menu dựng lại khi `is_vi()` khác `MENU_BUILT_VI`) (AppleLanguages đầu tiên bắt đầu bằng "vi" = tiếng Việt, còn lại = tiếng Anh).
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
- **Kiểm tra vùng chạm** (`Probe::Touch`, `src/coverage.rs`): canvas 640×400 vẽ vệt vàng nơi cảm biến ghi nhận (toạ độ sau hiệu chỉnh, KHÔNG kẹp, điểm ngoài [0,1] bỏ qua). Chạm 3 ngón để kết thúc; ghi nhận min/max toạ độ cảm biến (sau xoay/lật, trước hiệu chỉnh) → lưu `extent=` và `Calib::with_extents` thêm nút (min→0, max→1) để **dải sát mép không nhận chạm (vd. menu bar) vẫn chạm tới được**. "Đặt lại hiệu chỉnh" xoá cả extent.
- **Kiểm tra màn hình** (`Probe::Colors`): màu đặc lần lượt (trắng, đen, đỏ, lục, lam, xám) để tìm điểm ảnh chết; chạm để đổi, 3 ngón để thoát.
- Cả hai dùng chung cửa sổ với lớp phủ hiệu chỉnh (`CALIB_WIN` trong menubar.rs). Khi đang chạy, chạm không sinh click.

## Lưu ý AppKit trên arm64
`NSTextAlignment` dùng giá trị kiểu iOS trên Apple Silicon (giữa = 1, phải = 2), x86_64 thì giữa = 2: dùng hằng `TEXT_CENTER` trong menubar.rs, đừng hardcode.

## Hiệu chỉnh gắn với độ phân giải
Màn hình cảm ứng có thể đổi vùng chạm theo chế độ hiển thị (người dùng thấy: đổi 16:9 ↔ 16:10 thì vùng chạm lệch tới khi rút/cắm lại cáp). `Mapper.calib_size` ghi độ phân giải (điểm) của màn hình đích lúc hiệu chỉnh/đo vùng chạm (`calib_size=WxH` trong settings.txt, cờ `--calib-size`); khác độ phân giải hiện tại thì `calib_stale()` → hiệu chỉnh + extent tự TẮT (Cài đặt hiện cảnh báo) cho tới khi hiệu chỉnh lại hoặc quay về độ phân giải cũ. Dữ liệu cũ không có `calib_size` được coi là luôn áp dụng. Phát hiện: cảm biến ILITEK khai báo vùng vật lý 16:9 (3093×1739) nên trên màn 16:10 chỉ phủ ~90% chiều cao (4.3%–94.2%); chế độ 16:9 có viền đen khớp vùng chạm.

## Kéo thông minh (smart drag)
Một ngón mặc định là cuộn, nên giữ 0.3s mới kéo chuột rất bất tiện. `Engine.intent_probe` (đặt trong `macos.rs`) được hỏi ở thời điểm ngón vừa vượt ngưỡng kéo: `snap::drag_intent(p)` dò chuỗi vai trò AX từ phần tử dưới ngón lên gốc rồi `classify_drag_intent`: gặp vai trò KÉO trước (thanh trượt, thanh cuộn, splitter, ô nhập 1 dòng, dock item) → kéo chuột ngay; gặp vùng CUỘN trước (ScrollArea, WebArea, Table, List, TextArea…) → cuộn; hết chuỗi mà không có gì cuộn được (vd. thanh tiêu đề cửa sổ) → kéo; không dò được → cuộn như cũ. Giữ yên 0.3s vẫn ép kéo ở mọi nơi. Tắt: menu/Cài đặt "Kéo thông minh" hoặc `--no-smart-drag`.
**Cạm bẫy:** truy vấn AX vào cửa sổ của chính TouchRelay từ luồng chính sẽ treo tới hết timeout (app tự chờ chính mình). Mọi chỗ dùng AX (snap, drag_intent, kbd) phải bỏ qua điểm nằm trong cửa sổ của mình: `menubar::point_in_own_windows`.

## Lưu ý: mọi hành động của engine phải qua `App::dispatch`
`Engine::update` VÀ `Engine::tick` đều sinh `Action`. `ToggleKeyboard` là việc của App (không phải Sink), nên cả hai đường (on_report và on_timer) phải gọi `app.dispatch(a)`. Từng có lỗi: cử chỉ giữ 3 ngón do `tick` sinh ra bị Sink nuốt vì on_timer đưa thẳng vào `sink.apply`.

## Bàn phím ảo
`menubar.rs` (mục "bàn phím ảo nổi"): `NSPanel` không kích hoạt (`1<<7`), mức cửa sổ 25, 706×274pt, 5 hàng ANSI như bàn phím vật lý (không hàng F), Shift/Ctrl/Opt/Cmd dính một lần, Caps giữ, Backspace/mũi tên giữ để lặp, thanh kéo trên cùng + nút ✕, nhớ vị trí lần trước. Phím phát MÃ PHÍM (`macos::post_key`) nên Telex/VNI qua bộ gõ hệ thống vẫn chạy. Phím 🌐 (góc trái hàng dưới, `KeyKind::Lang`) đổi nguồn nhập bằng Text Input Services (`src/input_source.rs`: TISCreateInputSourceList → chọn kế tiếp, vòng lại) chứ KHÔNG phát phím Fn/Ctrl+Space (phụ thuộc cài đặt máy); nhãn hiện mã ngôn ngữ hiện tại. Tự hiện/ẩn: sau mỗi tap trái (`kbd_check`, +0.2s) `snap::focus_info` đọc `AXFocusedUIElement` của app trên cùng (bật `AXManualAccessibility` cho Chrome/Electron): ô nhập (`is_text_role`) → hiện (né ô nhập nếu bị che), thứ khác → ẩn; đóng tay bằng ✕ thì không tự hiện lại tới khi rời ô nhập (`kbd_dismissed`). Chạm 2 lần bằng 3 ngón (`Action::ToggleKeyboard`, nhận ở `State::Three` khi max_n≥3, không trượt, ≤0.45s, 2 lần cách ≤0.6s; App xử lý `toggle_keyboard_by_gesture`) hiện/ẩn thủ công; hoặc giữ yên 3 ngón `four_hold_time`=3s (kiểm ở `Engine::tick`, một lần mỗi lần đặt tay, huỷ nếu trượt). Cỡ phím 42/52/62pt (`kbd_set_size`, Prefs `kbd_size`, cờ `--kbd-size`). Tắt: menu/Cài đặt "Bàn phím ảo" hoặc `--no-keyboard`; `--show-keyboard` hiện ngay lúc khởi động (kiểm tra). `msg_ret_rect` có nhánh x86_64 (`objc_msgSend_stret`).

## Vuốt nhiều ngón và cảm biến ILITEK (đã đo)
- Cảm biến thường báo **4 ngón thành 3** (chỉ vài frame lên 4) và đôi khi chỉ nhận 2 trong 4 ngón (→ bị hiểu là chạm 2 ngón = click phải/context menu). Vì vậy: giữ 4 ngón chỉ cần *từng thấy 4* và *còn ≥3*; "đã di chuyển" đo theo vị trí đầu của TỪNG ngón (không theo trọng tâm vì trọng tâm nhảy khi 3↔4).
- Quãng vuốt không còn cố định 120px: `swipe_distance_x/y` do App đặt theo kích thước màn hình đích × độ nhạy (`sync_swipe`: 7% / 12% mặc định / 18%), và hướng chính phải trội `swipe_dominance`=1.5 lần hướng kia. Cài đặt "Độ nhạy vuốt nhiều ngón", cờ `--swipe-level`.
- Chưa làm: chuyển Space mượt theo ngón (cần phát sự kiện DockSwipe riêng, không tài liệu; thử nghiệm sau).
- `./scripts/dev.sh` log frame có mốc thời gian `t=`; `ToggleKeyboard` được in ở chế độ debug.

## Công cụ kiểm tra giao diện không cần ảnh chụp từ người dùng
`touchrelay --settings --settings-page N --debug --snapshot FILE.png` (chạy qua `open --stdout/--stderr … /Applications/TouchRelay.app --args …` để giữ quyền): mở cửa sổ Cài đặt ở trang N, in trạng thái cuộn (`[ui] …`) và sau 0.8s chụp cửa sổ bằng `cacheDisplayInRect` (không cần quyền ghi màn hình). Lưu ý ảnh chụp kiểu này vẽ nền trắng và không có thanh cuộn overlay (đã mờ), nên chữ màu phụ/chữ trắng sẽ KHÔNG thấy trong ảnh dù thật ra hiển thị đúng; dùng để kiểm tra bố cục/độ cao nội dung.
Cuộn trong Cài đặt: `NSScrollView` (style = `preferredScrollerStyle`) + quan sát bounds của clip view → `flashScrollers` mỗi lần cuộn, để thanh overlay hiện khi cuộn bằng sự kiện cuộn tự phát.

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

## Phát hành 1.0 (2026-11-25)
Cargo/Cargo.lock = 1.0.0. Trang web `docs/`: lưới ảnh 6 cột (3/2 cột trên màn nhỏ) + lightbox có nút/phím ←→, vuốt, vòng lại; ảnh dùng cho site: menu, general, scrolling, keyboard, display, tools (KHÔNG đưa ảnh trang About vì có email).
