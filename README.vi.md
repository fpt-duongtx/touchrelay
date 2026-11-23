# TouchRelay for Mac

[English](README.md) · [Trang web](https://fpt-duongtx.github.io/touchrelay/) · [Miễn trừ trách nhiệm](DISCLAIMER.md) · [Giấy phép MIT](LICENSE)

Giúp **màn hình cảm ứng USB rời** hoạt động **như native trên macOS**. macOS không hỗ trợ màn hình cảm ứng của bên thứ ba: cắm vào chỉ như một con chuột 1 điểm. TouchRelay đọc báo cáo HID thô, nhận diện cử chỉ rồi phát ra những sự kiện mà app macOS đã hiểu: click, kéo, cuộn mượt có quán tính, pinch-zoom và vuốt nhiều ngón.

Viết bằng Rust thuần, **không có dependency**: IOKit, CoreFoundation, CoreGraphics gọi qua FFI. Chạy dạng app nhỏ trên thanh menu và **không bao giờ kết nối mạng**.

> **Không chính thức, chưa ký/notarize.** Không liên kết với Apple hay hãng màn hình nào. Hãy đọc [miễn trừ trách nhiệm](DISCLAIMER.md) trước khi dùng.

## Cử chỉ

| Trên màn hình cảm ứng | Kết quả trên macOS |
|---|---|
| Chạm 1 ngón | Click trái (chạm nhanh 2 lần = double-click) |
| Kéo 1 ngón | Cuộn kiểu iPad (có thể đổi thành kéo chuột) |
| Giữ yên ~0.3s rồi kéo | Kéo chuột: thanh trượt, chọn chữ, kéo cửa sổ/file |
| Kéo 2 ngón | Cuộn mượt theo pixel có **quán tính** |
| Chạm 2 ngón | Click phải |
| Chụm / xoè | Zoom (Safari, Preview, Maps…) |
| Vuốt 3 ngón trái / phải | Chuyển Space hoặc app full-screen |
| Vuốt 3 ngón lên / xuống | Mission Control / App Exposé |
| Chạm khi đang trượt quán tính | Dừng cuộn ngay |

## Tính năng

- **Cắm/rút nóng**: cắm hoặc rút màn hình lúc nào cũng được.
- **Chạm thông minh**: nếu chạm trượt một nút nhỏ, TouchRelay dùng Accessibility để chọn điều khiển gần nhất trong vòng ~20 điểm.
- **Hiệu chỉnh vị trí chạm**: chạm 9 dấu để sửa cảm biến không tuyến tính ở gần mép (màn hình dùng để phát triển bị như vậy).
- **Kiểm tra vùng chạm**: rê ngón khắp màn hình, nơi cảm biến nhận chạm hiện **màu vàng**; phạm vi đo được dùng để chạm tới sát mép (ví dụ menu bar).
- **Kiểm tra màn hình**: các màu đặc để tìm điểm ảnh chết.
- **Cỡ giao diện**: tạm phóng to giao diện trên màn hình cảm ứng (đa số tấm nền chỉ mức 2× là sắc nét).
- **Ẩn con trỏ** khi dùng cảm ứng, hiện lại khi bạn dùng chuột/trackpad.
- Cửa sổ Cài đặt native (sidebar + nhóm form), giao diện tiếng Việt/Anh theo ngôn ngữ hệ thống.

## Phần cứng đã thử

| Màn hình cảm ứng | Kết quả |
|---|---|
| ILITEK-TP, `vid=0x222a pid=0x0001`, 10 điểm chạm (tấm nền 2560×1600) | Hoạt động |

Mọi màn hình "Windows compatible" dùng chuẩn **HID Digitizer / Touch Screen** (usage `0x0D/0x04`) đều nên chạy, vì cấu trúc được đọc từ HID report descriptor chứ không hardcode. **Rất mong báo cáo từ các màn hình khác**, xem [Đóng góp](#đóng-góp).

Vài điều học được với panel ILITEK:

- Nó im lặng cho đến khi host ghi feature report *Contact Count Maximum*. TouchRelay tự làm từ descriptor.
- Cảm biến chỉ phủ ~90% chiều cao của tấm 16:10 (cảm biến 16:9). Ở chế độ hiển thị 16:9 có viền đen, vùng chạm khớp hoàn toàn với hình ảnh.

## Cài đặt

Cần macOS 13 trở lên, chip Apple Silicon hoặc Intel.

### Cách A: tự build từ mã nguồn (khuyên dùng, không bị cảnh báo)

```bash
# Cần Rust (https://rustup.rs) và Xcode Command Line Tools: xcode-select --install
git clone https://github.com/fpt-duongtx/touchrelay.git
cd touchrelay
INSTALL=1 ./scripts/make-app.sh
open /Applications/TouchRelay.app
```

Script build bản release, tạo `dist/TouchRelay.app`, ký ngay trên máy bạn và (với `INSTALL=1`) chép vào `/Applications`. Tuỳ chọn: `UNIVERSAL=1` (Apple Silicon + Intel), `SIGN_IDENTITY="…"` (chứng chỉ của bạn), `BUNDLE_ID=…`.

### Cách B: tải app (DMG)

1. Tải `TouchRelay.dmg` (app universal Apple Silicon + Intel) ở [bản phát hành mới nhất](https://github.com/fpt-duongtx/touchrelay/releases/latest). Mã **MD5** (`.md5`) và SHA-256 (`.sha256`) được đăng ngay cạnh. Kiểm tra tuỳ chọn: `md5 -q ~/Downloads/TouchRelay.dmg`.
2. Mở file DMG và kéo **TouchRelay** vào **Applications**.
3. Mở TouchRelay. Bản này **ký ad-hoc và chưa được Apple notarize** (người duy trì không mua gói Developer ID), nên macOS có thể chặn: xem [Nếu macOS chặn app](#nếu-macos-chặn-app).

**Cho ai lười: dán một lần vào Terminal.** Tải DMG mới nhất, kiểm tra MD5, chép app vào Applications, đánh dấu tin cậy (gỡ cờ quarantine nên không bị Gatekeeper hỏi) rồi mở app:

```bash
cd ~/Downloads
curl -fLO https://github.com/fpt-duongtx/touchrelay/releases/latest/download/TouchRelay.dmg
curl -fLO https://github.com/fpt-duongtx/touchrelay/releases/latest/download/TouchRelay.dmg.md5
[ "$(md5 -q TouchRelay.dmg)" = "$(cut -d' ' -f1 TouchRelay.dmg.md5)" ] && echo "MD5 OK" || echo "MD5 MISMATCH - stop here"
hdiutil attach -nobrowse -quiet -mountpoint /tmp/touchrelay-dmg TouchRelay.dmg
cp -R /tmp/touchrelay-dmg/TouchRelay.app /Applications/
hdiutil detach -quiet /tmp/touchrelay-dmg
xattr -dr com.apple.quarantine /Applications/TouchRelay.app
open /Applications/TouchRelay.app
```

> Cách này bỏ qua bước kiểm tra của Apple, nên **chỉ chạy khi bạn tin repository này** và MD5 báo OK.
> Chữ ký ad-hoc đổi theo từng bản build nên macOS coi mỗi phiên bản là app mới, và bạn có thể phải cấp lại hai quyền sau khi cập nhật.

### Nếu macOS chặn app

Vì TouchRelay chưa được notarize, Gatekeeper có thể báo không xác minh được app. Bạn vẫn mở được khi đã tin nguồn tải (một lần cho mỗi phiên bản):

1. Mở TouchRelay trong Applications. Khi macOS báo không xác minh được app, bấm **Done** (đừng bấm *Move to Trash*).
2. Vào **System Settings → Privacy & Security**, cuộn tới mục **Security**, cạnh dòng "TouchRelay was blocked to protect your Mac" bấm **Open Anyway**.
3. Nhập mật khẩu hoặc dùng Touch ID, rồi bấm **Open Anyway** thêm lần nữa.

macOS 14 trở xuống: Control-click vào app → **Open** → **Open**. Hoặc dùng Terminal: `xattr -dr com.apple.quarantine /Applications/TouchRelay.app`.

### Quyền (lần đầu mở)

TouchRelay xin hai quyền và tự mở đúng trang cài đặt:

- **Input Monitoring**: để đọc báo cáo HID của màn hình cảm ứng.
- **Accessibility**: để phát sự kiện chuột/bàn phím và tìm nút gần nhất cho Chạm thông minh.

Bật cả hai cho TouchRelay rồi mở lại. Nếu app có trong danh sách mà vẫn bị từ chối, xoá bằng **−** rồi thêm lại.

## Cách dùng

TouchRelay nằm trên thanh menu (icon ☝︎, không có icon Dock). Các công tắc hay dùng ở cấp trên cùng, phần còn lại được gom nhóm:

- **Tạm dừng**, **Chạm thông minh**, **Ẩn con trỏ khi chạm**
- **Tốc độ cuộn ▸**, **Cuộn & cử chỉ ▸**, **Màn hình ▸** (màn hình đích, hướng, cỡ giao diện)
- **Công cụ ▸**: hiệu chỉnh, kiểm tra vùng chạm, kiểm tra màn hình, hướng dẫn cử chỉ, khắc phục sự cố, tuỳ chọn dòng lệnh, nhật ký
- **Cài đặt…** (⌘,), Mở cùng macOS, Giới thiệu, Thoát

Cài đặt lưu ở `~/Library/Application Support/touchrelay/settings.txt`, log ở `~/Library/Logs/touchrelay.log`.

### Chạm hơi lệch hoặc không tap được menu bar

1. Vào trang **Công cụ** trong Cài đặt, chạy **Kiểm tra vùng chạm**. Rê ngón khắp màn hình, kể cả bốn mép, rồi chạm 4 ngón để kết thúc. Nếu các mép vẫn tối là cảm biến không phủ tới đó; TouchRelay sẽ kéo vùng chạm với tới mép màn hình.
2. Nếu vẫn lệch, chạy **Hiệu chỉnh vị trí chạm** (9 dấu).
3. Nếu sau đó bạn đổi độ phân giải màn hình, hiệu chỉnh đã lưu tự tắt (nó chỉ áp dụng cho độ phân giải lúc hiệu chỉnh).

## Dòng lệnh

```bash
cargo build --release
./target/release/touchrelay --list     # liệt kê màn hình và thiết bị cảm ứng
./target/release/touchrelay --debug    # in layout, từng frame và hành động
```

Khi chạy từ Terminal, quyền thuộc về Terminal chứ không phải TouchRelay. Cờ hay dùng: `--display N`, `--swap-xy`, `--flip-x`, `--flip-y`, `--scroll-speed F`, `--invert-scroll`, `--one-finger-scroll`, `--no-momentum`, `--no-pinch`, `--no-swipe`, `--no-snap`, `--keep-cursor`, `--keep-mouse-interface`. Xem `touchrelay --help`.

## Hạn chế đã biết

- **macOS không có API touch native cho bên thứ ba.** macOS 27 có touch native cho phần cứng của Apple nhưng không có cách công khai để app khác phát hoặc đọc touch. `NSScreen.touchCapabilities` đo được `0` với màn hình bên thứ ba dùng ở đây, nên TouchRelay tổng hợp sự kiện chuột, cuộn và cử chỉ.
- **Không lên được Mac App Store**: sandbox chặn phát sự kiện, Accessibility API và chiếm độc quyền HID.
- **Pinch-zoom dùng gesture event riêng, không tài liệu hoá** của CoreGraphics, và việc ẩn con trỏ dùng một thuộc tính kết nối riêng. Cả hai có thể hỏng ở bản macOS sau.
- Chỉ hỗ trợ màn hình HID Digitizer chuẩn, không hỗ trợ driver riêng giao thức độc quyền.
- Vuốt 3 ngón gửi `Ctrl + mũi tên`, nên phải bật phím tắt Mission Control trong System Settings → Keyboard → Keyboard Shortcuts.
- Cỡ giao diện chỉ đổi trong phiên và được khôi phục khi thoát; nếu app bị tắt đột ngột nó có thể giữ nguyên cho đến khi bạn đổi lại ở System Settings → Displays.

## Đóng góp

Rất hoan nghênh issue và pull request. Nếu màn hình của bạn không chạy, hãy [mở issue](https://github.com/fpt-duongtx/touchrelay/issues) kèm kết quả:

```bash
./target/release/touchrelay --list
```

và dòng `ReportDescriptor (… byte): …` do `./target/release/touchrelay --debug` in ra (không chứa dữ liệu cá nhân). Khi sửa lỗi layout, hãy thêm test trong `src/touch.rs` dùng descriptor của bạn.

## Giấy phép và miễn trừ trách nhiệm

[MIT](LICENSE). Xem [miễn trừ trách nhiệm](DISCLAIMER.md): phần mềm cung cấp "nguyên trạng", không bảo hành, không liên kết với Apple hay hãng phần cứng nào.

Liên hệ: [GitHub Issues](https://github.com/fpt-duongtx/touchrelay/issues).
