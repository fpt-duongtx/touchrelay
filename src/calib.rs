//! Hiệu chỉnh vị trí chạm: ánh xạ từng đoạn (piecewise-linear) cho mỗi trục, từ toạ độ cảm biến
//! (chuẩn hoá 0..1, sau xoay/lật) sang toạ độ màn hình chuẩn hoá 0..1.
//!
//! Cảm biến thật hay phi tuyến ở gần rìa (ILITEK: dấu ở 8% trên cùng đọc 0.048, tâm 0.503, 92% dưới 0.98),
//! nên affine không đủ. Mỗi trục có vài "nút" (toạ độ cảm biến → toạ độ đích), nội suy tuyến tính giữa các nút
//! và ngoại suy tuyến tính theo đoạn ngoài cùng. Hai trục độc lập (đã kiểm tra: ghép chéo < 1%).

#[derive(Clone, Debug, PartialEq)]
pub struct AxisMap {
    /// (cảm biến, đích), tăng dần theo cảm biến.
    pub knots: Vec<(f64, f64)>,
}

impl AxisMap {
    pub fn apply(&self, v: f64) -> f64 {
        let k = &self.knots;
        let n = k.len();
        if n < 2 {
            return v;
        }
        let seg = if v <= k[0].0 {
            0
        } else if v >= k[n - 1].0 {
            n - 2
        } else {
            k.windows(2).position(|w| v >= w[0].0 && v <= w[1].0).unwrap_or(0)
        };
        let (a, b) = (k[seg], k[seg + 1]);
        let slope = (b.1 - a.1) / (b.0 - a.0);
        a.1 + (v - a.0) * slope
    }
}

impl AxisMap {
    /// Thêm hai nút ở rìa vùng cảm biến thật sự nhận chạm (`min`, `max` đo được) ánh xạ vào mép màn hình 0 và 1,
    /// để dải sát mép không nhận chạm (vd. menu bar) vẫn chạm tới được.
    pub fn with_extent(&self, min: f64, max: f64) -> AxisMap {
        let mut k = self.knots.clone();
        if let (Some(first), Some(last)) = (k.first().copied(), k.last().copied()) {
            if (0.0..first.0 - 0.003).contains(&min) {
                k.insert(0, (min, 0.0));
            }
            if (last.0 + 0.003..=1.0).contains(&max) {
                k.push((max, 1.0));
            }
        }
        AxisMap { knots: k }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Calib {
    pub x: AxisMap,
    pub y: AxisMap,
}

impl Calib {
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.x.apply(x), self.y.apply(y))
    }

    /// Ánh xạ đồng nhất (khi chưa hiệu chỉnh nhưng đã đo vùng nhận chạm).
    pub fn identity() -> Calib {
        let id = AxisMap { knots: vec![(0.0, 0.0), (1.0, 1.0)] };
        Calib { x: id.clone(), y: id }
    }

    /// `e` = [min x, max x, min y, max y] của toạ độ cảm biến thật sự đạt tới.
    pub fn with_extents(&self, e: [f64; 4]) -> Calib {
        Calib { x: self.x.with_extent(e[0], e[1]), y: self.y.with_extent(e[2], e[3]) }
    }

    /// Dạng phẳng để lưu: [n, (cảm biến, đích)×n cho x, m, (cảm biến, đích)×m cho y].
    pub fn to_vec(&self) -> Vec<f64> {
        let mut v = vec![self.x.knots.len() as f64];
        v.extend(self.x.knots.iter().flat_map(|k| [k.0, k.1]));
        v.push(self.y.knots.len() as f64);
        v.extend(self.y.knots.iter().flat_map(|k| [k.0, k.1]));
        v
    }

    pub fn from_slice(v: &[f64]) -> Option<Calib> {
        let take = |i: usize| -> Option<(AxisMap, usize)> {
            let n = *v.get(i)? as usize;
            if !(2..=16).contains(&n) || v.len() < i + 1 + 2 * n {
                return None;
            }
            let knots: Vec<(f64, f64)> = (0..n).map(|j| (v[i + 1 + 2 * j], v[i + 2 + 2 * j])).collect();
            if knots.windows(2).any(|w| w[1].0 <= w[0].0) {
                return None;
            }
            Some((AxisMap { knots }, i + 1 + 2 * n))
        };
        let (x, next) = take(0)?;
        let (y, _) = take(next)?;
        Some(Calib { x, y })
    }
}

/// Kết quả hiệu chỉnh: ánh xạ và sai số lớn nhất (tỉ lệ cạnh màn hình).
pub struct Fit {
    pub calib: Calib,
    pub max_error: f64,
}

fn knots_for(samples: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    // Gom theo giá trị đích, lấy trung bình giá trị cảm biến của mỗi nhóm.
    let mut groups: Vec<(f64, Vec<f64>)> = vec![];
    for &(sensor, target) in samples {
        match groups.iter_mut().find(|g| (g.0 - target).abs() < 1e-6) {
            Some(g) => g.1.push(sensor),
            None => groups.push((target, vec![sensor])),
        }
    }
    groups.sort_by(|a, b| a.0.total_cmp(&b.0));
    if groups.len() < 3 {
        return None;
    }
    let knots: Vec<(f64, f64)> = groups
        .iter()
        .map(|(t, s)| (s.iter().sum::<f64>() / s.len() as f64, *t))
        .collect();
    // Cảm biến phải tăng ngặt theo đích, nếu không là chạm nhầm.
    if knots.windows(2).any(|w| w[1].0 <= w[0].0 + 1e-4) {
        return None;
    }
    Some(knots)
}

/// `samples`: ((x, y) cảm biến, (u, v) vị trí đích). Mỗi trục cần ≥3 giá trị đích khác nhau.
pub fn fit_calib(samples: &[((f64, f64), (f64, f64))]) -> Option<Fit> {
    let xs: Vec<(f64, f64)> = samples.iter().map(|&((x, _), (u, _))| (x, u)).collect();
    let ys: Vec<(f64, f64)> = samples.iter().map(|&((_, y), (_, v))| (y, v)).collect();
    let calib = Calib { x: AxisMap { knots: knots_for(&xs)? }, y: AxisMap { knots: knots_for(&ys)? } };
    let max_error = samples
        .iter()
        .map(|&((x, y), (u, v))| {
            let (pu, pv) = calib.apply(x, y);
            ((pu - u).powi(2) + (pv - v).powi(2)).sqrt()
        })
        .fold(0.0, f64::max);
    Some(Fit { calib, max_error })
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: [f64; 5] = [0.03, 0.265, 0.5, 0.735, 0.97];

    /// 9 điểm: đường chéo chính + đường chéo phụ (mỗi giá trị trục có 2 mẫu, trừ tâm có 1).
    fn targets() -> Vec<(f64, f64)> {
        let mut v: Vec<(f64, f64)> = T.iter().map(|&t| (t, t)).collect();
        v.extend([(T[0], T[4]), (T[1], T[3]), (T[3], T[1]), (T[4], T[0])]);
        v
    }

    #[test]
    fn identity_when_sensor_is_accurate() {
        let pairs: Vec<_> = targets().iter().map(|&t| (t, t)).collect();
        let fit = fit_calib(&pairs).unwrap();
        assert!(fit.max_error < 1e-9);
        let (u, v) = fit.calib.apply(0.3, 0.7);
        assert!((u - 0.3).abs() < 1e-9 && (v - 0.7).abs() < 1e-9);
    }

    #[test]
    fn corrects_nonlinear_axis() {
        // Mô phỏng ILITEK trục Y: 0.08→0.048, 0.5→0.503, 0.92→0.98 (phi tuyến, 2 đoạn khác độ dốc).
        let sensor_y = |v: f64| if v < 0.5 { 0.048 + (v - 0.08) * (0.503 - 0.048) / 0.42 } else { 0.503 + (v - 0.5) * (0.98 - 0.503) / 0.42 };
        let pairs: Vec<_> = targets().iter().map(|&(u, v)| ((u, sensor_y(v)), (u, v))).collect();
        let fit = fit_calib(&pairs).unwrap();
        assert!(fit.max_error < 1e-9, "sai số {}", fit.max_error);
        // Một điểm giữa hai nút cũng được sửa đúng (mô hình phi tuyến theo đoạn).
        let (_, v) = fit.calib.apply(0.3, sensor_y(0.3));
        assert!((v - 0.3).abs() < 0.01, "v={v}");
        // Ngoại suy sát rìa: chạm ở 1% dưới nút ngoài cùng vẫn đi đúng hướng.
        let (_, v) = fit.calib.apply(0.3, sensor_y(0.01));
        assert!(v < 0.03, "v={v}");
    }

    #[test]
    fn inconsistent_taps_report_error() {
        let mut pairs: Vec<_> = targets().iter().map(|&t| (t, t)).collect();
        pairs[2].0 = (0.62, 0.5); // một lần chạm sai giữa hai mẫu cùng giá trị
        pairs[6].0 = (0.38, 0.5);
        let fit = fit_calib(&pairs).unwrap();
        assert!(fit.max_error > 0.05);
    }

    #[test]
    fn non_monotonic_or_too_few_fail() {
        let pairs: Vec<_> = targets().iter().map(|&t| (t, t)).collect();
        assert!(fit_calib(&pairs[..2]).is_none());
        let mut bad = pairs.clone();
        // Cảm biến đảo thứ tự ở trục x
        for p in bad.iter_mut() {
            p.0 .0 = 1.0 - p.0 .0;
        }
        assert!(fit_calib(&bad).is_none());
    }

    #[test]
    fn extents_stretch_reachable_edge_to_screen_edge() {
        // Cảm biến chỉ chạm tới y∈[0.012, 0.99]; sau hiệu chỉnh nút đầu ở (0.048→0.06).
        let c = Calib {
            x: AxisMap { knots: vec![(0.05, 0.06), (0.5, 0.5), (0.95, 0.94)] },
            y: AxisMap { knots: vec![(0.048, 0.06), (0.503, 0.5), (0.98, 0.94)] },
        };
        let e = c.with_extents([0.01, 0.99, 0.012, 0.99]);
        assert!(e.y.apply(0.012).abs() < 1e-9, "mép trên phải về 0");
        assert!((e.y.apply(0.99) - 1.0).abs() < 1e-9, "mép dưới phải về 1");
        // Phần giữa không đổi.
        assert!((e.y.apply(0.503) - 0.5).abs() < 1e-9);
        // Không có thay đổi nếu vùng đo được không rộng hơn nút ngoài cùng.
        let same = c.with_extents([0.06, 0.9, 0.06, 0.9]);
        assert_eq!(same, c);
        assert_eq!(Calib::identity().with_extents([0.0, 1.0, 0.0, 1.0]), Calib::identity());
    }

    #[test]
    fn roundtrip_serialization() {
        let pairs: Vec<_> = targets().iter().map(|&t| (t, t)).collect();
        let c = fit_calib(&pairs).unwrap().calib;
        let back = Calib::from_slice(&c.to_vec()).unwrap();
        assert_eq!(c, back);
        assert!(Calib::from_slice(&[1.0, 0.1, 0.1]).is_none());
        assert!(Calib::from_slice(&[]).is_none());
    }
}
