//! Shared flat board geometry. Independent gear never occupies a unit hit area.
use macroquad::prelude::Rect;

pub const FIELDS: [Rect; 2] = [
    Rect::new(245.0, 258.0, 435.0, 232.0),
    Rect::new(693.0, 258.0, 435.0, 232.0),
];
pub const BASES: [Rect; 2] = [
    Rect::new(245.0, 494.0, 883.0, 76.0),
    Rect::new(245.0, 178.0, 883.0, 76.0),
];

pub fn unit_capacity(has_independent_gear: bool) -> usize {
    if has_independent_gear { 3 } else { 5 }
}

pub fn unit_rects(lane: usize, owner: usize, total: usize, has_gear: bool) -> Vec<Rect> {
    let field = FIELDS[lane];
    let count = total.min(unit_capacity(has_gear));
    let region_width = if has_gear { 236.0 } else { 384.0 };
    let width = (region_width / count.max(1) as f32 - 6.0).min(132.0);
    (0..count)
        .map(|index| {
            Rect::new(
                field.x
                    + 45.0
                    + (region_width - count as f32 * (width + 6.0)) * 0.5
                    + index as f32 * (width + 6.0),
                field.y + if owner == 1 { 66.0 } else { 150.0 },
                width,
                76.0,
            )
        })
        .collect()
}

pub fn gear_rect(lane: usize, owner: usize) -> Rect {
    let field = FIELDS[lane];
    Rect::new(
        field.x + 291.0,
        field.y + if owner == 1 { 66.0 } else { 150.0 },
        132.0,
        76.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
    }

    #[test]
    fn independent_gear_is_disjoint_from_units_and_protected_mat_areas_at_all_sizes() {
        let protected = [
            BASES[0],
            BASES[1],
            Rect::new(245.0, 110.0, 883.0, 66.0),
            Rect::new(245.0, 571.0, 883.0, 66.0),
            Rect::new(218.0, 110.0, 23.0, 527.0),
            Rect::new(1132.0, 110.0, 23.0, 527.0),
        ];
        for (width, height) in [
            (1440.0_f32, 900.0_f32),
            (1152.0, 720.0),
            (1280.0, 800.0),
            (1600.0, 1000.0),
        ] {
            let scale = (width / 1440.0).min(height / 900.0);
            for (lane, field) in FIELDS.iter().enumerate() {
                for owner in 0..2 {
                    let gear = gear_rect(lane, owner);
                    assert!(gear.x >= field.x && gear.x + gear.w <= field.x + field.w);
                    assert!(gear.y >= field.y + 62.0 && gear.y + gear.h <= field.y + field.h);
                    assert!(!overlaps(gear, gear_rect(lane, 1 - owner)));
                    assert!(protected.iter().all(|r| !overlaps(gear, *r)));
                    assert!(gear.w * scale >= 105.0 && gear.h * scale >= 60.0);
                    for count in 0..=30 {
                        let units = unit_rects(lane, owner, count, true);
                        assert_eq!(units.len(), count.min(3));
                        for (index, unit) in units.iter().enumerate() {
                            assert!(!overlaps(*unit, gear));
                            assert!(unit.w * scale >= 58.0);
                            assert!(
                                units[index + 1..]
                                    .iter()
                                    .all(|other| !overlaps(*unit, *other))
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn fields_without_gear_preserve_the_existing_five_unit_layout() {
        for (lane, field) in FIELDS.iter().enumerate() {
            for owner in 0..2 {
                for total in 0..=12 {
                    let count = total.min(5);
                    let width = (384.0 / count.max(1) as f32 - 6.0).min(132.0);
                    for (index, actual) in unit_rects(lane, owner, total, false).iter().enumerate()
                    {
                        assert_eq!(
                            *actual,
                            Rect::new(
                                field.x
                                    + 45.0
                                    + (384.0 - count as f32 * (width + 6.0)) * 0.5
                                    + index as f32 * (width + 6.0),
                                field.y + if owner == 1 { 66.0 } else { 150.0 },
                                width,
                                76.0,
                            )
                        );
                    }
                    assert_eq!(unit_rects(lane, owner, total, false).len(), count);
                }
            }
        }
    }
}
