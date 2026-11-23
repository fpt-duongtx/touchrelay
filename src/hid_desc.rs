//! Parser HID Report Descriptor tối giản (đủ cho digitizer multitouch).
//!
//! Tự parse descriptor thay vì dùng IOHIDDeviceGetValue vì cách đó có race:
//! khi màn hình gửi nhiều report liên tiếp (hybrid mode), giá trị cache trong
//! kernel có thể đã bị report sau ghi đè trước khi callback của ta chạy.
//! Parse raw report bytes thì mỗi report được xử lý đúng nội dung của nó.

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub report_id: u8,
    /// Vị trí bit tính từ đầu payload (KHÔNG tính byte report ID).
    pub bit_offset: u32,
    pub bit_size: u32,
    pub usage_page: u16,
    pub usage: u16,
    pub logical_min: i32,
    pub logical_max: i32,
    /// Index của collection trong cùng chứa field.
    pub collection: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Collection {
    pub usage_page: u16,
    pub usage: u16,
    /// 0 = Physical, 1 = Application, 2 = Logical, ...
    #[allow(dead_code)]
    pub kind: u8,
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Descriptor {
    pub inputs: Vec<Field>,
    pub features: Vec<Field>,
    pub collections: Vec<Collection>,
    pub uses_report_ids: bool,
    /// Tổng số bit payload của mỗi feature report (theo report ID).
    pub feature_report_bits: HashMap<u8, u32>,
}

#[derive(Clone, Default)]
struct Globals {
    usage_page: u16,
    logical_min: i32,
    logical_max_s: i32,
    logical_max_u: u32,
    report_size: u32,
    report_count: u32,
    report_id: u8,
}

impl Descriptor {
    /// Chuỗi collection từ `idx` lên tới gốc.
    pub fn ancestors(&self, idx: Option<usize>) -> Vec<usize> {
        let mut out = Vec::new();
        let mut cur = idx;
        while let Some(i) = cur {
            out.push(i);
            cur = self.collections[i].parent;
        }
        out
    }

    pub fn in_collection(&self, f: &Field, page: u16, usage: u16) -> Option<usize> {
        self.ancestors(f.collection).into_iter().find(|&i| {
            let c = &self.collections[i];
            c.usage_page == page && c.usage == usage
        })
    }
}

pub fn parse(desc: &[u8]) -> Result<Descriptor, String> {
    let mut d = Descriptor::default();
    let mut g = Globals::default();
    let mut gstack: Vec<Globals> = Vec::new();
    // Local items: usage có thể kèm usage page (usage 32-bit).
    let mut usages: Vec<(Option<u16>, u16)> = Vec::new();
    let mut usage_min: Option<u32> = None;
    let mut usage_max: Option<u32> = None;
    let mut cstack: Vec<usize> = Vec::new();
    let mut input_bits: HashMap<u8, u32> = HashMap::new();
    let mut feature_bits: HashMap<u8, u32> = HashMap::new();

    let mut i = 0usize;
    while i < desc.len() {
        let prefix = desc[i];
        if prefix == 0xFE {
            // Long item: bỏ qua.
            let len = *desc.get(i + 1).ok_or("long item bị cắt")? as usize;
            i += 3 + len;
            continue;
        }
        let size = match prefix & 0x03 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        };
        if i + 1 + size > desc.len() {
            return Err(format!("item tại byte {i} bị cắt"));
        }
        let data = &desc[i + 1..i + 1 + size];
        let mut udata: u32 = 0;
        for (k, b) in data.iter().enumerate() {
            udata |= (*b as u32) << (8 * k);
        }
        let sdata: i32 = match size {
            1 => data[0] as i8 as i32,
            2 => (udata as u16) as i16 as i32,
            4 => udata as i32,
            _ => 0,
        };
        let typ = (prefix >> 2) & 0x03;
        let tag = prefix >> 4;
        i += 1 + size;

        match typ {
            // ---------------- Main ----------------
            0 => {
                match tag {
                    0x8 | 0xB => {
                        // Input / Feature
                        let is_input = tag == 0x8;
                        let constant = udata & 0x01 != 0;
                        let variable = udata & 0x02 != 0;
                        let rid = g.report_id;
                        let bits = if is_input { &mut input_bits } else { &mut feature_bits };
                        let off = bits.entry(rid).or_insert(0);

                        // Gộp usage đơn lẻ + usage range.
                        let mut list: Vec<(u16, u16)> =
                            usages.iter().map(|(p, u)| (p.unwrap_or(g.usage_page), *u)).collect();
                        if let (Some(mn), Some(mx)) = (usage_min, usage_max) {
                            let page = if mn > 0xFFFF { (mn >> 16) as u16 } else { g.usage_page };
                            let (mn, mx) = (mn & 0xFFFF, mx & 0xFFFF);
                            if mx >= mn && mx - mn < 4096 {
                                for u in mn..=mx {
                                    list.push((page, u as u16));
                                }
                            }
                        }

                        let lmax = if g.logical_min >= 0 && g.logical_max_s < g.logical_min {
                            // Logical Max mã hoá unsigned (vd 0xFF trong 1 byte).
                            g.logical_max_u as i32
                        } else {
                            g.logical_max_s
                        };

                        for k in 0..g.report_count {
                            if !constant && variable && !list.is_empty() {
                                let (page, usage) = list[(k as usize).min(list.len() - 1)];
                                let f = Field {
                                    report_id: rid,
                                    bit_offset: *off,
                                    bit_size: g.report_size,
                                    usage_page: page,
                                    usage,
                                    logical_min: g.logical_min,
                                    logical_max: lmax,
                                    collection: cstack.last().copied(),
                                };
                                if is_input {
                                    d.inputs.push(f);
                                } else {
                                    d.features.push(f);
                                }
                            }
                            *off += g.report_size;
                        }
                    }
                    0x9 => { /* Output: không cần */ }
                    0xA => {
                        // Collection
                        let (page, usage) = usages
                            .first()
                            .map(|(p, u)| (p.unwrap_or(g.usage_page), *u))
                            .unwrap_or((g.usage_page, 0));
                        d.collections.push(Collection {
                            usage_page: page,
                            usage,
                            kind: udata as u8,
                            parent: cstack.last().copied(),
                        });
                        cstack.push(d.collections.len() - 1);
                    }
                    0xC => {
                        cstack.pop();
                    }
                    _ => {}
                }
                usages.clear();
                usage_min = None;
                usage_max = None;
            }
            // ---------------- Global ----------------
            1 => match tag {
                0x0 => g.usage_page = udata as u16,
                0x1 => g.logical_min = sdata,
                0x2 => {
                    g.logical_max_s = sdata;
                    g.logical_max_u = udata;
                }
                0x7 => g.report_size = udata,
                0x8 => {
                    g.report_id = udata as u8;
                    d.uses_report_ids = true;
                }
                0x9 => g.report_count = udata,
                0xA => gstack.push(g.clone()),
                0xB => {
                    if let Some(p) = gstack.pop() {
                        g = p;
                    }
                }
                _ => {}
            },
            // ---------------- Local ----------------
            2 => match tag {
                0x0 => {
                    if size == 4 {
                        usages.push((Some((udata >> 16) as u16), (udata & 0xFFFF) as u16));
                    } else {
                        usages.push((None, udata as u16));
                    }
                }
                0x1 => usage_min = Some(udata),
                0x2 => usage_max = Some(udata),
                _ => {}
            },
            _ => {}
        }
    }
    d.feature_report_bits = feature_bits;
    Ok(d)
}

/// Đọc giá trị field từ payload (đã bỏ byte report ID).
pub fn extract(payload: &[u8], f: &Field) -> Option<i32> {
    if f.bit_size == 0 || f.bit_size > 32 {
        return None;
    }
    let mut v: u32 = 0;
    for b in 0..f.bit_size {
        let bit = f.bit_offset + b;
        let byte = *payload.get((bit / 8) as usize)?;
        if (byte >> (bit % 8)) & 1 == 1 {
            v |= 1 << b;
        }
    }
    if f.logical_min < 0 && f.bit_size < 32 && (v >> (f.bit_size - 1)) & 1 == 1 {
        v |= !0u32 << f.bit_size; // sign-extend
    }
    Some(v as i32)
}

/// Ghi giá trị vào payload (dùng cho feature report "Device Mode").
pub fn insert(payload: &mut [u8], f: &Field, value: i32) -> bool {
    let v = value as u32;
    for b in 0..f.bit_size.min(32) {
        let bit = f.bit_offset + b;
        let Some(byte) = payload.get_mut((bit / 8) as usize) else {
            return false;
        };
        let mask = 1u8 << (bit % 8);
        if (v >> b) & 1 == 1 {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_and_insert_roundtrip() {
        let f = Field {
            report_id: 0,
            bit_offset: 3,
            bit_size: 12,
            usage_page: 1,
            usage: 0x30,
            logical_min: 0,
            logical_max: 4095,
            collection: None,
        };
        let mut buf = [0u8; 4];
        assert!(insert(&mut buf, &f, 3000));
        assert_eq!(extract(&buf, &f), Some(3000));
    }

    #[test]
    fn signed_extract() {
        let f = Field {
            report_id: 0,
            bit_offset: 0,
            bit_size: 8,
            usage_page: 1,
            usage: 0x30,
            logical_min: -127,
            logical_max: 127,
            collection: None,
        };
        assert_eq!(extract(&[0xFE], &f), Some(-2));
    }
}
