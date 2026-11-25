//! touchrelay — biến màn hình cảm ứng rời thành thao tác native trên macOS.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

mod calib;
mod coverage;
mod display;
mod i18n;
mod input_source;
mod snap;
mod gesture;
mod hid_desc;
mod touch;

#[cfg(target_os = "macos")]
mod ffi;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod menubar;

use std::path::PathBuf;

pub const DEFAULT_SCROLL_SPEED: f64 = 1.2;

/// Cài đặt chỉnh được từ menu, lưu ở ~/Library/Application Support/touchrelay/settings.txt.
#[derive(Clone, Debug, PartialEq)]
pub struct Prefs {
    pub scroll_speed: f64,
    pub invert_scroll: bool,
    pub one_finger_scroll: bool,
    pub momentum: bool,
    pub pinch: bool,
    pub swipe: bool,
    /// 0 = ngang, 1 = dọc (xoay phải 90°), 2 = dọc (xoay trái 90°).
    pub orientation: u8,
    /// None = tự chọn màn hình rời.
    pub display: Option<usize>,
    pub hide_cursor: bool,
    /// Hút vào nút gần nhất khi chạm trượt (cần Accessibility).
    pub snap: bool,
    /// Kéo 1 ngón thông minh: tự chọn kéo chuột hay cuộn theo thứ nằm dưới ngón tay (Accessibility).
    pub smart_drag: bool,
    /// Tự hiện bàn phím ảo khi chạm vào ô nhập văn bản.
    pub keyboard: bool,
    /// Cỡ bàn phím ảo: 0 nhỏ, 1 vừa (mặc định), 2 lớn.
    pub kbd_size: u8,
    /// Độ nhạy vuốt nhiều ngón: 0 nhạy, 1 vừa (mặc định), 2 khó.
    pub swipe_level: u8,
    /// Ngôn ngữ giao diện: 0 theo hệ thống (mặc định), 1 English, 2 Tiếng Việt.
    pub language: u8,
    /// Hiệu chỉnh vị trí chạm (các nút ánh xạ từng đoạn theo trục); None = chưa hiệu chỉnh.
    pub calib: Option<Vec<f64>>,
    /// Vùng cảm biến thật sự nhận chạm [min x, max x, min y, max y] (từ "Kiểm tra vùng chạm").
    pub extent: Option<Vec<f64>>,
    /// Độ phân giải (điểm) của màn hình đích lúc hiệu chỉnh; đổi độ phân giải thì hiệu chỉnh tự tắt.
    pub calib_size: Option<(u32, u32)>,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            scroll_speed: DEFAULT_SCROLL_SPEED,
            invert_scroll: false,
            one_finger_scroll: true,
            momentum: true,
            pinch: true,
            swipe: true,
            orientation: 0,
            display: None,
            hide_cursor: true,
            snap: true,
            smart_drag: true,
            keyboard: true,
            kbd_size: 1,
            swipe_level: 1,
            language: 0,
            calib: None,
            extent: None,
            calib_size: None,
        }
    }
}

/// Chuỗi số cách nhau bằng dấu phẩy -> dữ liệu hiệu chỉnh (None nếu không hợp lệ).
/// "WxH" -> (W, H).
pub fn parse_size(v: &str) -> Option<(u32, u32)> {
    let (w, h) = v.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

/// "minx,maxx,miny,maxy" (mỗi giá trị 0..1, min < max).
pub fn parse_extent(v: &str) -> Option<Vec<f64>> {
    let n: Vec<f64> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    (n.len() == 4 && n.iter().all(|x| (0.0..=1.0).contains(x)) && n[0] < n[1] && n[2] < n[3]).then_some(n)
}

pub fn parse_calib(v: &str) -> Option<Vec<f64>> {
    let n: Vec<f64> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    calib::Calib::from_slice(&n).map(|_| n)
}

pub fn settings_file_path() -> PathBuf {
    home().join("Library/Application Support/touchrelay/settings.txt")
}

impl Prefs {
    pub fn load() -> Prefs {
        let mut p = Prefs::default();
        let Ok(text) = std::fs::read_to_string(settings_file_path()) else {
            return p;
        };
        for l in text.lines() {
            let Some((k, v)) = l.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let b = v == "1";
            match k {
                "scroll_speed" => {
                    if let Ok(x) = v.parse::<f64>() {
                        p.scroll_speed = x.clamp(0.05, 5.0);
                    }
                }
                "invert_scroll" => p.invert_scroll = b,
                "one_finger_scroll" => p.one_finger_scroll = b,
                "momentum" => p.momentum = b,
                "pinch" => p.pinch = b,
                "swipe" => p.swipe = b,
                "hide_cursor" => p.hide_cursor = b,
                "snap" => p.snap = b,
                "smart_drag" => p.smart_drag = b,
                "keyboard" => p.keyboard = b,
                "language" => p.language = v.parse::<u8>().unwrap_or(0).min(2),
                "swipe_level" => p.swipe_level = v.parse::<u8>().unwrap_or(1).min(2),
                "kbd_size" => p.kbd_size = v.parse::<u8>().unwrap_or(1).min(2),
                "orientation" => p.orientation = v.parse::<u8>().unwrap_or(0).min(2),
                "display" => p.display = v.parse().ok(),
                "calib" => p.calib = parse_calib(v),
                "extent" => p.extent = parse_extent(v),
                "calib_size" => p.calib_size = parse_size(v),
                _ => {}
            }
        }
        p
    }

    pub fn save(&self) {
        let path = settings_file_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let b = |x: bool| if x { 1 } else { 0 };
        let text = format!(
            "scroll_speed={}\ninvert_scroll={}\none_finger_scroll={}\nmomentum={}\npinch={}\nswipe={}\norientation={}\nhide_cursor={}\nsnap={}\nsmart_drag={}\nkeyboard={}\nkbd_size={}\nswipe_level={}\nlanguage={}\n{}",
            self.scroll_speed,
            b(self.invert_scroll),
            b(self.one_finger_scroll),
            b(self.momentum),
            b(self.pinch),
            b(self.swipe),
            self.orientation,
            b(self.hide_cursor),
            b(self.snap),
            b(self.smart_drag),
            b(self.keyboard),
            self.kbd_size,
            self.swipe_level,
            self.language,
            self.display.map(|d| format!("display={d}\n")).unwrap_or_default()
        ) + &self
            .calib_size
            .map(|(w, h)| format!("calib_size={w}x{h}\n"))
            .unwrap_or_default()
            + &self
            .extent
            .as_ref()
            .map(|c| format!("extent={}\n", c.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")))
            .unwrap_or_default()
            + &self
            .calib
            .as_ref()
            .map(|c| format!("calib={}\n", c.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")))
            .unwrap_or_default();
        let _ = std::fs::write(path, text);
    }

    /// Chuyển thành cờ dòng lệnh; đặt TRƯỚC file tuỳ chọn nâng cao để file đó ghi đè được.
    pub fn to_flags(&self) -> Vec<String> {
        let mut f = vec!["--scroll-speed".to_string(), self.scroll_speed.to_string()];
        if self.invert_scroll {
            f.push("--invert-scroll".into());
        }
        if self.one_finger_scroll {
            f.push("--one-finger-scroll".into());
        }
        if !self.momentum {
            f.push("--no-momentum".into());
        }
        if !self.pinch {
            f.push("--no-pinch".into());
        }
        if !self.swipe {
            f.push("--no-swipe".into());
        }
        if !self.hide_cursor {
            f.push("--keep-cursor".into());
        }
        if !self.snap {
            f.push("--no-snap".into());
        }
        if !self.smart_drag {
            f.push("--no-smart-drag".into());
        }
        if !self.keyboard {
            f.push("--no-keyboard".into());
        }
        if self.language != 0 {
            f.extend(["--lang".into(), if self.language == 1 { "en".into() } else { "vi".into() }]);
        }
        if self.swipe_level != 1 {
            f.extend(["--swipe-level".into(), self.swipe_level.to_string()]);
        }
        if self.kbd_size != 1 {
            f.extend(["--kbd-size".into(), self.kbd_size.to_string()]);
        }
        match self.orientation {
            1 => f.extend(["--swap-xy".into(), "--flip-x".into()]),
            2 => f.extend(["--swap-xy".into(), "--flip-y".into()]),
            _ => {}
        }
        if let Some(d) = self.display {
            f.extend(["--display".into(), d.to_string()]);
        }
        if let Some((w, h)) = self.calib_size {
            f.extend(["--calib-size".into(), format!("{w}x{h}")]);
        }
        if let Some(c) = &self.extent {
            f.extend(["--extent".into(), c.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")]);
        }
        if let Some(c) = &self.calib {
            f.extend(["--calib".into(), c.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")]);
        }
        f
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub list: bool,
    pub debug: bool,
    pub display: Option<usize>,
    pub vid: Option<i64>,
    pub pid: Option<i64>,
    pub swap_xy: bool,
    pub flip_x: bool,
    pub flip_y: bool,
    pub invert_scroll: bool,
    pub scroll_scale: f64,
    pub no_seize: bool,
    /// Ẩn con trỏ khi đang dùng cảm ứng (hiện lại khi chuột/trackpad thật di chuyển).
    pub hide_cursor: bool,
    /// Hút vào nút gần nhất khi chạm trượt.
    pub snap: bool,
    /// Kéo 1 ngón thông minh (kéo chuột hay cuộn theo ngữ cảnh).
    pub smart_drag: bool,
    /// Tự hiện bàn phím ảo khi chạm vào ô nhập.
    pub keyboard: bool,
    /// Cỡ bàn phím ảo (0 nhỏ, 1 vừa, 2 lớn).
    pub kbd_size: u8,
    /// Độ nhạy vuốt nhiều ngón (0 nhạy, 1 vừa, 2 khó).
    pub swipe_level: u8,
    /// Ngôn ngữ giao diện: 0 theo hệ thống, 1 English, 2 Tiếng Việt.
    pub language: u8,
    /// Hiệu chỉnh vị trí chạm (các nút ánh xạ từng đoạn theo trục).
    pub calib: Option<Vec<f64>>,
    pub extent: Option<Vec<f64>>,
    pub calib_size: Option<(u32, u32)>,
    pub keep_mouse_interface: bool,
    /// Chạy dạng app thanh menu (tự bật khi mở TouchRelay.app).
    pub app_mode: bool,
    /// Mở cửa sổ cài đặt ngay khi khởi động (chế độ menu bar).
    pub open_settings: bool,
    /// Với --settings: mở sẵn trang số N (0 = Chung...).
    pub settings_page: Option<usize>,
    /// Chụp cửa sổ Cài đặt ra file PNG (kiểm tra giao diện), cần --settings.
    pub snapshot: Option<String>,
    /// Hiện bàn phím ảo ngay khi khởi động (để kiểm tra).
    pub show_keyboard: bool,
    pub gesture: gesture::Config,
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// File tuỳ chọn khi chạy dạng .app (không có dòng lệnh).
/// Chép cài đặt từ thư mục cũ (tên app trước khi đổi sang TouchRelay) nếu chưa có.
pub fn migrate_old_files() {
    let old = home().join("Library/Application Support/touchmac");
    let new = home().join("Library/Application Support/touchrelay");
    if !old.exists() || new.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&new);
    for f in ["settings.txt", "options.txt"] {
        let _ = std::fs::copy(old.join(f), new.join(f));
    }
}

pub fn options_file_path() -> PathBuf {
    home().join("Library/Application Support/touchrelay/options.txt")
}

pub fn log_file_path() -> PathBuf {
    home().join("Library/Logs/touchrelay.log")
}

pub const OPTIONS_FILE_TEMPLATE: &str = "\
# Tuỳ chọn cho TouchRelay.app – viết các cờ dòng lệnh, cách nhau bằng dấu cách/xuống dòng.
# Dòng bắt đầu bằng # là ghi chú. Sửa xong thì Thoát và mở lại TouchRelay.
#
# Ví dụ:
#   --display 1
#   --swap-xy --flip-x
#   --scroll-speed 1.5
#   --invert-scroll
#   --no-pinch
#   --long-press 0.7
#   --debug
";

fn load_options_file() -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(options_file_path()) else {
        return vec![];
    };
    text.lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .flat_map(|l| l.split_whitespace())
        .map(String::from)
        .collect()
}

/// Đang được mở từ bên trong một bundle .app (Finder / Login Items)?
fn launched_from_bundle() -> bool {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().contains(".app/Contents/MacOS/"))
        .unwrap_or(false)
}

/// Ghi stderr ra ~/Library/Logs/touchrelay.log (app không có Terminal).
#[cfg(target_os = "macos")]
fn redirect_stderr_to_log() {
    use std::os::fd::AsRawFd;
    extern "C" {
        fn dup2(src: i32, dst: i32) -> i32;
    }
    let path = log_file_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // Log quá 1MB thì làm mới.
    let big = std::fs::metadata(&path).map(|m| m.len() > 1_000_000).unwrap_or(false);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(!big)
        .write(true)
        .truncate(big)
        .open(&path);
    if let Ok(f) = file {
        unsafe { dup2(f.as_raw_fd(), 2) };
        std::mem::forget(f);
    }
}

const HELP: &str = "\
touchrelay – dùng màn hình cảm ứng rời như thao tác native trên macOS

CÁCH DÙNG:
    touchrelay [tuỳ chọn]

TUỲ CHỌN:
    --list                  Liệt kê màn hình & thiết bị cảm ứng rồi thoát
    --display <N>           Chọn màn hình đích theo số thứ tự trong --list
                            (mặc định: màn hình rời đầu tiên)
    --vid <hex> --pid <hex> Chỉ dùng thiết bị có VendorID/ProductID này
    --swap-xy               Đổi trục X/Y (màn hình xoay dọc)
    --flip-x / --flip-y     Lật trục
    --one-finger-scroll     Kéo 1 ngón = cuộn (kiểu iPad); giữ yên 0.3s rồi kéo = kéo chuột
    --scroll-speed <f>      Hệ số tốc độ cuộn (mặc định 1.2)
    --invert-scroll         Đảo chiều cuộn
    --no-momentum           Tắt cuộn quán tính
    --no-pinch              Tắt pinch-zoom
    --no-swipe              Tắt vuốt 3 ngón
    --long-press <giây>     Thời gian giữ để click phải (mặc định 0.55)
    --keep-mouse-interface  Không chặn interface chuột giả lập của màn hình
    --calib <số,...>        Hiệu chỉnh vị trí chạm (thường do app tự lưu)
    --lang <system|en|vi>   Ngôn ngữ giao diện (mặc định: theo hệ thống)
    --swipe-level <0|1|2>   Độ nhạy vuốt 3-4 ngón: 0 nhạy, 1 vừa (mặc định), 2 khó (cần vuốt xa hơn)
    --kbd-size <0|1|2>      Cỡ bàn phím ảo: 0 nhỏ, 1 vừa (mặc định), 2 lớn
    --no-keyboard           Tắt bàn phím ảo tự hiện khi chạm vào ô nhập
    --no-smart-drag         Tắt kéo 1 ngón thông minh (luôn cuộn, giữ 0.3s mới kéo chuột)
    --no-snap               Tắt chạm thông minh (hút vào nút gần nhất)
    --keep-cursor           Không ẩn con trỏ khi chạm
    --no-seize              Không chiếm độc quyền thiết bị (debug)
    --show-keyboard         Như --menubar và hiện luôn bàn phím ảo
    --settings              Như --menubar và mở luôn cửa sổ cài đặt
    --menubar               Chạy kiểu app thanh menu (mặc định khi mở TouchRelay.app)
    --debug                 In layout HID, từng frame và hành động
    -h, --help              Trợ giúp
";

fn parse_hex(s: &str) -> Option<i64> {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    i64::from_str_radix(s, 16).ok()
}

fn parse_args(argv: Vec<String>) -> Result<Options, String> {
    let mut o = Options {
        list: false,
        debug: false,
        display: None,
        vid: None,
        pid: None,
        swap_xy: false,
        flip_x: false,
        flip_y: false,
        invert_scroll: false,
        scroll_scale: DEFAULT_SCROLL_SPEED,
        no_seize: false,
        hide_cursor: true,
        snap: true,
        smart_drag: true,
        keyboard: true,
        kbd_size: 1,
        swipe_level: 1,
        language: 0,
        calib: None,
        extent: None,
        calib_size: None,
        keep_mouse_interface: false,
        app_mode: false,
        open_settings: false,
        settings_page: None,
        snapshot: None,
        show_keyboard: false,
        gesture: gesture::Config::default(),
    };
    let mut args = argv.into_iter();
    while let Some(a) = args.next() {
        let mut val = |name: &str| args.next().ok_or(format!("{name} cần giá trị"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "--list" => o.list = true,
            "--debug" => o.debug = true,
            "--display" => o.display = Some(val("--display")?.parse().map_err(|_| "--display phải là số")?),
            "--vid" => o.vid = Some(parse_hex(&val("--vid")?).ok_or("--vid phải là hex")?),
            "--pid" => o.pid = Some(parse_hex(&val("--pid")?).ok_or("--pid phải là hex")?),
            "--swap-xy" => o.swap_xy = true,
            "--flip-x" => o.flip_x = true,
            "--flip-y" => o.flip_y = true,
            "--one-finger-scroll" => o.gesture.one_finger_scroll = true,
            "--scroll-speed" => {
                o.scroll_scale = val("--scroll-speed")?
                    .parse()
                    .map_err(|_| "--scroll-speed phải là số")?
            }
            "--invert-scroll" => o.invert_scroll = true,
            "--no-momentum" => o.gesture.momentum = false,
            "--no-pinch" => o.gesture.pinch = false,
            "--no-swipe" => o.gesture.three_finger_swipe = false,
            "--long-press" => {
                o.gesture.long_press_time = val("--long-press")?.parse().map_err(|_| "--long-press phải là số")?
            }
            "--keep-mouse-interface" => o.keep_mouse_interface = true,
            "--no-seize" => o.no_seize = true,
            "--keep-cursor" => o.hide_cursor = false,
            "--no-snap" => o.snap = false,
            "--no-smart-drag" => o.smart_drag = false,
            "--no-keyboard" => o.keyboard = false,
            "--lang" => {
                o.language = match val("--lang")?.as_str() {
                    "system" | "auto" => 0,
                    "en" => 1,
                    "vi" => 2,
                    _ => return Err("--lang phải là system, en hoặc vi".into()),
                }
            }
            "--swipe-level" => o.swipe_level = val("--swipe-level")?.parse::<u8>().map_err(|_| "--swipe-level phải là 0, 1 hoặc 2")?.min(2),
            "--kbd-size" => o.kbd_size = val("--kbd-size")?.parse::<u8>().map_err(|_| "--kbd-size phải là 0, 1 hoặc 2")?.min(2),
            "--calib-size" => o.calib_size = Some(parse_size(&val("--calib-size")?).ok_or("--calib-size không hợp lệ")?),
            "--extent" => o.extent = Some(parse_extent(&val("--extent")?).ok_or("--extent không hợp lệ (dữ liệu do app tự lưu)")?),
            "--calib" => o.calib = Some(parse_calib(&val("--calib")?).ok_or("--calib không hợp lệ (dữ liệu do app tự lưu)")?),
            "--menubar" => o.app_mode = true,
            "--show-keyboard" => {
                o.app_mode = true;
                o.show_keyboard = true;
            }
            "--snapshot" => o.snapshot = Some(val("--snapshot")?),
            "--settings-page" => o.settings_page = Some(val("--settings-page")?.parse().map_err(|_| "--settings-page phải là số")?),
            "--settings" => {
                o.app_mode = true;
                o.open_settings = true;
            }
            // macOS cũ truyền -psn_... khi mở từ Finder
            psn if psn.starts_with("-psn_") => {}
            other => return Err(format!("Tuỳ chọn không hợp lệ: {other}\n\n{HELP}")),
        }
    }
    Ok(o)
}

fn main() {
    migrate_old_files();
    let cli: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with("-psn_")).collect();
    // Mở bằng click đúp: không có tham số dòng lệnh -> đọc file tuỳ chọn, bật chế độ menu bar.
    let bundle = launched_from_bundle() && cli.is_empty();
    let argv = if bundle {
        let mut a = Prefs::load().to_flags();
        a.extend(load_options_file());
        a
    } else if cli.iter().any(|a| a == "--menubar" || a == "--settings" || a == "--show-keyboard" || a == "--settings-page" || a == "--snapshot") {
        // Chạy dạng app từ dòng lệnh: cũng dùng cài đặt đã lưu (cờ dòng lệnh ghi đè).
        let mut a = Prefs::load().to_flags();
        a.extend(cli);
        a
    } else {
        cli
    };

    #[cfg(target_os = "macos")]
    if bundle {
        redirect_stderr_to_log();
        eprintln!("\n=== touchrelay khởi động, tuỳ chọn: {argv:?}");
    }

    let opts = match parse_args(argv) {
        Ok(mut o) => {
            o.app_mode |= bundle;
            o
        }
        Err(e) => {
            eprintln!("{e}");
            #[cfg(target_os = "macos")]
            if bundle {
                menubar::init_app();
                let first = e.lines().next().unwrap_or("").to_string();
                menubar::alert(
                    &i18n::tr("File tuỳ chọn có lỗi"),
                    &format!("{first}\n\nSửa file:\n{}", options_file_path().display()),
                );
            }
            std::process::exit(2);
        }
    };

    #[cfg(target_os = "macos")]
    std::process::exit(macos::run(opts));

    #[cfg(not(target_os = "macos"))]
    {
        let _ = opts;
        eprintln!("touchrelay chỉ chạy trên macOS.");
        std::process::exit(1);
    }
}
