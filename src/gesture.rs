//! Gesture engine: chuỗi frame cảm ứng -> hành động kiểu macOS.
//! Thuần Rust, không phụ thuộc macOS nên test được ở mọi nền tảng.
//!
//! | Cử chỉ                          | Kết quả                                   |
//! |---------------------------------|-------------------------------------------|
//! | Chạm 1 ngón                     | Click trái (chạm nhanh 2 lần = double)    |
//! | Giữ 1 ngón (~0.55s)             | Click phải                                |
//! | Kéo 1 ngón                      | Kéo chuột (chọn chữ, kéo cửa sổ)          |
//! |   hoặc với `--one-finger-scroll`| Cuộn như iPad                              |
//! | Kéo 2 ngón                      | Cuộn mượt có phase + quán tính (momentum) |
//! | Chạm 2 ngón                     | Click phải                                |
//! | Chụm/xoè 2 ngón                 | Pinch-zoom (magnify gesture)              |
//! | Vuốt 3 ngón trái/phải           | Chuyển Space (Ctrl+←/→)                   |
//! | Vuốt 3 ngón lên/xuống           | Mission Control / App Exposé              |

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct P {
    pub x: f64,
    pub y: f64,
}

impl P {
    pub fn new(x: f64, y: f64) -> Self {
        P { x, y }
    }
    pub fn dist(self, o: P) -> f64 {
        ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Touch {
    pub id: u32,
    /// Toạ độ global của macOS (point, gốc trên-trái màn hình chính).
    pub p: P,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Button {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollPhase {
    None,
    Began,
    Changed,
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MomentumPhase {
    None,
    Begin,
    Continue,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GesturePhase {
    Began,
    Changed,
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    MoveTo(P),
    MouseDown {
        button: Button,
        at: P,
        clicks: i64,
    },
    MouseUp {
        button: Button,
        at: P,
        clicks: i64,
    },
    Drag(P),
    /// dx/dy: quãng ngón tay di chuyển (point). Lớp macOS tự đổi sang dấu của wheel.
    Scroll {
        dx: f64,
        dy: f64,
        phase: ScrollPhase,
        momentum: MomentumPhase,
    },
    /// delta: hệ số phóng to tương đối (0.05 = to thêm 5%).
    Magnify {
        delta: f64,
        phase: GesturePhase,
        at: P,
    },
    /// Ctrl + phím mũi tên
    CtrlKey(Key),
}

#[derive(Clone, Debug)]
pub struct Config {
    pub tap_slop: f64,
    pub tap_max_time: f64,
    pub long_press_time: f64,
    /// Với kéo-1-ngón-để-cuộn: giữ yên ngón ≥ thời gian này rồi kéo thì thành kéo chuột (thanh trượt, chọn chữ).
    pub drag_hold_time: f64,
    pub double_click_time: f64,
    pub double_click_slop: f64,
    pub scroll_slop: f64,
    pub pinch_slop: f64,
    pub swipe_distance: f64,
    pub one_finger_scroll: bool,
    pub momentum: bool,
    /// Hệ số giảm tốc mỗi mili-giây (giống UIScrollView: 0.998 = normal).
    pub deceleration_per_ms: f64,
    pub min_momentum_speed: f64,
    pub pinch: bool,
    pub three_finger_swipe: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            tap_slop: 10.0,
            tap_max_time: 0.35,
            long_press_time: 0.55,
            drag_hold_time: 0.3,
            double_click_time: 0.4,
            double_click_slop: 25.0,
            scroll_slop: 8.0,
            pinch_slop: 25.0,
            swipe_distance: 120.0,
            one_finger_scroll: false,
            momentum: true,
            deceleration_per_ms: 0.998,
            min_momentum_speed: 150.0,
            pinch: true,
            three_finger_swipe: true,
        }
    }
}

#[derive(Clone, Debug)]
enum State {
    Idle,
    /// 1 ngón, chưa biết là tap, giữ hay kéo.
    One {
        id: u32,
        start: P,
        start_t: f64,
        /// Tổng vị trí các frame (kể cả frame đầu) và số frame, để click ở vị trí trung bình ổn định:
        /// frame đầu khi ngón mới tiếp xúc thường lệch nhất.
        sum: P,
        cnt: u32,
    },
    Drag {
        id: u32,
        cur: P,
    },
    /// 2 ngón, chưa biết là tap, cuộn hay pinch.
    Two {
        start_c: P,
        start_d: f64,
        start_t: f64,
    },
    Scroll(Tracker),
    Pinch {
        last_d: f64,
        at: P,
    },
    Three {
        start_c: P,
        done: bool,
    },
    /// Đã xử lý xong cử chỉ, chờ nhấc hết tay.
    Hold,
}

#[derive(Clone, Debug)]
struct Tracker {
    last: P,
    last_t: f64,
    last_move_t: f64,
    vx: f64,
    vy: f64,
    /// Số ngón tạo nên cử chỉ cuộn (1 hoặc 2)
    fingers: usize,
}

impl Tracker {
    fn new(p: P, now: f64, fingers: usize) -> Self {
        Tracker {
            last: p,
            last_t: now,
            last_move_t: now,
            vx: 0.0,
            vy: 0.0,
            fingers,
        }
    }
    fn step(&mut self, p: P, now: f64) -> (f64, f64) {
        let (dx, dy) = (p.x - self.last.x, p.y - self.last.y);
        let dt = (now - self.last_t).max(0.001);
        let a = 0.6;
        self.vx = a * dx / dt + (1.0 - a) * self.vx;
        self.vy = a * dy / dt + (1.0 - a) * self.vy;
        if dx != 0.0 || dy != 0.0 {
            self.last_move_t = now;
        }
        self.last = p;
        self.last_t = now;
        (dx, dy)
    }
}

#[derive(Clone, Debug)]
struct Momentum {
    vx: f64,
    vy: f64,
    last_t: f64,
    started: bool,
}

pub struct Engine {
    pub cfg: Config,
    state: State,
    momentum: Option<Momentum>,
    last_click: Option<(f64, P, i64)>,
}

fn centroid(t: &[Touch]) -> P {
    let n = t.len().max(1) as f64;
    P::new(
        t.iter().map(|t| t.p.x).sum::<f64>() / n,
        t.iter().map(|t| t.p.y).sum::<f64>() / n,
    )
}

fn spread(t: &[Touch]) -> f64 {
    if t.len() < 2 {
        0.0
    } else {
        t[0].p.dist(t[1].p)
    }
}

/// Điểm chạm ổn định: trung bình các frame sau frame đầu (nếu đủ ≥3 frame), ngược lại trung bình tất cả.
fn tap_point(start: P, sum: P, cnt: u32) -> P {
    if cnt >= 3 {
        let n = (cnt - 1) as f64;
        P::new((sum.x - start.x) / n, (sum.y - start.y) / n)
    } else if cnt >= 1 {
        P::new(sum.x / cnt as f64, sum.y / cnt as f64)
    } else {
        start
    }
}

impl Engine {
    pub fn new(cfg: Config) -> Self {
        Engine {
            cfg,
            state: State::Idle,
            momentum: None,
            last_click: None,
        }
    }

    fn click(&mut self, out: &mut Vec<Action>, button: Button, at: P, now: f64) {
        let clicks = match (button, self.last_click) {
            (Button::Left, Some((t, p, n)))
                if now - t <= self.cfg.double_click_time && p.dist(at) <= self.cfg.double_click_slop =>
            {
                n + 1
            }
            _ => 1,
        };
        out.push(Action::MoveTo(at));
        out.push(Action::MouseDown { button, at, clicks });
        out.push(Action::MouseUp { button, at, clicks });
        self.last_click = if button == Button::Left {
            Some((now, at, clicks))
        } else {
            None
        };
    }

    fn end_scroll(&mut self, out: &mut Vec<Action>, tr: &Tracker, now: f64) {
        out.push(Action::Scroll {
            dx: 0.0,
            dy: 0.0,
            phase: ScrollPhase::Ended,
            momentum: MomentumPhase::None,
        });
        // Ngón dừng lại trước khi nhấc -> không có quán tính.
        let paused = now - tr.last_move_t > 0.08;
        let speed = (tr.vx * tr.vx + tr.vy * tr.vy).sqrt();
        if self.cfg.momentum && !paused && speed >= self.cfg.min_momentum_speed {
            self.momentum = Some(Momentum {
                vx: tr.vx,
                vy: tr.vy,
                last_t: now,
                started: false,
            });
        }
    }

    fn enter(&mut self, touches: &[Touch], now: f64, out: &mut Vec<Action>) {
        self.state = match touches.len() {
            0 => State::Idle,
            1 => {
                out.push(Action::MoveTo(touches[0].p));
                State::One {
                    id: touches[0].id,
                    start: touches[0].p,
                    start_t: now,
                    sum: touches[0].p,
                    cnt: 1,
                }
            }
            2 => {
                let c = centroid(touches);
                out.push(Action::MoveTo(c));
                State::Two {
                    start_c: c,
                    start_d: spread(touches),
                    start_t: now,
                }
            }
            _ => State::Three {
                start_c: centroid(touches),
                done: false,
            },
        };
    }

    /// Gọi mỗi khi có frame mới (danh sách ngón đang chạm).
    pub fn update(&mut self, touches: &[Touch], now: f64) -> Vec<Action> {
        let mut out = Vec::new();
        let n = touches.len();

        // Chạm vào màn hình khi đang trượt quán tính -> dừng lại, giống native.
        if n > 0 {
            if let Some(m) = self.momentum.take() {
                if m.started {
                    out.push(Action::Scroll {
                        dx: 0.0,
                        dy: 0.0,
                        phase: ScrollPhase::None,
                        momentum: MomentumPhase::End,
                    });
                }
            }
        }

        let state = std::mem::replace(&mut self.state, State::Idle);
        match state {
            State::Idle => self.enter(touches, now, &mut out),

            State::One { id, start, start_t, sum, cnt } => {
                if n == 0 {
                    let at = tap_point(start, sum, cnt);
                    self.click(&mut out, Button::Left, at, now);
                    self.state = State::Idle;
                } else if n >= 2 {
                    self.enter(touches, now, &mut out);
                } else {
                    let t = touches.iter().find(|t| t.id == id).unwrap_or(&touches[0]);
                    if t.p.dist(start) > self.cfg.tap_slop {
                        if self.cfg.one_finger_scroll && now - start_t < self.cfg.drag_hold_time {
                            let mut tr = Tracker::new(start, start_t, 1);
                            let (dx, dy) = tr.step(t.p, now);
                            out.push(Action::Scroll {
                                dx,
                                dy,
                                phase: ScrollPhase::Began,
                                momentum: MomentumPhase::None,
                            });
                            self.state = State::Scroll(tr);
                        } else {
                            out.push(Action::MouseDown {
                                button: Button::Left,
                                at: start,
                                clicks: 1,
                            });
                            out.push(Action::Drag(t.p));
                            self.state = State::Drag { id: t.id, cur: t.p };
                        }
                    } else {
                        self.state = State::One {
                            id,
                            start,
                            start_t,
                            sum: P::new(sum.x + t.p.x, sum.y + t.p.y),
                            cnt: cnt + 1,
                        };
                    }
                }
            }

            State::Drag { id, cur } => match touches.iter().find(|t| t.id == id) {
                Some(t) => {
                    if t.p != cur {
                        out.push(Action::Drag(t.p));
                    }
                    self.state = State::Drag { id, cur: t.p };
                }
                None => {
                    out.push(Action::MouseUp {
                        button: Button::Left,
                        at: cur,
                        clicks: 1,
                    });
                    self.last_click = None;
                    self.state = if n == 0 { State::Idle } else { State::Hold };
                }
            },

            State::Two {
                start_c,
                start_d,
                start_t,
            } => {
                if n < 2 {
                    if now - start_t <= self.cfg.tap_max_time {
                        self.click(&mut out, Button::Right, start_c, now);
                    }
                    self.state = if n == 0 { State::Idle } else { State::Hold };
                } else if n >= 3 {
                    self.enter(touches, now, &mut out);
                } else {
                    let c = centroid(touches);
                    let d = spread(touches);
                    let dc = c.dist(start_c);
                    let dd = (d - start_d).abs();
                    if self.cfg.pinch && dd > self.cfg.pinch_slop && dd > dc && start_d > 1.0 {
                        out.push(Action::Magnify {
                            delta: d / start_d - 1.0,
                            phase: GesturePhase::Began,
                            at: c,
                        });
                        self.state = State::Pinch { last_d: d, at: c };
                    } else if dc > self.cfg.scroll_slop {
                        let mut tr = Tracker::new(start_c, start_t, 2);
                        let (dx, dy) = tr.step(c, now);
                        out.push(Action::Scroll {
                            dx,
                            dy,
                            phase: ScrollPhase::Began,
                            momentum: MomentumPhase::None,
                        });
                        self.state = State::Scroll(tr);
                    } else {
                        self.state = State::Two {
                            start_c,
                            start_d,
                            start_t,
                        };
                    }
                }
            }

            State::Scroll(mut tr) => {
                if n < tr.fingers {
                    self.end_scroll(&mut out, &tr, now);
                    self.state = if n == 0 { State::Idle } else { State::Hold };
                } else {
                    let c = if tr.fingers == 1 {
                        touches[0].p
                    } else {
                        centroid(&touches[..2])
                    };
                    let (dx, dy) = tr.step(c, now);
                    if dx != 0.0 || dy != 0.0 {
                        out.push(Action::Scroll {
                            dx,
                            dy,
                            phase: ScrollPhase::Changed,
                            momentum: MomentumPhase::None,
                        });
                    }
                    self.state = State::Scroll(tr);
                }
            }

            State::Pinch { last_d, at } => {
                if n < 2 {
                    out.push(Action::Magnify {
                        delta: 0.0,
                        phase: GesturePhase::Ended,
                        at,
                    });
                    self.state = if n == 0 { State::Idle } else { State::Hold };
                } else {
                    let d = spread(touches);
                    if last_d > 1.0 && d != last_d {
                        out.push(Action::Magnify {
                            delta: d / last_d - 1.0,
                            phase: GesturePhase::Changed,
                            at,
                        });
                    }
                    self.state = State::Pinch { last_d: d, at };
                }
            }

            State::Three { start_c, mut done } => {
                if n == 0 {
                    self.state = State::Idle;
                } else {
                    if !done && n >= 3 && self.cfg.three_finger_swipe {
                        let c = centroid(touches);
                        let (dx, dy) = (c.x - start_c.x, c.y - start_c.y);
                        let s = self.cfg.swipe_distance;
                        let key = if dx.abs() > s && dx.abs() > dy.abs() {
                            // Vuốt sang phải -> về Space bên trái (như trackpad)
                            Some(if dx > 0.0 { Key::Left } else { Key::Right })
                        } else if dy.abs() > s {
                            Some(if dy < 0.0 { Key::Up } else { Key::Down })
                        } else {
                            None
                        };
                        if let Some(k) = key {
                            out.push(Action::CtrlKey(k));
                            done = true;
                        }
                    }
                    self.state = State::Three { start_c, done };
                }
            }

            State::Hold => {
                self.state = if n == 0 { State::Idle } else { State::Hold };
            }
        }
        out
    }

    /// Gọi đều đặn (~120Hz): xử lý long-press và quán tính cuộn.
    pub fn tick(&mut self, now: f64) -> Vec<Action> {
        let mut out = Vec::new();

        if let State::One { id, start, start_t, sum, cnt } = self.state {
            let start = tap_point(start, sum, cnt);
            if self.cfg.one_finger_scroll {
                // Giữ yên đủ lâu = nhấn giữ nút chuột (sẵn sàng kéo thanh trượt / chọn chữ).
                // Nhả ra mà không kéo thì thành một click. Click phải dùng chạm 2 ngón.
                if now - start_t >= self.cfg.drag_hold_time {
                    out.push(Action::MoveTo(start));
                    out.push(Action::MouseDown {
                        button: Button::Left,
                        at: start,
                        clicks: 1,
                    });
                    self.last_click = None;
                    self.state = State::Drag { id, cur: start };
                }
            } else if now - start_t >= self.cfg.long_press_time {
                self.click(&mut out, Button::Right, start, now);
                self.state = State::Hold;
            }
        }

        if let Some(m) = self.momentum.as_mut() {
            let dt = now - m.last_t;
            if dt > 0.0 {
                let decay = self.cfg.deceleration_per_ms.powf(dt * 1000.0);
                // Quãng đường = tích phân v(t) trong khoảng dt
                let k = -(self.cfg.deceleration_per_ms.ln()) * 1000.0;
                let factor = if k > 0.0 { (1.0 - decay) / k } else { dt };
                let (dx, dy) = (m.vx * factor, m.vy * factor);
                m.vx *= decay;
                m.vy *= decay;
                m.last_t = now;
                let speed = (m.vx * m.vx + m.vy * m.vy).sqrt();
                if speed < 20.0 {
                    if m.started {
                        out.push(Action::Scroll {
                            dx: 0.0,
                            dy: 0.0,
                            phase: ScrollPhase::None,
                            momentum: MomentumPhase::End,
                        });
                    }
                    self.momentum = None;
                } else {
                    let phase = if m.started {
                        MomentumPhase::Continue
                    } else {
                        MomentumPhase::Begin
                    };
                    m.started = true;
                    out.push(Action::Scroll {
                        dx,
                        dy,
                        phase: ScrollPhase::None,
                        momentum: phase,
                    });
                }
            }
        }
        out
    }

    /// Nhả mọi nút/cử chỉ đang dở (khi thoát chương trình).
    pub fn reset(&mut self) -> Vec<Action> {
        let mut out = Vec::new();
        match std::mem::replace(&mut self.state, State::Idle) {
            State::Drag { cur, .. } => out.push(Action::MouseUp {
                button: Button::Left,
                at: cur,
                clicks: 1,
            }),
            State::Scroll(_) => out.push(Action::Scroll {
                dx: 0.0,
                dy: 0.0,
                phase: ScrollPhase::Ended,
                momentum: MomentumPhase::None,
            }),
            State::Pinch { at, .. } => out.push(Action::Magnify {
                delta: 0.0,
                phase: GesturePhase::Ended,
                at,
            }),
            _ => {}
        }
        if let Some(m) = self.momentum.take() {
            if m.started {
                out.push(Action::Scroll {
                    dx: 0.0,
                    dy: 0.0,
                    phase: ScrollPhase::None,
                    momentum: MomentumPhase::End,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(id: u32, x: f64, y: f64) -> Touch {
        Touch { id, p: P::new(x, y) }
    }

    fn has<F: Fn(&Action) -> bool>(a: &[Action], f: F) -> bool {
        a.iter().any(f)
    }

    #[test]
    fn tap_clicks_at_stable_mean_not_first_frame() {
        let mut e = Engine::new(Config::default());
        // Frame đầu lệch 6px (ngón mới chạm), các frame sau ổn định ở 100.
        e.update(&[t(1, 106.0, 100.0)], 0.0);
        e.update(&[t(1, 100.0, 100.0)], 0.02);
        e.update(&[t(1, 100.0, 100.0)], 0.04);
        e.update(&[t(1, 100.0, 100.0)], 0.06);
        let a = e.update(&[], 0.08);
        assert!(has(&a, |x| matches!(x, Action::MouseDown { at, .. } if at.dist(P::new(100.0, 100.0)) < 0.5)));
    }

    #[test]
    fn single_tap_clicks() {
        let mut e = Engine::new(Config::default());
        e.update(&[t(1, 100.0, 100.0)], 0.0);
        let a = e.update(&[], 0.08);
        assert!(has(&a, |x| matches!(
            x,
            Action::MouseDown {
                button: Button::Left,
                clicks: 1,
                ..
            }
        )));
        assert!(has(&a, |x| matches!(
            x,
            Action::MouseUp {
                button: Button::Left,
                ..
            }
        )));
    }

    #[test]
    fn double_tap_increments_click_count() {
        let mut e = Engine::new(Config::default());
        e.update(&[t(1, 100.0, 100.0)], 0.0);
        e.update(&[], 0.05);
        e.update(&[t(2, 103.0, 101.0)], 0.15);
        let a = e.update(&[], 0.2);
        assert!(has(&a, |x| matches!(x, Action::MouseDown { clicks: 2, .. })));
    }

    #[test]
    fn drag_one_finger() {
        let mut e = Engine::new(Config::default());
        e.update(&[t(1, 100.0, 100.0)], 0.0);
        let a = e.update(&[t(1, 150.0, 100.0)], 0.05);
        assert!(has(&a, |x| matches!(
            x,
            Action::MouseDown {
                button: Button::Left,
                ..
            }
        )));
        assert!(has(&a, |x| matches!(x, Action::Drag(_))));
        let a = e.update(&[], 0.1);
        assert!(has(&a, |x| matches!(x, Action::MouseUp { .. })));
    }

    #[test]
    fn long_press_right_clicks() {
        let mut e = Engine::new(Config::default());
        e.update(&[t(1, 100.0, 100.0)], 0.0);
        assert!(e.tick(0.3).is_empty());
        let a = e.tick(0.6);
        assert!(has(&a, |x| matches!(
            x,
            Action::MouseDown {
                button: Button::Right,
                ..
            }
        )));
        // Nhấc tay sau long-press không được click trái nữa
        assert!(e.update(&[], 0.7).is_empty());
    }

    #[test]
    fn two_finger_tap_right_clicks() {
        let mut e = Engine::new(Config::default());
        e.update(&[t(1, 100.0, 100.0), t(2, 160.0, 100.0)], 0.0);
        let a = e.update(&[], 0.1);
        assert!(has(&a, |x| matches!(
            x,
            Action::MouseDown {
                button: Button::Right,
                ..
            }
        )));
    }

    #[test]
    fn two_finger_scroll_with_momentum() {
        let mut e = Engine::new(Config::default());
        let mut now = 0.0;
        e.update(&[t(1, 100.0, 100.0), t(2, 160.0, 100.0)], now);
        let mut began = false;
        for i in 1..=10 {
            now += 0.008;
            let y = 100.0 + i as f64 * 12.0;
            let a = e.update(&[t(1, 100.0, y), t(2, 160.0, y)], now);
            began |= has(&a, |x| {
                matches!(
                    x,
                    Action::Scroll {
                        phase: ScrollPhase::Began,
                        ..
                    }
                )
            });
        }
        assert!(began);
        now += 0.008;
        let a = e.update(&[], now);
        assert!(has(&a, |x| matches!(
            x,
            Action::Scroll {
                phase: ScrollPhase::Ended,
                ..
            }
        )));
        // Quán tính
        let mut phases = vec![];
        for _ in 0..2000 {
            now += 0.008;
            for a in e.tick(now) {
                if let Action::Scroll { momentum, dy, .. } = a {
                    if momentum == MomentumPhase::Begin || momentum == MomentumPhase::Continue {
                        assert!(dy > 0.0);
                    }
                    phases.push(momentum);
                }
            }
        }
        assert_eq!(phases.first(), Some(&MomentumPhase::Begin));
        assert_eq!(phases.last(), Some(&MomentumPhase::End));
    }

    #[test]
    fn touching_stops_momentum() {
        let mut e = Engine::new(Config::default());
        e.update(&[t(1, 0.0, 0.0), t(2, 50.0, 0.0)], 0.0);
        for i in 1..=5 {
            let y = i as f64 * 20.0;
            e.update(&[t(1, 0.0, y), t(2, 50.0, y)], i as f64 * 0.008);
        }
        e.update(&[], 0.05);
        e.tick(0.06);
        let a = e.update(&[t(3, 10.0, 10.0)], 0.07);
        assert!(has(&a, |x| matches!(
            x,
            Action::Scroll {
                momentum: MomentumPhase::End,
                ..
            }
        )));
    }

    #[test]
    fn pinch_zoom() {
        let mut e = Engine::new(Config::default());
        e.update(&[t(1, 100.0, 100.0), t(2, 200.0, 100.0)], 0.0);
        let a = e.update(&[t(1, 70.0, 100.0), t(2, 230.0, 100.0)], 0.02);
        assert!(has(
            &a,
            |x| matches!(x, Action::Magnify { phase: GesturePhase::Began, delta, .. } if *delta > 0.5)
        ));
        let a = e.update(&[t(1, 60.0, 100.0), t(2, 240.0, 100.0)], 0.04);
        assert!(has(&a, |x| matches!(
            x,
            Action::Magnify {
                phase: GesturePhase::Changed,
                ..
            }
        )));
        let a = e.update(&[], 0.06);
        assert!(has(&a, |x| matches!(
            x,
            Action::Magnify {
                phase: GesturePhase::Ended,
                ..
            }
        )));
        assert!(!has(&a, |x| matches!(x, Action::MouseDown { .. })));
    }

    #[test]
    fn three_finger_swipe_left_fires_once() {
        let mut e = Engine::new(Config::default());
        let f = |dx: f64| {
            [
                t(1, 300.0 + dx, 300.0),
                t(2, 350.0 + dx, 300.0),
                t(3, 400.0 + dx, 300.0),
            ]
        };
        e.update(&f(0.0), 0.0);
        let a = e.update(&f(-150.0), 0.1);
        assert_eq!(a, vec![Action::CtrlKey(Key::Right)]);
        assert!(e.update(&f(-300.0), 0.2).is_empty());
        e.update(&[], 0.3);
    }

    #[test]
    fn three_finger_swipe_vertical_up_and_down() {
        let f = |dy: f64| {
            [
                t(1, 300.0, 300.0 + dy),
                t(2, 350.0, 300.0 + dy),
                t(3, 400.0, 300.0 + dy),
            ]
        };
        // Vuốt lên (y giảm) = Ctrl+Up = Mission Control, như trackpad.
        let mut e = Engine::new(Config::default());
        e.update(&f(0.0), 0.0);
        assert_eq!(e.update(&f(-150.0), 0.1), vec![Action::CtrlKey(Key::Up)]);
        e.update(&[], 0.3);
        // Vuốt xuống = Ctrl+Down = App Exposé.
        let mut e = Engine::new(Config::default());
        e.update(&f(0.0), 0.0);
        assert_eq!(e.update(&f(150.0), 0.1), vec![Action::CtrlKey(Key::Down)]);
        // Lệch chéo nhưng dọc trội hơn vẫn tính là vuốt dọc.
        let mut e = Engine::new(Config::default());
        e.update(&f(0.0), 0.0);
        let g = [t(1, 340.0, 150.0), t(2, 390.0, 150.0), t(3, 440.0, 150.0)];
        assert_eq!(e.update(&g, 0.1), vec![Action::CtrlKey(Key::Up)]);
    }

    #[test]
    fn one_finger_hold_then_drag_is_mouse_drag() {
        let cfg = Config {
            one_finger_scroll: true,
            ..Config::default()
        };
        let mut e = Engine::new(cfg);
        e.update(&[t(1, 100.0, 100.0)], 0.0);
        // Giữ yên: nhấn giữ nút, KHÔNG mở context menu dù giữ lâu hơn long_press_time.
        let a = e.tick(0.35);
        assert!(has(&a, |x| matches!(x, Action::MouseDown { button: Button::Left, .. })));
        let a = e.tick(0.8);
        assert!(!has(&a, |x| matches!(x, Action::MouseDown { button: Button::Right, .. })));
        let a = e.update(&[t(1, 100.0, 140.0)], 0.85);
        assert!(has(&a, |x| matches!(x, Action::Drag(_))));
        assert!(!has(&a, |x| matches!(x, Action::Scroll { .. })));
        // Nhả ra thì nhả nút.
        let a = e.update(&[], 0.9);
        assert!(has(&a, |x| matches!(x, Action::MouseUp { button: Button::Left, .. })));
    }

    #[test]
    fn one_finger_scroll_mode() {
        let cfg = Config {
            one_finger_scroll: true,
            ..Config::default()
        };
        let mut e = Engine::new(cfg);
        e.update(&[t(1, 100.0, 100.0)], 0.0);
        let a = e.update(&[t(1, 100.0, 140.0)], 0.02);
        assert!(has(&a, |x| matches!(
            x,
            Action::Scroll {
                phase: ScrollPhase::Began,
                ..
            }
        )));
        assert!(!has(&a, |x| matches!(x, Action::MouseDown { .. })));
    }
}
