# Disclaimer / Miễn trừ trách nhiệm

[English](#english) · [Tiếng Việt](#tiếng-việt) · Web version: <https://fpt-duongtx.github.io/touchrelay/disclaimer.html>

*Last updated: 2026-11-23. This page is information for users, not legal advice.*

## English

**1. No warranty.** TouchRelay for Mac ("the software") is released under the [MIT License](LICENSE) and is provided **"as is"**, without warranty of any kind, express or implied, including fitness for a particular purpose, merchantability, non-infringement, or that it will work with your hardware or with any macOS version. You use it at your own risk. To the fullest extent permitted by law, the authors and contributors are not liable for any claim, damage or loss arising from the use of, or inability to use, the software.

**2. Not affiliated.** TouchRelay is an independent, unofficial project. It is **not affiliated with, endorsed by or sponsored by Apple Inc., ILITEK, or any touchscreen, monitor or computer manufacturer**. Apple, Mac, macOS, MacBook, Mission Control, Safari and other names are trademarks of their respective owners. Product names are used only to describe compatibility.

**3. What the software does on your Mac.** To work, it asks for two macOS permissions and uses them as follows:
- *Input Monitoring*: reads the raw reports of the touchscreen (a HID device). It does not read keystrokes or any other device on purpose.
- *Accessibility*: posts mouse, scroll, gesture and keyboard events (for example `Ctrl + arrow` for three-finger swipes), and, if "Smart tap" is on, asks other apps' accessibility interface for the control nearest to a tap.
- It takes **exclusive control** of the touchscreen (and its fake mouse interface) while running, so the device stops behaving as a normal mouse. Quitting the app releases it.
- It may **hide the mouse pointer** while you touch the screen.
- The optional "Interface size" feature **temporarily changes the display mode** (resolution) of your touchscreen. It is restored when the app quits, but if the app is killed or crashes the mode may stay until you change it in System Settings → Displays. Changing display modes can briefly blank the screen or, on unsupported hardware, render it blurry.
- It stores its settings and a log file under `~/Library/Application Support/touchrelay/` and `~/Library/Logs/`. The log may contain the name, vendor/product IDs and HID descriptor of your touchscreen. Check it before you share it.

**4. Privacy.** The software has **no network code**: it does not collect, transmit or sell any data, and it has no analytics or auto-update. If you contact the maintainer or open an issue, the information you send is handled by the channel you use (for example GitHub).

**5. Private and unsupported interfaces.** Pinch-to-zoom and some pointer behaviour rely on **undocumented macOS interfaces**. Apple may change or remove them in any update, which can break features or cause unexpected behaviour. The software is not designed for, and must not be relied on in, safety-critical, medical, industrial or other situations where a malfunction could cause harm.

**6. Unsigned builds.** Prebuilt downloads are **ad-hoc signed and not notarized by Apple**. macOS will warn when you open them. Only run a prebuilt app if you trust this repository, verify the published SHA-256 checksum, or build it yourself from the source. Do not install copies from other sources.

**7. Your data and settings.** Tools such as "Touch calibration", "Touch area test" and "Reset calibration" change or delete only TouchRelay's own settings. You are responsible for your own backups.

**8. Third-party content.** Links to external sites are provided for convenience; the maintainer is not responsible for their content.

**9. Changes.** This disclaimer may be updated at any time. Continued use of the software means you accept the current version.

Questions or reports: [GitHub Issues](https://github.com/fpt-duongtx/touchrelay/issues).

## Tiếng Việt

**1. Không bảo hành.** TouchRelay for Mac ("phần mềm") được phát hành theo [giấy phép MIT](LICENSE) và được cung cấp **"nguyên trạng"**, không có bất kỳ bảo đảm nào, rõ ràng hay ngầm định, gồm cả việc phù hợp cho một mục đích cụ thể, khả năng thương mại, không vi phạm quyền, hoặc việc phần mềm sẽ chạy với phần cứng hay phiên bản macOS của bạn. Bạn tự chịu rủi ro khi sử dụng. Trong phạm vi pháp luật cho phép, tác giả và người đóng góp không chịu trách nhiệm về bất kỳ khiếu nại, thiệt hại hay mất mát nào phát sinh từ việc dùng hoặc không dùng được phần mềm.

**2. Không liên kết.** TouchRelay là dự án độc lập, không chính thức. **Không liên kết, không được bảo trợ hay xác nhận bởi Apple Inc., ILITEK hay bất kỳ nhà sản xuất màn hình cảm ứng, màn hình hoặc máy tính nào.** Apple, Mac, macOS, MacBook, Mission Control, Safari và các tên khác là nhãn hiệu của chủ sở hữu tương ứng. Tên sản phẩm chỉ dùng để mô tả khả năng tương thích.

**3. Phần mềm làm gì trên máy của bạn.** Để hoạt động, phần mềm xin hai quyền của macOS và dùng như sau:
- *Input Monitoring*: đọc báo cáo thô của màn hình cảm ứng (một thiết bị HID). Phần mềm không chủ ý đọc phím gõ hay thiết bị nào khác.
- *Accessibility*: phát sự kiện chuột, cuộn, cử chỉ và bàn phím (ví dụ `Ctrl + mũi tên` cho vuốt 3 ngón), và nếu bật "Chạm thông minh" thì hỏi giao diện trợ năng của app khác để tìm điều khiển gần điểm chạm nhất.
- Khi chạy, nó **chiếm quyền điều khiển độc quyền** màn hình cảm ứng (và giao diện chuột giả lập của nó), nên thiết bị không còn hoạt động như chuột thông thường. Thoát app là nhả ra.
- Nó có thể **ẩn con trỏ chuột** khi bạn chạm vào màn hình.
- Tính năng tuỳ chọn "Cỡ giao diện" **tạm đổi chế độ hiển thị** (độ phân giải) của màn hình cảm ứng. Chế độ này được khôi phục khi thoát app, nhưng nếu app bị tắt đột ngột hoặc lỗi, chế độ có thể giữ nguyên cho đến khi bạn đổi lại ở System Settings → Displays. Đổi chế độ hiển thị có thể làm màn hình tối tạm thời, hoặc bị mờ trên phần cứng không hỗ trợ.
- Phần mềm lưu cài đặt và file log trong `~/Library/Application Support/touchrelay/` và `~/Library/Logs/`. Log có thể chứa tên, mã nhà sản xuất/sản phẩm và HID descriptor của màn hình cảm ứng; hãy kiểm tra trước khi chia sẻ.

**4. Quyền riêng tư.** Phần mềm **không có mã kết nối mạng**: không thu thập, gửi hay bán dữ liệu, không có phân tích hay tự cập nhật. Khi bạn liên hệ hoặc mở issue, thông tin bạn gửi do kênh bạn dùng xử lý (ví dụ GitHub).

**5. Giao diện riêng, không được hỗ trợ.** Pinch-zoom và một số hành vi con trỏ dựa trên **giao diện macOS không có tài liệu**. Apple có thể đổi hoặc gỡ chúng trong bất kỳ bản cập nhật nào, làm tính năng hỏng hoặc hoạt động bất ngờ. Phần mềm không được thiết kế cho, và không nên dựa vào trong, các tình huống đòi hỏi an toàn, y tế, công nghiệp hay nơi sự cố có thể gây hại.

**6. Bản build chưa ký.** Các bản tải sẵn được **ký ad-hoc và chưa được Apple notarize**; macOS sẽ cảnh báo khi mở. Chỉ chạy bản tải sẵn nếu bạn tin repository này, đã kiểm tra mã SHA-256 được công bố, hoặc tự build từ mã nguồn. Không cài các bản sao từ nguồn khác.

**7. Dữ liệu và cài đặt của bạn.** Các công cụ như "Hiệu chỉnh vị trí chạm", "Kiểm tra vùng chạm" và "Đặt lại hiệu chỉnh" chỉ thay đổi hoặc xoá cài đặt của chính TouchRelay. Bạn tự chịu trách nhiệm sao lưu dữ liệu của mình.

**8. Nội dung bên thứ ba.** Liên kết đến trang ngoài chỉ để thuận tiện; người duy trì không chịu trách nhiệm về nội dung của các trang đó.

**9. Thay đổi.** Miễn trừ này có thể được cập nhật bất kỳ lúc nào. Tiếp tục sử dụng phần mềm nghĩa là bạn chấp nhận phiên bản hiện hành.

Câu hỏi hoặc báo lỗi: [GitHub Issues](https://github.com/fpt-duongtx/touchrelay/issues).
