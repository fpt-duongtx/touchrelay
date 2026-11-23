---
name: macos-settings-ui
description: Thiết kế/sửa giao diện cho TouchRelay theo phong cách System Settings của Apple (sidebar + form nhóm), căn lề/padding rõ ràng, control native, tương thích cảm ứng và Liquid Glass. Dùng khi sửa src/menubar.rs (cửa sổ cài đặt, menu, hộp thoại, About) hoặc thêm một màn hình/cài đặt mới.
---

# UI kiểu Apple System Settings cho TouchRelay

Mọi UI nằm trong `src/menubar.rs` (AppKit gọi qua `objc_msgSend`, không có Swift/SwiftUI). Chuỗi hiển thị luôn qua `tr()` (`src/i18n.rs`, khoá tiếng Việt + bảng EN). Không thể chụp màn hình từ shell của Claude: nhờ người dùng gửi ảnh để kiểm tra bố cục.

## Cấu trúc cửa sổ Cài đặt
- Cửa sổ 780×560, titlebar trong suốt. **Sidebar trái 200pt** (kiểu source list, vibrancy) + **vùng chi tiết bên phải** (nền `windowBackgroundColor`).
- Sidebar: mỗi dòng cao 32pt, icon SF Symbol 18pt + nhãn 13pt, lề trong 12pt. Chọn dòng nào hiện trang đó ở vùng chi tiết.
- Vùng chi tiết là `NSScrollView`: trang dài thì cuộn được; `show_page` đo chiều cao bằng lượt dựng thử (`build_page`) rồi dựng thật, nên cứ thêm hàng mới, không cần tính tay chiều cao cửa sổ.
- Trang chi tiết: tiêu đề trang 22pt semibold, lề ngoài **24pt**, khoảng cách tiêu đề → nhóm đầu 20pt.
- Mỗi trang gồm các **nhóm form** (rounded group): hộp bo góc 10pt, nền `quaternarySystemFillColor` + viền 0.5pt `separatorColor`; bên trong là các hàng cách nhau bằng đường kẻ mảnh thụt lề 16pt.
- Chú thích của nhóm (footnote) đặt **dưới** hộp, chữ 11–12pt màu `secondaryLabelColor`, lề trái 16pt cho thẳng với chữ trong hộp.

## Lưới và khoảng cách (bội số của 4/8)
- Lề trang 24; lề trong nhóm 16 (ngang), hàng cao 44 (một dòng) hoặc 56 (tiêu đề + mô tả); khoảng cách giữa hai nhóm 20.
- Tiêu đề hàng 13pt regular (màu `labelColor`); mô tả hàng 11pt `secondaryLabelColor`, tối đa 2 dòng.
- Control nằm **mép phải** nhóm (cách mép 16), căn giữa dọc theo hàng; nhãn nằm mép trái. Tiêu đề và mô tả cùng một lề trái, không thụt lệch.
- Cùng loại control thì cùng chiều rộng (popup 200pt, switch mặc định). Không trộn chữ căn trái/giữa trong cùng nhóm.

## Control: dùng loại native, đừng tự vẽ
- Bật/tắt → `NSSwitch`. Chọn một trong nhiều → `NSPopUpButton` (hoặc `NSSegmentedControl` khi ≤ 4 lựa chọn ngắn). Hành động → `NSButton` (bezel Glass = 16 trên macOS 26+, Push = 1 trước đó).
- Mục tiêu chạm tối thiểu 44pt: hàng ≥ 44pt cao, dùng `controlSize` Large (3) cho nút/segmented quan trọng. Màn hình cảm ứng là đối tượng chính.
- Hover/nhấn/focus do hệ thống xử lý: **không** dùng nút square tự vẽ.

## Màu, chữ, vật liệu
- Chỉ dùng màu ngữ nghĩa (`labelColor`, `secondaryLabelColor`, `separatorColor`, `controlAccentColor`), tự đúng ở Dark mode. Không hardcode RGB.
- Chữ dùng `NSFont systemFont…` / `boldSystemFont…`; tiêu đề trang 22 semibold, tiêu đề nhóm 13 semibold, thân 13, mô tả 11.
- Liquid Glass (macOS 26+): sidebar dùng `NSVisualEffectView` (material sidebar); nút hành động bezel Glass. Kiểm tra `class("NSGlassEffectView")` không null trước khi dùng API mới.

## Nội dung và địa phương hoá
- Nhãn ngắn, danh từ/cụm động từ, không dấu chấm cuối; mô tả là câu hoàn chỉnh giải thích **hệ quả** của cài đặt.
- Mọi chuỗi qua `tr("khoá tiếng Việt")`, thêm bản EN vào `EN` trong `src/i18n.rs`; test kiểm tra khoá trùng. Không ghép chuỗi bằng nối câu: dùng placeholder `{app}`, `{d}`…
- Tên app lấy từ `APP_NAME`; email liên hệ từ `CONTACT_EMAIL`.

## Quy trình khi thêm một cài đặt
1. Thêm trường vào `Prefs` (`src/main.rs`) + `to_flags()` nếu cần cờ dòng lệnh, và vào `App::prefs()`.
2. Thêm tag công tắc/hàm `set_*` dùng chung giữa menu và cửa sổ (`flip(tag)`, `set_speed`…), lưu bằng `save_prefs()`.
3. Thêm hàng vào trang phù hợp bằng helper hàng/nhóm; thêm mô tả + bản dịch EN.
4. `cargo test`, `INSTALL=1 ./scripts/make-app.sh`, rồi nhờ người dùng kiểm tra bằng mắt.

## Lỗi thường gặp
- Frame AppKit gốc ở **góc dưới trái**: tính vị trí từ mép trên bằng `top - h`.
- `NSTextField` nhãn nhiều dòng phải dùng `wrappingLabelWithString:`.
- Đổi cửa sổ/menu từ callback phải chạy trên main thread (callback AppKit đã ở main).
- Menu dựng lại khi ngôn ngữ đổi (`menuNeedsUpdate:`); cửa sổ dựng lại mỗi lần mở.
