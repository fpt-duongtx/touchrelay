//! Bản đồ vùng nhận chạm: canvas RGBA độ phân giải thấp, vẽ vệt vàng nơi ngón tay được cảm biến ghi nhận.
//! Tọa độ chuẩn hoá 0..1 (gốc trên-trái, như màn hình); điểm ngoài [0,1] bị bỏ qua nên dải mép
//! không nhận chạm sẽ để trống.

pub const BG: [u8; 4] = [28, 28, 30, 255];
pub const GRID: [u8; 4] = [52, 52, 56, 255];
pub const YELLOW: [u8; 4] = [255, 214, 10, 255];

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Canvas {
    /// Nền tối có lưới mờ mỗi 10% để dễ ước lượng vị trí.
    pub fn new(w: usize, h: usize) -> Canvas {
        let mut c = Canvas { w, h, px: vec![0; w * h * 4] };
        for y in 0..h {
            for x in 0..w {
                let on_grid = (0..=10).any(|i| {
                    let gx = (i as f64 / 10.0 * (w - 1) as f64).round() as usize;
                    let gy = (i as f64 / 10.0 * (h - 1) as f64).round() as usize;
                    x == gx || y == gy
                });
                c.set(x, y, if on_grid { GRID } else { BG });
            }
        }
        c
    }

    fn set(&mut self, x: usize, y: usize, c: [u8; 4]) {
        let i = (y * self.w + x) * 4;
        self.px[i..i + 4].copy_from_slice(&c);
    }

    #[cfg(test)]
    pub fn get(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.w + x) * 4;
        [self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]]
    }

    fn to_px(&self, p: (f64, f64)) -> (f64, f64) {
        (p.0 * (self.w - 1) as f64, p.1 * (self.h - 1) as f64)
    }

    fn disc(&mut self, cx: f64, cy: f64, r: f64) {
        let (x0, x1) = ((cx - r).floor().max(0.0) as usize, (cx + r).ceil().min((self.w - 1) as f64) as usize);
        let (y0, y1) = ((cy - r).floor().max(0.0) as usize, (cy + r).ceil().min((self.h - 1) as f64) as usize);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2) <= r * r {
                    self.set(x, y, YELLOW);
                }
            }
        }
    }

    /// Vẽ một vệt từ `a` đến `b` (nội suy để không đứt nét khi ngón rê nhanh). `None` = ngoài màn hình.
    pub fn stroke(&mut self, a: Option<(f64, f64)>, b: Option<(f64, f64)>, radius_px: f64) {
        let inside = |p: (f64, f64)| (0.0..=1.0).contains(&p.0) && (0.0..=1.0).contains(&p.1);
        let Some(b) = b.filter(|&p| inside(p)) else { return };
        let (bx, by) = self.to_px(b);
        match a.filter(|&p| inside(p)) {
            Some(a) => {
                let (ax, ay) = self.to_px(a);
                let steps = ((bx - ax).hypot(by - ay) / (radius_px * 0.5).max(0.5)).ceil().max(1.0) as usize;
                for i in 0..=steps {
                    let t = i as f64 / steps as f64;
                    self.disc(ax + (bx - ax) * t, ay + (by - ay) * t, radius_px);
                }
            }
            None => self.disc(bx, by, radius_px),
        }
    }

    /// Tỉ lệ pixel đã vẽ vàng (0..1), dùng cho test.
    #[cfg(test)]
    pub fn painted_fraction(&self) -> f64 {
        let n = (0..self.h).flat_map(|y| (0..self.w).map(move |x| (x, y))).filter(|&(x, y)| self.get(x, y) == YELLOW).count();
        n as f64 / (self.w * self.h) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_dark_with_grid_and_no_yellow() {
        let c = Canvas::new(64, 40);
        assert_eq!(c.painted_fraction(), 0.0);
        assert_eq!(c.get(0, 5), GRID); // đường lưới ở mép trái
        assert_eq!(c.get(5, 5), BG);
    }

    #[test]
    fn stroke_paints_line_without_gaps() {
        let mut c = Canvas::new(100, 50);
        c.stroke(Some((0.1, 0.5)), Some((0.9, 0.5)), 2.0);
        // Mọi cột giữa hai đầu đều có pixel vàng ở hàng giữa.
        for x in 12..88 {
            assert_eq!(c.get(x, 24), YELLOW, "đứt nét ở x={x}");
        }
        // Vùng xa vệt vẫn tối.
        assert_ne!(c.get(50, 5), YELLOW);
    }

    #[test]
    fn points_outside_screen_are_ignored() {
        let mut c = Canvas::new(64, 40);
        c.stroke(None, Some((-0.1, 0.5)), 3.0);
        c.stroke(Some((0.5, 0.5)), Some((0.5, 1.2)), 3.0);
        assert_eq!(c.painted_fraction(), 0.0);
    }

    #[test]
    fn single_dot_without_previous_point() {
        let mut c = Canvas::new(64, 40);
        c.stroke(None, Some((0.5, 0.5)), 3.0);
        assert_eq!(c.get(31, 19), YELLOW);
        assert!(c.painted_fraction() > 0.0);
    }
}
