use local_rift::cards::Domain;
use local_rift::game::domain_index;
use macroquad::prelude::*;

pub const BG: Color = Color::new(0.027, 0.047, 0.063, 1.0);
pub const PANEL: Color = Color::new(0.049, 0.075, 0.090, 1.0);
pub const LINE: Color = Color::new(0.12, 0.18, 0.20, 1.0);
pub const TEXT: Color = Color::new(0.89, 0.91, 0.87, 1.0);
pub const MUTED: Color = Color::new(0.44, 0.55, 0.56, 1.0);
pub const GOLD: Color = Color::new(0.84, 0.69, 0.40, 1.0);
pub const TEAL: Color = Color::new(0.38, 0.84, 0.73, 1.0);

const RUNE_DOMAINS: [Domain; 6] = [
    Domain::Fury,
    Domain::Calm,
    Domain::Mind,
    Domain::Body,
    Domain::Chaos,
    Domain::Order,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuneSlot {
    pub domain: Domain,
    pub ready: bool,
}

/// Public channelled resources only. Empty sockets include recycled runes;
/// they never reveal the order or identities of cards in the rune deck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuneReserve {
    pub slots: [Option<RuneSlot>; 12],
    pub ready: u8,
    pub stored_energy: u16,
    pub exhausted: u8,
    pub in_deck: u8,
    pub power: [u16; 6],
    pub any_power: u16,
    pub spell_power: u16,
}

impl RuneReserve {
    pub fn new(
        runes: [u8; 6],
        exhausted: [u8; 6],
        power: [u16; 6],
        any_power: u16,
        spell_power: u16,
        stored_energy: u16,
    ) -> Self {
        let mut slots = [None; 12];
        let mut occupied = 0;
        let mut ready = 0;
        let mut spent = 0;
        for domain in RUNE_DOMAINS {
            let i = domain_index(domain);
            let available = runes[i].saturating_sub(exhausted[i]);
            for ordinal in 0..runes[i] {
                if occupied == slots.len() {
                    break;
                }
                let is_ready = ordinal < available;
                slots[occupied] = Some(RuneSlot {
                    domain,
                    ready: is_ready,
                });
                occupied += 1;
                if is_ready {
                    ready += 1;
                } else {
                    spent += 1;
                }
            }
        }
        Self {
            slots,
            ready,
            stored_energy,
            exhausted: spent,
            in_deck: 12 - occupied as u8,
            power,
            any_power,
            spell_power,
        }
    }
}

pub struct Ui {
    pub scale: f32,
    pub offset: Vec2,
    regular: Font,
    bold: Font,
    serif: Font,
    pub enabled: bool,
    pub input_override: Option<(Vec2, bool, bool)>,
    pub pointer_override: Option<(bool, bool)>,
    pub wheel_override: Option<f32>,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            offset: Vec2::ZERO,
            regular: load_ttf_font_from_bytes(include_bytes!(
                "../assets/fonts/NotoSans-Regular.ttf"
            ))
            .unwrap(),
            bold: load_ttf_font_from_bytes(include_bytes!("../assets/fonts/NotoSans-Bold.ttf"))
                .unwrap(),
            serif: load_ttf_font_from_bytes(include_bytes!(
                "../assets/fonts/NotoSerifDisplay-Regular.ttf"
            ))
            .unwrap(),
            enabled: true,
            input_override: None,
            pointer_override: None,
            wheel_override: None,
        }
    }
    pub fn resize(&mut self) {
        self.scale = (screen_width() / 1440.0).min(screen_height() / 900.0);
        self.offset = vec2(
            (screen_width() - 1440.0 * self.scale) / 2.0,
            (screen_height() - 900.0 * self.scale) / 2.0,
        );
    }
    pub fn physical(&self, r: Rect) -> Rect {
        Rect::new(
            self.offset.x + r.x * self.scale,
            self.offset.y + r.y * self.scale,
            r.w * self.scale,
            r.h * self.scale,
        )
    }
    pub fn mouse(&self) -> Vec2 {
        self.input_override
            .map(|input| input.0)
            .unwrap_or_else(|| (Vec2::from(mouse_position()) - self.offset) / self.scale)
    }
    pub fn hover(&self, r: Rect) -> bool {
        self.enabled && r.contains(self.mouse())
    }
    pub fn clicked(&self, r: Rect) -> bool {
        self.hover(r) && self.primary_pressed()
    }
    pub fn primary_pressed(&self) -> bool {
        self.input_override
            .map(|i| i.1)
            .unwrap_or_else(|| is_mouse_button_pressed(MouseButton::Left))
    }
    pub fn secondary_pressed(&self) -> bool {
        self.input_override
            .map(|i| i.2)
            .unwrap_or_else(|| is_mouse_button_pressed(MouseButton::Right))
    }
    pub fn primary_down(&self) -> bool {
        self.pointer_override
            .map(|i| i.0)
            .unwrap_or_else(|| is_mouse_button_down(MouseButton::Left))
    }
    pub fn primary_released(&self) -> bool {
        self.pointer_override
            .map(|i| i.1)
            .unwrap_or_else(|| is_mouse_button_released(MouseButton::Left))
    }
    pub fn wheel(&self) -> f32 {
        self.wheel_override.unwrap_or_else(|| mouse_wheel().1)
    }
    pub fn logical(&self, r: Rect) -> Rect {
        Rect::new(
            (r.x - self.offset.x) / self.scale,
            (r.y - self.offset.y) / self.scale,
            r.w / self.scale,
            r.h / self.scale,
        )
    }
    /// All lines remain available to scrollable views, including paragraph breaks.
    pub fn text_lines(&self, text: &str, width: f32, size: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let next = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty() && self.width(&next, size) > width {
                    lines.push(std::mem::take(&mut line));
                }
                if self.width(word, size) > width {
                    for ch in word.chars() {
                        if self.width(&format!("{line}{ch}"), size) > width && !line.is_empty() {
                            lines.push(std::mem::take(&mut line));
                        }
                        line.push(ch);
                    }
                } else {
                    if !line.is_empty() {
                        line.push(' ');
                    }
                    line.push_str(word);
                }
            }
            lines.push(line);
        }
        lines
    }
    pub fn scroll_text(&self, text: &str, r: Rect, size: f32, scroll: &mut usize) -> usize {
        let lines = self.text_lines(text, r.w - 12.0, size);
        let visible = (r.h / (size * 1.5)).floor().max(1.0) as usize;
        let maximum = lines.len().saturating_sub(visible);
        *scroll = (*scroll).min(maximum);
        if self.wheel() != 0.0 {
            let delta = (self.wheel().abs() * 3.0).ceil() as usize;
            *scroll = if self.wheel() < 0.0 {
                scroll.saturating_add(delta).min(maximum)
            } else {
                scroll.saturating_sub(delta)
            };
        }
        for (i, line) in lines.iter().skip(*scroll).take(visible).enumerate() {
            self.text(line, r.x, r.y + size + i as f32 * size * 1.5, size, TEXT);
        }
        if maximum > 0 {
            self.rect(Rect::new(r.x + r.w - 4.0, r.y, 2.0, r.h), LINE);
            let h = (r.h * visible as f32 / lines.len() as f32).max(18.0);
            self.rect(
                Rect::new(
                    r.x + r.w - 5.0,
                    r.y + (r.h - h) * *scroll as f32 / maximum as f32,
                    4.0,
                    h,
                ),
                TEAL,
            );
        }
        maximum
    }
    pub fn rect(&self, r: Rect, c: Color) {
        let r = self.physical(r);
        draw_rectangle(r.x, r.y, r.w, r.h, c);
    }
    pub fn border(&self, r: Rect, c: Color) {
        let r = self.physical(r);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, self.scale, c);
    }
    pub fn line(&self, a: Vec2, b: Vec2, width: f32, c: Color) {
        let a = self.offset + a * self.scale;
        let b = self.offset + b * self.scale;
        draw_line(a.x, a.y, b.x, b.y, width * self.scale, c);
    }
    pub fn circle(&self, p: Vec2, r: f32, c: Color) {
        let p = self.offset + p * self.scale;
        draw_circle(p.x, p.y, r * self.scale, c);
    }
    pub fn diamond(&self, p: Vec2, r: f32, c: Color) {
        let p = self.offset + p * self.scale;
        draw_poly(p.x, p.y, 4, r * self.scale, 0.0, c);
    }
    fn facet(&self, a: Vec2, b: Vec2, c: Vec2, color: Color) {
        draw_triangle(
            self.offset + a * self.scale,
            self.offset + b * self.scale,
            self.offset + c * self.scale,
            color,
        );
    }
    pub fn rune_gem(&self, center: Vec2, slot: Option<RuneSlot>, size: f32) {
        let Some(slot) = slot else {
            let r = Rect::new(
                center.x - size * 0.52,
                center.y - size * 0.77,
                size * 1.04,
                size * 1.54,
            );
            self.border(r, Color::new(0.16, 0.23, 0.25, 0.55));
            self.circle(center, size * 0.11, Color::new(0.20, 0.28, 0.29, 0.7));
            return;
        };
        let color = rgb(slot.domain.color());
        let bright = if slot.ready { 0.9 } else { 0.40 };
        let body = Color::new(color.r * bright, color.g * bright, color.b * bright, 1.0);
        let light = Color::new(
            (color.r * bright + 0.17).min(1.0),
            (color.g * bright + 0.17).min(1.0),
            (color.b * bright + 0.17).min(1.0),
            1.0,
        );
        let shade = Color::new(
            color.r * bright * 0.42,
            color.g * bright * 0.42,
            color.b * bright * 0.42,
            1.0,
        );
        let rotate = |point: Vec2| {
            if slot.ready {
                point
            } else {
                vec2(-point.y, point.x)
            }
        };
        let points = [
            vec2(0.0, -1.0),
            vec2(0.58, -0.48),
            vec2(0.58, 0.48),
            vec2(0.0, 1.0),
            vec2(-0.58, 0.48),
            vec2(-0.58, -0.48),
        ]
        .map(|point| center + rotate(point * size));
        if slot.ready {
            self.circle(
                center,
                size * 1.18,
                Color::new(color.r, color.g, color.b, 0.055),
            );
            self.circle(
                center,
                size * 0.87,
                Color::new(color.r, color.g, color.b, 0.10),
            );
        }
        for index in 0..6 {
            self.facet(
                center,
                points[index],
                points[(index + 1) % 6],
                if index < 2 {
                    light
                } else if index < 4 {
                    body
                } else {
                    shade
                },
            );
            self.line(
                points[index],
                points[(index + 1) % 6],
                1.0,
                Color::new(
                    color.r,
                    color.g,
                    color.b,
                    if slot.ready { 0.85 } else { 0.65 },
                ),
            );
        }
        // Distinct etched glyphs supplement domain color. Exhausted crystals
        // remain colored because they can still recycle for matching power.
        if size >= 8.0 {
            let ink = Color::new(0.025, 0.055, 0.065, if slot.ready { 0.88 } else { 0.95 });
            let line =
                |a: Vec2, b: Vec2| self.line(center + rotate(a), center + rotate(b), 1.15, ink);
            match slot.domain {
                Domain::Fury => {
                    line(vec2(-2.5, 2.7), vec2(0.0, -3.5));
                    line(vec2(0.0, -3.5), vec2(2.5, 2.7));
                    line(vec2(-1.5, 0.5), vec2(2.5, 2.7));
                }
                Domain::Calm => {
                    line(vec2(-2.7, -1.8), vec2(0.0, 0.0));
                    line(vec2(0.0, 0.0), vec2(2.7, -1.8));
                    line(vec2(-2.7, 1.7), vec2(2.7, 1.7));
                }
                Domain::Mind => {
                    line(vec2(-2.8, 0.0), vec2(0.0, -3.0));
                    line(vec2(0.0, -3.0), vec2(2.8, 0.0));
                    line(vec2(2.8, 0.0), vec2(0.0, 3.0));
                    line(vec2(0.0, 3.0), vec2(-2.8, 0.0));
                }
                Domain::Body => {
                    line(vec2(0.0, 3.2), vec2(0.0, -3.2));
                    line(vec2(-2.5, -0.5), vec2(0.0, -3.2));
                    line(vec2(0.0, -3.2), vec2(2.5, -0.5));
                }
                Domain::Chaos => {
                    line(vec2(-2.5, -3.0), vec2(2.5, 3.0));
                    line(vec2(2.5, -3.0), vec2(-2.5, 3.0));
                }
                Domain::Order => {
                    line(vec2(-2.5, -2.5), vec2(2.5, -2.5));
                    line(vec2(-2.5, 2.5), vec2(2.5, 2.5));
                    line(vec2(0.0, -2.5), vec2(0.0, 2.5));
                }
            }
        }
    }
    pub fn rune_rail(&self, r: Rect, title: &str, reserve: &RuneReserve) {
        self.rect(r, Color::new(0.035, 0.065, 0.077, 0.98));
        self.border(r, LINE);
        self.text(title, r.x + 12.0, r.y + 15.0, 10.0, GOLD);
        self.text(
            "UPRIGHT = READY     SIDEWAYS = EXHAUSTED",
            r.x + 174.0,
            r.y + 11.0,
            8.5,
            TEXT,
        );
        self.bold(
            &u16::from(reserve.ready)
                .saturating_add(reserve.stored_energy)
                .to_string(),
            r.x + 12.0,
            r.y + 43.0,
            23.0,
            TEAL,
        );
        self.text("energy available", r.x + 45.0, r.y + 41.0, 10.0, TEXT);
        self.text(
            &format!("{} ready · {} exhausted", reserve.ready, reserve.exhausted),
            r.x + 12.0,
            r.y + 59.0,
            8.5,
            MUTED,
        );
        for (index, slot) in reserve.slots.iter().enumerate() {
            let center = vec2(r.x + 185.0 + index as f32 * 33.0, r.y + 34.0);
            if let Some(slot) = slot {
                let (w, h) = if slot.ready {
                    (22.0, 32.0)
                } else {
                    (32.0, 22.0)
                };
                let c = rgb(slot.domain.color());
                let card = Rect::new(center.x - w / 2.0, center.y - h / 2.0, w, h);
                self.rect(card, Color::new(c.r * 0.18, c.g * 0.18, c.b * 0.18, 1.0));
                self.border(card, if slot.ready { c } else { MUTED });
                self.rune_gem(center, Some(*slot), 8.0);
            } else {
                self.border(
                    Rect::new(center.x - 10.0, center.y - 15.0, 20.0, 30.0),
                    Color::new(0.11, 0.18, 0.19, 1.0),
                );
            }
            if let Some(slot) = slot {
                self.text(
                    &slot.domain.name()[..2].to_uppercase(),
                    center.x - 6.0,
                    r.y + 59.0,
                    7.5,
                    rgb(slot.domain.color()),
                );
            }
        }
        self.text("DECK", r.x + 580.0, r.y + 15.0, 9.0, MUTED);
        for n in (0..3).rev() {
            self.rect(
                Rect::new(
                    r.x + 584.0 + n as f32 * 2.0,
                    r.y + 24.0 - n as f32 * 2.0,
                    21.0,
                    30.0,
                ),
                PANEL,
            );
            self.border(
                Rect::new(
                    r.x + 584.0 + n as f32 * 2.0,
                    r.y + 24.0 - n as f32 * 2.0,
                    21.0,
                    30.0,
                ),
                LINE,
            );
        }
        self.bold(
            &reserve.in_deck.to_string(),
            r.x + 590.0,
            r.y + 44.0,
            12.0,
            GOLD,
        );
        self.text("GENERATED RESOURCES", r.x + 620.0, r.y + 15.0, 9.0, GOLD);
        self.text(
            &format!("Energy {}", reserve.stored_energy),
            r.x + 620.0,
            r.y + 34.0,
            10.0,
            TEXT,
        );
        self.text(
            &format!("Any {}", reserve.any_power),
            r.x + 704.0,
            r.y + 34.0,
            10.0,
            TEXT,
        );
        self.text(
            &format!("Spell-only {}", reserve.spell_power),
            r.x + 771.0,
            r.y + 34.0,
            10.0,
            TEXT,
        );
        for (index, domain) in RUNE_DOMAINS.into_iter().enumerate() {
            let x = r.x + 625.0 + index as f32 * 40.0;
            let count = reserve.power[domain_index(domain)];
            self.diamond(
                vec2(x, r.y + 52.0),
                3.0,
                if count > 0 { rgb(domain.color()) } else { LINE },
            );
            self.text(
                &format!("{}{}", &domain.name()[..2], count),
                x + 6.0,
                r.y + 55.0,
                9.0,
                if count > 0 { TEXT } else { MUTED },
            );
        }
    }
    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, c: Color) {
        self.font_text(s, x, y, size, c, 0);
    }
    pub fn bold(&self, s: &str, x: f32, y: f32, size: f32, c: Color) {
        self.font_text(s, x, y, size, c, 1);
    }
    pub fn title(&self, s: &str, x: f32, y: f32, size: f32, c: Color) {
        self.font_text(s, x, y, size, c, 2);
    }
    fn font_text(&self, s: &str, x: f32, y: f32, size: f32, c: Color, style: u8) {
        let font = match style {
            1 => &self.bold,
            2 => &self.serif,
            _ => &self.regular,
        };
        draw_text_ex(
            s,
            self.offset.x + x * self.scale,
            self.offset.y + y * self.scale,
            TextParams {
                font: Some(font),
                font_size: (size * self.scale).round().max(1.0) as u16,
                color: c,
                ..Default::default()
            },
        );
    }
    pub fn width(&self, s: &str, size: f32) -> f32 {
        measure_text(
            s,
            Some(&self.regular),
            (size * self.scale).round().max(1.0) as u16,
            1.0,
        )
        .width
            / self.scale
    }
    pub fn wrapped(&self, s: &str, x: f32, y: f32, width: f32, size: f32, c: Color) -> f32 {
        let mut line = String::new();
        let mut cy = y;
        for word in s.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && self.width(&candidate, size) > width {
                self.text(&line, x, cy, size, c);
                cy += size * 1.5;
                line = word.to_owned();
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            self.text(&line, x, cy, size, c);
            cy += size * 1.5;
        }
        cy
    }
    #[allow(clippy::too_many_arguments)]
    pub fn wrapped_lines(
        &self,
        s: &str,
        x: f32,
        y: f32,
        width: f32,
        size: f32,
        c: Color,
        max_lines: usize,
    ) {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in s.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && self.width(&candidate, size) > width {
                lines.push(line);
                line = word.to_owned();
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        for (i, line) in lines.iter().take(max_lines).enumerate() {
            if i + 1 == max_lines && lines.len() > max_lines {
                self.ellipsis(
                    &format!("{line} …"),
                    x,
                    y + i as f32 * size * 1.5,
                    width,
                    size,
                    c,
                );
            } else {
                self.text(line, x, y + i as f32 * size * 1.5, size, c);
            }
        }
    }
    pub fn ellipsis(&self, s: &str, x: f32, y: f32, width: f32, size: f32, c: Color) {
        if self.width(s, size) <= width {
            self.text(s, x, y, size, c);
            return;
        }
        let mut shortened = String::new();
        for ch in s.chars() {
            let test = format!("{shortened}{ch}…");
            if self.width(&test, size) > width {
                break;
            }
            shortened.push(ch);
        }
        shortened.push('…');
        self.text(&shortened, x, y, size, c);
    }
    pub fn button(&self, r: Rect, label: &str, primary: bool, enabled: bool) -> bool {
        let h = self.hover(r) && enabled;
        let bg = if primary {
            if h {
                Color::new(0.94, 0.79, 0.49, 1.0)
            } else {
                GOLD
            }
        } else if h {
            Color::new(0.13, 0.21, 0.23, 1.0)
        } else {
            PANEL
        };
        self.rect(
            r,
            if enabled {
                bg
            } else {
                Color::new(0.08, 0.12, 0.14, 1.0)
            },
        );
        if !primary {
            self.border(r, if h { TEAL } else { LINE });
        }
        let color = if !enabled {
            MUTED
        } else if primary {
            BG
        } else {
            TEXT
        };
        let mut size = 13.0_f32;
        let mut w;
        loop {
            w = measure_text(
                label,
                Some(&self.bold),
                (size * self.scale).round().max(1.0) as u16,
                1.0,
            )
            .width
                / self.scale;
            if w <= r.w - 12.0 || size <= 10.0 {
                break;
            }
            size -= 0.5;
        }
        self.bold(
            label,
            r.x + (r.w - w) / 2.0,
            r.y + r.h / 2.0 + size * 0.38,
            size,
            color,
        );
        enabled && self.clicked(r)
    }
    pub fn pill(&self, x: f32, y: f32, label: &str, color: Color) {
        let w = self.width(label, 10.0) + 20.0;
        self.rect(
            Rect::new(x, y, w, 23.0),
            Color::new(color.r * 0.15, color.g * 0.15, color.b * 0.15, 1.0),
        );
        self.text(label, x + 10.0, y + 15.5, 10.0, color);
    }
    pub fn image(&self, tex: &Texture2D, r: Rect, source: Option<Rect>, tint: Color) {
        let p = self.physical(r);
        draw_texture_ex(
            tex,
            p.x,
            p.y,
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(p.w, p.h)),
                source,
                ..Default::default()
            },
        );
    }
}

pub fn rgb(a: [u8; 3]) -> Color {
    Color::from_rgba(a[0], a[1], a[2], 255)
}

#[cfg(test)]
mod resource_tests {
    use super::*;

    #[test]
    fn ready_and_exhausted_runes_keep_their_domains() {
        let reserve = RuneReserve::new([0, 0, 4, 0, 3, 0], [0, 0, 1, 0, 2, 0], [0; 6], 0, 0, 0);
        assert_eq!(
            (reserve.ready, reserve.exhausted, reserve.in_deck),
            (4, 3, 5)
        );
        assert_eq!(
            reserve
                .slots
                .iter()
                .flatten()
                .filter(|r| r.domain == Domain::Mind && r.ready)
                .count(),
            3
        );
        assert_eq!(
            reserve
                .slots
                .iter()
                .flatten()
                .filter(|r| r.domain == Domain::Chaos && !r.ready)
                .count(),
            2
        );
        assert_eq!(
            reserve.slots.iter().filter(|slot| slot.is_none()).count(),
            5
        );
    }

    #[test]
    fn recycling_returns_a_socket_to_the_rune_deck() {
        let before = RuneReserve::new([0, 0, 6, 0, 0, 0], [0, 0, 2, 0, 0, 0], [0; 6], 0, 0, 0);
        let after = RuneReserve::new([0, 0, 5, 0, 0, 0], [0, 0, 1, 0, 0, 0], [0; 6], 0, 0, 0);
        assert_eq!(before.ready, after.ready);
        assert_eq!(after.in_deck, before.in_deck + 1);
        assert_eq!(after.exhausted, before.exhausted - 1);
    }

    #[test]
    fn generated_power_does_not_create_runes_or_energy() {
        let reserve = RuneReserve::new([0; 6], [0; 6], [1, 0, 2, 0, 3, 0], 2, 4, 0);
        assert_eq!(
            (reserve.ready, reserve.exhausted, reserve.in_deck),
            (0, 0, 12)
        );
        assert_eq!(reserve.power, [1, 0, 2, 0, 3, 0]);
        assert_eq!((reserve.any_power, reserve.spell_power), (2, 4));
    }

    #[test]
    fn a_full_reserve_has_exactly_twelve_visible_runes() {
        let reserve = RuneReserve::new([2; 6], [1; 6], [0; 6], 0, 0, 0);
        assert_eq!(
            (reserve.ready, reserve.exhausted, reserve.in_deck),
            (6, 6, 0)
        );
        assert!(reserve.slots.iter().all(Option::is_some));
    }
    #[test]
    fn generated_resources_above_255_keep_their_full_value() {
        let reserve = RuneReserve::new(
            [2, 0, 0, 0, 0, 0],
            [1, 0, 0, 0, 0, 0],
            [300, 0, 0, 0, 0, 0],
            900,
            700,
            1000,
        );
        assert_eq!(
            (reserve.ready, reserve.exhausted, reserve.in_deck),
            (1, 1, 10)
        );
        assert_eq!(reserve.power[0], 300);
        assert_eq!(
            (
                reserve.any_power,
                reserve.spell_power,
                reserve.stored_energy
            ),
            (900, 700, 1000)
        );
    }
    #[test]
    fn stored_energy_is_separate_from_ready_rune_orientation() {
        let reserve = RuneReserve::new([1, 0, 0, 0, 0, 0], [1, 0, 0, 0, 0, 0], [0; 6], 0, 0, 2);
        assert_eq!(reserve.ready, 0);
        assert_eq!(reserve.exhausted, 1);
        assert_eq!(reserve.stored_energy, 2);
    }
}
