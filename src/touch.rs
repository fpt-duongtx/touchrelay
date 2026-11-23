//! Từ HID descriptor -> layout các "ngón tay" -> danh sách contact mỗi frame.
//!
//! Hỗ trợ cả 2 kiểu báo cáo của touchscreen chuẩn Windows (HID Digitizer):
//! - Parallel mode: mọi ngón nằm trong 1 report, kèm Contact Count.
//! - Hybrid mode: report đầu mang Contact Count = N, các report sau mang 0,
//!   gom đủ N contact mới thành 1 frame.

use crate::hid_desc::{extract, Descriptor, Field};
use std::collections::HashMap;

pub const PAGE_GENERIC_DESKTOP: u16 = 0x01;
pub const PAGE_DIGITIZER: u16 = 0x0D;
const USAGE_X: u16 = 0x30;
const USAGE_Y: u16 = 0x31;
pub const USAGE_TOUCH_SCREEN: u16 = 0x04;
const USAGE_FINGER: u16 = 0x22;
const USAGE_TIP_SWITCH: u16 = 0x42;
const USAGE_CONFIDENCE: u16 = 0x47;
const USAGE_CONTACT_ID: u16 = 0x51;
const USAGE_DEVICE_MODE: u16 = 0x52;
const USAGE_CONTACT_COUNT: u16 = 0x54;

#[derive(Debug, Clone)]
pub struct Slot {
    pub tip: Option<Field>,
    pub contact_id: Option<Field>,
    pub confidence: Option<Field>,
    pub x: Field,
    pub y: Field,
}

#[derive(Debug, Default, Clone)]
pub struct ReportLayout {
    pub slots: Vec<Slot>,
    pub contact_count: Option<Field>,
}

#[derive(Debug, Clone)]
pub struct TouchLayout {
    pub reports: HashMap<u8, ReportLayout>,
    pub uses_report_ids: bool,
    /// Feature "Device Mode" (0x0D/0x52). Một số màn hình cần set = 2 mới gửi multitouch.
    pub device_mode: Option<Field>,
    /// Feature "Contact Count Maximum" (0x0D/0x55). ILITEK chỉ gửi multitouch sau khi host ghi giá trị này.
    pub contact_count_max: Option<Field>,
    pub feature_report_bits: HashMap<u8, u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub id: u32,
    pub tip: bool,
    /// Toạ độ chuẩn hoá 0..1
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedReport {
    pub contacts: Vec<Contact>,
    pub contact_count: Option<u32>,
}

fn is(f: &Field, page: u16, usage: u16) -> bool {
    f.usage_page == page && f.usage == usage
}

fn norm(v: i32, f: &Field) -> f64 {
    let (a, b) = (f.logical_min as f64, f.logical_max as f64);
    if b <= a {
        return 0.0;
    }
    ((v as f64 - a) / (b - a)).clamp(0.0, 1.0)
}

impl TouchLayout {
    pub fn from_descriptor(d: &Descriptor) -> Option<Self> {
        let mut order: Vec<usize> = Vec::new();
        let mut groups: HashMap<usize, Vec<&Field>> = HashMap::new();
        let mut counts: HashMap<u8, Field> = HashMap::new();

        for f in &d.inputs {
            if d.in_collection(f, PAGE_DIGITIZER, USAGE_TOUCH_SCREEN).is_none() {
                continue;
            }
            if is(f, PAGE_DIGITIZER, USAGE_CONTACT_COUNT) {
                counts.entry(f.report_id).or_insert_with(|| f.clone());
                continue;
            }
            // Ưu tiên nhóm theo collection "Finger"; X/Y đôi khi nằm trong
            // physical collection con của nó.
            let Some(key) = d.in_collection(f, PAGE_DIGITIZER, USAGE_FINGER).or(f.collection) else {
                continue;
            };
            if !groups.contains_key(&key) {
                order.push(key);
            }
            groups.entry(key).or_default().push(f);
        }

        let mut reports: HashMap<u8, ReportLayout> = HashMap::new();
        for key in order {
            let fs = &groups[&key];
            let find = |p, u| fs.iter().find(|f| is(f, p, u)).map(|f| (*f).clone());
            let (Some(x), Some(y)) = (find(PAGE_GENERIC_DESKTOP, USAGE_X), find(PAGE_GENERIC_DESKTOP, USAGE_Y)) else {
                continue;
            };
            let slot = Slot {
                tip: find(PAGE_DIGITIZER, USAGE_TIP_SWITCH),
                contact_id: find(PAGE_DIGITIZER, USAGE_CONTACT_ID),
                confidence: find(PAGE_DIGITIZER, USAGE_CONFIDENCE),
                x,
                y,
            };
            reports.entry(slot.x.report_id).or_default().slots.push(slot);
        }
        if reports.is_empty() {
            return None;
        }
        for (rid, f) in counts {
            if let Some(r) = reports.get_mut(&rid) {
                r.contact_count = Some(f);
            }
        }
        let device_mode = d
            .features
            .iter()
            .find(|f| is(f, PAGE_DIGITIZER, USAGE_DEVICE_MODE))
            .cloned();

        Some(TouchLayout {
            reports,
            uses_report_ids: d.uses_report_ids,
            device_mode,
            contact_count_max: d
                .features
                .iter()
                .find(|f| is(f, PAGE_DIGITIZER, 0x55))
                .cloned(),
            feature_report_bits: d.feature_report_bits.clone(),
        })
    }

    pub fn max_slots(&self) -> usize {
        self.reports.values().map(|r| r.slots.len()).sum()
    }

    /// `data` là raw report đúng như IOKit đưa (có byte report ID ở đầu nếu dùng ID).
    pub fn parse_report(&self, data: &[u8]) -> Option<ParsedReport> {
        let (rid, payload) = if self.uses_report_ids {
            (*data.first()?, &data[1..])
        } else {
            (0u8, data)
        };
        let layout = self.reports.get(&rid)?;
        let mut contacts = Vec::with_capacity(layout.slots.len());
        for (i, s) in layout.slots.iter().enumerate() {
            let (Some(xv), Some(yv)) = (extract(payload, &s.x), extract(payload, &s.y)) else {
                continue;
            };
            let tip = s
                .tip
                .as_ref()
                .and_then(|f| extract(payload, f))
                .map(|v| v != 0)
                .unwrap_or(true);
            // Confidence = 0 nghĩa là lòng bàn tay / chạm không chủ đích.
            let confident = s
                .confidence
                .as_ref()
                .and_then(|f| extract(payload, f))
                .map(|v| v != 0)
                .unwrap_or(true);
            let id = s
                .contact_id
                .as_ref()
                .and_then(|f| extract(payload, f))
                .map(|v| v as u32)
                .unwrap_or(i as u32);
            contacts.push(Contact {
                id,
                tip: tip && confident,
                x: norm(xv, &s.x),
                y: norm(yv, &s.y),
            });
        }
        let contact_count = layout
            .contact_count
            .as_ref()
            .and_then(|f| extract(payload, f))
            .map(|v| v.max(0) as u32);
        Some(ParsedReport {
            contacts,
            contact_count,
        })
    }
}

/// Gom report thành frame hoàn chỉnh. Trả về danh sách ngón ĐANG chạm (tip = 1).
#[derive(Default, Debug)]
pub struct FrameAssembler {
    pending: Vec<Contact>,
    expected: usize,
}

impl FrameAssembler {
    pub fn push(&mut self, r: ParsedReport) -> Option<Vec<Contact>> {
        match r.contact_count {
            None => return Some(r.contacts.into_iter().filter(|c| c.tip).collect()),
            Some(n) if n > 0 => {
                self.pending.clear();
                self.expected = n as usize;
            }
            Some(_) => {
                if self.expected == 0 {
                    // Count = 0 mà không đang gom frame: không còn ngón nào.
                    return Some(Vec::new());
                }
            }
        }
        let remaining = self.expected.saturating_sub(self.pending.len());
        self.pending.extend(r.contacts.into_iter().take(remaining));
        if self.pending.len() >= self.expected {
            self.expected = 0;
            let frame = std::mem::take(&mut self.pending);
            return Some(frame.into_iter().filter(|c| c.tip).collect());
        }
        None
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::hid_desc::parse;

    fn finger_block(d: &mut Vec<u8>) {
        d.extend_from_slice(&[
            0x05, 0x0D, // Usage Page (Digitizer)
            0x09, 0x22, // Usage (Finger)
            0xA1, 0x02, // Collection (Logical)
            0x09, 0x42, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x01, 0x81, 0x02, // Tip Switch
            0x09, 0x47, 0x81, 0x02, // Confidence (1 bit)
            0x95, 0x06, 0x81, 0x03, // 6 bit padding
            0x75, 0x08, 0x25, 0x7F, 0x09, 0x51, 0x95, 0x01, 0x81, 0x02, // Contact ID
            0x05, 0x01, 0x26, 0xFF, 0x0F, 0x75, 0x10, // GD, max 4095, 16 bit
            0x09, 0x30, 0x09, 0x31, 0x95, 0x02, 0x81, 0x02, // X, Y
            0xC0,
        ]);
    }

    /// Touchscreen kiểu Windows: report 1 chứa 2 ngón + contact count,
    /// report 3 là feature Device Mode.
    pub fn sample_descriptor() -> Vec<u8> {
        let mut d = vec![0x05, 0x0D, 0x09, 0x04, 0xA1, 0x01, 0x85, 0x01];
        finger_block(&mut d);
        finger_block(&mut d);
        d.extend_from_slice(&[
            0x05, 0x0D, 0x09, 0x54, 0x15, 0x00, 0x25, 0x7F, 0x75, 0x08, 0x95, 0x01, 0x81, 0x02, // Contact count
            0x85, 0x02, 0x09, 0x55, 0x25, 0x02, 0xB1, 0x02, // Feature: contact count max
            0xC0, // Device configuration collection
            0x05, 0x0D, 0x09, 0x0E, 0xA1, 0x01, 0x85, 0x03, 0x09, 0x23, 0xA1, 0x02, 0x09, 0x52, 0x09, 0x53, 0x15, 0x00,
            0x25, 0x0A, 0x75, 0x08, 0x95, 0x02, 0xB1, 0x02, 0xC0, 0xC0,
        ]);
        d
    }

    pub fn report(fingers: &[(bool, u8, u16, u16)], count: u8) -> Vec<u8> {
        let mut r = vec![0x01];
        for i in 0..2 {
            let (tip, id, x, y) = fingers.get(i).copied().unwrap_or((false, 0, 0, 0));
            r.push(if tip { 0b11 } else { 0 });
            r.push(id);
            r.extend_from_slice(&x.to_le_bytes());
            r.extend_from_slice(&y.to_le_bytes());
        }
        r.push(count);
        r
    }

    #[test]
    fn layout_from_sample() {
        let d = parse(&sample_descriptor()).unwrap();
        let l = TouchLayout::from_descriptor(&d).unwrap();
        assert!(l.uses_report_ids);
        let r = &l.reports[&1];
        assert_eq!(r.slots.len(), 2);
        assert!(r.contact_count.is_some());
        assert_eq!(r.slots[0].x.bit_offset, 16);
        assert_eq!(r.slots[1].x.bit_offset, 16 + 48);
        assert_eq!(r.slots[0].x.logical_max, 4095);
        let dm = l.device_mode.as_ref().unwrap();
        assert_eq!((dm.report_id, dm.bit_offset), (3, 0));
    }

    #[test]
    fn parse_parallel_report() {
        let d = parse(&sample_descriptor()).unwrap();
        let l = TouchLayout::from_descriptor(&d).unwrap();
        let p = l
            .parse_report(&report(&[(true, 5, 4095, 0), (true, 6, 2048, 4095)], 2))
            .unwrap();
        assert_eq!(p.contact_count, Some(2));
        assert_eq!(p.contacts[0].id, 5);
        assert!((p.contacts[0].x - 1.0).abs() < 1e-9);
        assert!((p.contacts[1].y - 1.0).abs() < 1e-9);
        let mut asm = FrameAssembler::default();
        assert_eq!(asm.push(p).unwrap().len(), 2);
    }

    #[test]
    fn hybrid_mode_assembles_frames() {
        let d = parse(&sample_descriptor()).unwrap();
        let l = TouchLayout::from_descriptor(&d).unwrap();
        let mut asm = FrameAssembler::default();
        // 3 ngón trên màn hình 2-slot/report: report 1 count=3, report 2 count=0
        let a = l
            .parse_report(&report(&[(true, 1, 10, 10), (true, 2, 20, 20)], 3))
            .unwrap();
        let b = l
            .parse_report(&report(&[(true, 3, 30, 30), (false, 0, 0, 0)], 0))
            .unwrap();
        assert!(asm.push(a).is_none());
        let f = asm.push(b).unwrap();
        assert_eq!(f.iter().map(|c| c.id).collect::<Vec<_>>(), vec![1, 2, 3]);
        // Nhấc hết tay
        let z = l.parse_report(&report(&[(false, 1, 10, 10)], 1)).unwrap();
        assert!(asm.push(z).unwrap().is_empty());
    }

    /// Descriptor và report thật của ILITEK-TP (vid 0x222a pid 0x0001): 10 slot, report ID 4.
    const ILITEK_DESC: &str = "05 0d 09 04 a1 01 85 04 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 22 a1 02 05 0d 95 01 75 06 09 51 15 00 25 3f 81 02 09 42 25 01 75 01 95 01 81 02 75 01 95 01 81 03 05 01 75 10 55 0e 65 11 09 30 26 80 25 35 00 46 15 0c 81 42 09 31 26 80 25 46 cb 06 81 42 c0 05 0d 09 56 55 00 65 00 27 ff ff ff 7f 95 01 75 20 81 02 09 54 25 7f 95 01 75 08 81 02 75 08 95 08 81 03 85 06 09 55 25 0a 75 08 95 01 b1 02 06 00 ff 09 c5 85 06 15 00 26 ff 00 75 08 96 00 01 b1 02 c0";
    const ILITEK_REPORT_2F: &str = "04 40 b5 1b ed 12 41 b0 19 65 14 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 70 30 05 00 02 01 ff 00 00 00 00 00 00";

    fn unhex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    #[test]
    fn ilitek_real_descriptor_and_report() {
        let d = parse(&unhex(ILITEK_DESC)).unwrap();
        let l = TouchLayout::from_descriptor(&d).unwrap();
        assert!(l.uses_report_ids);
        assert_eq!(l.max_slots(), 10);
        let ccm = l.contact_count_max.as_ref().unwrap();
        assert_eq!((ccm.report_id, ccm.logical_max), (6, 10));
        let p = l.parse_report(&unhex(ILITEK_REPORT_2F)).unwrap();
        assert_eq!(p.contact_count, Some(2));
        let c: Vec<_> = p.contacts.iter().filter(|c| c.tip).collect();
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].id, c[1].id), (0, 1));
        assert!((c[0].x - 0x1bb5 as f64 / 9600.0).abs() < 1e-9);
        assert!((c[0].y - 0x12ed as f64 / 9600.0).abs() < 1e-9);
        let mut asm = FrameAssembler::default();
        assert_eq!(asm.push(p).unwrap().len(), 2);
    }
}
