//! Procedural tabletop: every mesh is made locally, without licensed artwork.
use macroquad::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SceneUnit {
    pub id: u32,
    pub owner: usize,
    /// 0 is the owner's base; 1 and 2 are the two battlefields.
    pub location: usize,
    pub ready: bool,
    pub might: local_rift::numeric::Might,
    pub accent: [u8; 3],
}

#[derive(Clone, Debug, Default)]
pub struct SceneState {
    pub units: Vec<SceneUnit>,
    pub selected: Option<u32>,
    pub owners: [Option<usize>; 2],
    pub time: f32,
    pub orbit: f32,
}

pub struct SceneHits {
    pub battlefields: [Rect; 2],
    pub bases: [Rect; 2],
    pub units: Vec<(u32, Rect)>,
}

thread_local! {
    static ENVIRONMENT: RefCell<Option<Vec<Mesh>>> = const { RefCell::new(None) };
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}
fn tint(c: Color, value: f32) -> Color {
    Color::new(
        (c.r * value).min(1.),
        (c.g * value).min(1.),
        (c.b * value).min(1.),
        c.a,
    )
}
fn shade(c: Color, normal: Vec3) -> Color {
    let light = vec3(-0.5, 1., 0.65).normalize();
    tint(
        c,
        0.58 + normal.normalize_or_zero().dot(light).max(0.) * 0.49,
    )
}

/// Small batches keep the meshes below Macroquad's default vertex capacity.
struct Sculptor {
    meshes: Vec<Mesh>,
}
impl Sculptor {
    fn new() -> Self {
        Self { meshes: vec![] }
    }
    fn mesh(&mut self) -> &mut Mesh {
        if self.meshes.last().is_none_or(|m| m.vertices.len() > 2800) {
            self.meshes.push(Mesh {
                vertices: vec![],
                indices: vec![],
                texture: None,
            });
        }
        self.meshes.last_mut().unwrap()
    }
    fn triangle(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Color) {
        let mesh = self.mesh();
        let n = mesh.vertices.len() as u16;
        for p in [a, b, c] {
            mesh.vertices.push(Vertex::new2(p, Vec2::ZERO, color));
        }
        mesh.indices.extend([n, n + 1, n + 2]);
    }
    fn quad(&mut self, points: [Vec3; 4], color: Color) {
        let mesh = self.mesh();
        let n = mesh.vertices.len() as u16;
        for p in points {
            mesh.vertices.push(Vertex::new2(p, Vec2::ZERO, color));
        }
        mesh.indices.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
    }
    fn block(&mut self, center: Vec3, size: Vec3, rotation: f32, color: Color) {
        let rotation = Quat::from_rotation_y(rotation);
        let point = |x: f32, y: f32, z: f32| center + rotation * (vec3(x, y, z) * size * 0.5);
        for (normal, corners) in [
            (
                Vec3::Y,
                [(-1., 1., -1.), (-1., 1., 1.), (1., 1., 1.), (1., 1., -1.)],
            ),
            (
                -Vec3::Y,
                [
                    (-1., -1., 1.),
                    (-1., -1., -1.),
                    (1., -1., -1.),
                    (1., -1., 1.),
                ],
            ),
            (
                Vec3::Z,
                [(-1., -1., 1.), (1., -1., 1.), (1., 1., 1.), (-1., 1., 1.)],
            ),
            (
                -Vec3::Z,
                [
                    (1., -1., -1.),
                    (-1., -1., -1.),
                    (-1., 1., -1.),
                    (1., 1., -1.),
                ],
            ),
            (
                Vec3::X,
                [(1., -1., 1.), (1., -1., -1.), (1., 1., -1.), (1., 1., 1.)],
            ),
            (
                -Vec3::X,
                [
                    (-1., -1., -1.),
                    (-1., -1., 1.),
                    (-1., 1., 1.),
                    (-1., 1., -1.),
                ],
            ),
        ] {
            self.quad(
                corners.map(|(x, y, z)| point(x, y, z)),
                shade(color, rotation * normal),
            );
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn frustum(
        &mut self,
        bottom: Vec3,
        low: f32,
        high: f32,
        height: f32,
        sides: usize,
        phase: f32,
        color: Color,
    ) {
        for i in 0..sides {
            let a = i as f32 / sides as f32 * TAU + phase;
            let b = (i + 1) as f32 / sides as f32 * TAU + phase;
            let direction = |angle: f32| vec3(angle.cos(), 0., angle.sin());
            let p = bottom + direction(a) * low;
            let q = bottom + direction(b) * low;
            let r = bottom + direction(b) * high + Vec3::Y * height;
            let s = bottom + direction(a) * high + Vec3::Y * height;
            let normal = vec3(
                ((a + b) * 0.5).cos(),
                (low - high) / height.max(0.001),
                ((a + b) * 0.5).sin(),
            );
            self.quad([p, q, r, s], shade(color, normal));
            if high > 0.001 {
                self.triangle(bottom + Vec3::Y * height, s, r, tint(color, 1.06));
            }
        }
    }
    fn ring(&mut self, center: Vec3, inner: f32, outer: f32, sides: usize, color: Color) {
        for i in 0..sides {
            let a = i as f32 / sides as f32 * TAU;
            let b = (i + 1) as f32 / sides as f32 * TAU;
            let point =
                |angle: f32, radius: f32| center + vec3(angle.cos(), 0., angle.sin()) * radius;
            self.quad(
                [
                    point(a, inner),
                    point(b, inner),
                    point(b, outer),
                    point(a, outer),
                ],
                color,
            );
        }
    }
    fn crystal(&mut self, bottom: Vec3, radius: f32, height: f32, color: Color) {
        self.frustum(bottom, radius * 0.75, radius, height * 0.65, 5, 0.4, color);
        self.frustum(
            bottom + Vec3::Y * height * 0.65,
            radius,
            0.,
            height * 0.35,
            5,
            0.4,
            tint(color, 1.2),
        );
    }
    fn tree(&mut self, at: Vec3, height: f32, phase: f32) {
        self.frustum(at, 0.09, 0.05, height * 0.66, 5, phase, rgb(73, 74, 58));
        for (i, (y, r, h)) in [(0.15, 0.37, 0.52), (0.36, 0.3, 0.48), (0.57, 0.22, 0.43)]
            .iter()
            .enumerate()
        {
            self.frustum(
                at + Vec3::Y * height * *y,
                height * r,
                0.,
                height * h,
                6,
                phase,
                [rgb(25, 79, 68), rgb(37, 108, 86), rgb(55, 139, 111)][i],
            );
        }
    }
    fn outline(&mut self, points: &[Vec3], width: f32, color: Color) {
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            let d = (b - a).normalize_or_zero();
            let n = vec3(-d.z, 0., d.x) * width * 0.5;
            self.quad([a + n, b + n, b - n, a - n], color);
        }
    }
}

fn octagon(center: Vec3, width: f32, depth: f32) -> [Vec3; 8] {
    [
        (-0.72, -1.),
        (0.72, -1.),
        (1., -0.64),
        (1., 0.64),
        (0.72, 1.),
        (-0.72, 1.),
        (-1., 0.64),
        (-1., -0.64),
    ]
    .map(|(x, z)| center + vec3(x * width, 0., z * depth))
}
fn slab(
    s: &mut Sculptor,
    center: Vec3,
    width: f32,
    depth: f32,
    thickness: f32,
    top: Color,
    edge: Color,
) {
    let upper = octagon(center, width, depth);
    let lower = octagon(center - vec3(0., thickness, 0.), width * 0.97, depth * 0.97);
    for i in 0..8 {
        let next = (i + 1) % 8;
        s.triangle(
            center,
            upper[i],
            upper[next],
            tint(top, 1. - (i % 3) as f32 * 0.025),
        );
        let normal = (upper[i] + upper[next]) * 0.5 - center;
        s.quad(
            [upper[i], lower[i], lower[next], upper[next]],
            shade(edge, normal),
        );
    }
}

fn build_environment() -> Vec<Mesh> {
    let mut s = Sculptor::new();
    let stone = rgb(38, 51, 54);
    let gold = rgb(124, 105, 64);
    // The board's three layers have visible bevels and a fine brass inlay.
    slab(
        &mut s,
        vec3(0., -0.83, 0.),
        6.05,
        4.12,
        0.28,
        rgb(18, 30, 38),
        rgb(12, 20, 28),
    );
    slab(
        &mut s,
        vec3(0., -0.68, 0.),
        5.92,
        3.99,
        0.15,
        rgb(37, 48, 52),
        rgb(59, 60, 49),
    );
    slab(
        &mut s,
        vec3(0., -0.61, 0.),
        5.76,
        3.83,
        0.11,
        rgb(20, 35, 39),
        rgb(27, 40, 43),
    );
    s.outline(&octagon(vec3(0., -0.603, 0.), 5.5, 3.58), 0.021, gold);
    s.outline(
        &octagon(vec3(0., -0.600, 0.), 5.39, 3.48),
        0.009,
        rgb(65, 69, 58),
    );
    // Inlaid central compass and fine connecting paths.
    s.ring(vec3(0., -0.595, 0.), 0.34, 0.36, 48, rgb(83, 83, 62));
    s.ring(vec3(0., -0.594, 0.), 0.15, 0.17, 32, rgb(94, 90, 64));
    for i in 0..8 {
        let a = i as f32 * TAU / 8.;
        let p = vec3(a.cos(), 0., a.sin());
        let tangent = vec3(-a.sin(), 0., a.cos());
        let c = vec3(0., -0.59, 0.);
        s.triangle(
            c + p * 0.58,
            c + p * 0.24 + tangent * 0.055,
            c + p * 0.24 - tangent * 0.055,
            gold,
        );
    }
    for side in [-1., 1.] {
        s.block(
            vec3(side * 3.1, -0.58, 0.),
            vec3(4.1, 0.025, 0.025),
            0.,
            rgb(64, 72, 62),
        );
        for z in [-2.8, 2.8] {
            s.block(
                vec3(side * 3.8, -0.565, z),
                vec3(1.2, 0.023, 0.017),
                0.,
                rgb(60, 73, 65),
            );
        }
    }
    // Each battlefield is a separate floating, faceted stone island.
    for index in 0..2 {
        let x = if index == 0 { -2.67 } else { 2.67 };
        let center = vec3(x, 0.44, 0.);
        let top = if index == 0 {
            rgb(49, 78, 65)
        } else {
            rgb(109, 94, 67)
        };
        slab(&mut s, center, 2.27, 1.66, 0.2, top, stone);
        s.outline(
            &octagon(center + Vec3::Y * 0.006, 2.16, 1.56),
            0.025,
            if index == 0 {
                rgb(90, 132, 101)
            } else {
                rgb(177, 151, 95)
            },
        );
        let top_edge = octagon(center - Vec3::Y * 0.2, 2.22, 1.62);
        let lower = octagon(vec3(x, -0.49, 0.04), 1.72, 1.22);
        for i in 0..8 {
            let next = (i + 1) % 8;
            let mid =
                (top_edge[i] + top_edge[next] + lower[i] + lower[next]) * 0.25 + vec3(0., 0.03, 0.);
            let base = if index == 0 {
                rgb(34, 55, 54)
            } else {
                rgb(66, 65, 57)
            };
            s.triangle(
                top_edge[i],
                top_edge[next],
                mid,
                tint(base, 0.8 + (i % 3) as f32 * 0.12),
            );
            s.triangle(
                top_edge[next],
                lower[next],
                mid,
                tint(base, 0.65 + (i % 2) as f32 * 0.15),
            );
            s.triangle(lower[next], lower[i], mid, tint(base, 0.8));
            s.triangle(lower[i], top_edge[i], mid, tint(base, 0.95));
        }
        // Tapered stones suspended beneath the island edge.
        for j in 0..5 {
            let px = x - 1.75 + j as f32 * 0.8;
            s.frustum(
                vec3(px, -0.62, 0.95 + (j % 2) as f32 * 0.12),
                0.03,
                0.18,
                0.47,
                5,
                j as f32,
                stone,
            );
        }
        // Circular arena of individually inset stones.
        for j in 0..18 {
            let a = j as f32 * TAU / 18.;
            let color = if index == 0 {
                rgb(79, 102, 85)
            } else {
                rgb(142, 123, 84)
            };
            s.block(
                vec3(x + a.cos() * 1.12, 0.47, 0.18 + a.sin() * 0.93),
                vec3(0.31, 0.045, 0.15),
                -a + PI * 0.5,
                color,
            );
        }
        s.ring(
            vec3(x, 0.471, 0.18),
            0.63,
            0.65,
            48,
            if index == 0 {
                rgb(83, 116, 93)
            } else {
                rgb(172, 147, 98)
            },
        );
        s.ring(
            vec3(x, 0.472, 0.18),
            0.13,
            0.17,
            12,
            if index == 0 {
                rgb(105, 139, 108)
            } else {
                rgb(182, 151, 92)
            },
        );
    }
    // Emerald grove: sculpted pines, irregular flagstones, luminous crystal clusters.
    for (x, z, h, a) in [
        (-4.27, -0.72, 1.31, 0.1),
        (-3.55, -1.1, 1.56, 0.5),
        (-2.72, -1.25, 1.13, 0.2),
        (-4.4, 0.23, 0.82, 0.8),
    ] {
        s.tree(vec3(x, 0.47, z), h, a);
    }
    for (x, z, scale) in [
        (-4.08, 1.02, 0.34),
        (-3.73, 1.2, 0.23),
        (-1.06, -0.8, 0.33),
        (-1.3, -1.1, 0.2),
    ] {
        s.crystal(
            vec3(x, 0.47, z),
            scale * 0.42,
            scale * 1.9,
            rgb(76, 205, 181),
        );
        s.crystal(
            vec3(x + 0.2, 0.47, z + 0.12),
            scale * 0.24,
            scale,
            rgb(46, 141, 139),
        );
    }
    for (i, (x, z)) in [
        (-1.64, 0.7),
        (-1.37, 1.08),
        (-2.95, 0.55),
        (-3.12, 1.35),
        (-2.2, -0.68),
        (-4., 0.77),
    ]
    .iter()
    .enumerate()
    {
        s.block(
            vec3(*x, 0.48, *z),
            vec3(0.34, 0.06, 0.25),
            i as f32 * 0.4,
            rgb(97, 115, 101),
        );
    }
    // Temple island: a miniature shrine, columns and broken golden sandstone.
    s.block(
        vec3(2.95, 0.51, -1.04),
        vec3(2.04, 0.13, 0.65),
        0.,
        rgb(155, 139, 102),
    );
    s.block(
        vec3(2.95, 0.6, -1.04),
        vec3(1.76, 0.1, 0.51),
        0.,
        rgb(185, 164, 117),
    );
    for x in [2.23, 3.68] {
        s.block(
            vec3(x, 0.71, -1.06),
            vec3(0.4, 0.16, 0.4),
            0.,
            rgb(187, 164, 112),
        );
        s.frustum(
            vec3(x, 0.79, -1.06),
            0.14,
            0.11,
            1.12,
            8,
            0.,
            rgb(185, 165, 120),
        );
        for y in [0.83, 1.72] {
            s.frustum(
                vec3(x, y, -1.06),
                0.17,
                0.17,
                0.07,
                8,
                0.,
                rgb(216, 185, 119),
            );
        }
        s.block(
            vec3(x, 1.97, -1.06),
            vec3(0.39, 0.16, 0.39),
            0.,
            rgb(195, 170, 115),
        );
    }
    s.block(
        vec3(2.95, 2.1, -1.06),
        vec3(1.93, 0.18, 0.42),
        0.,
        rgb(197, 169, 109),
    );
    s.block(
        vec3(2.95, 2.225, -1.06),
        vec3(2.12, 0.07, 0.48),
        0.,
        rgb(222, 191, 125),
    );
    s.block(
        vec3(2.95, 2.33, -1.06),
        vec3(1.35, 0.13, 0.35),
        0.,
        rgb(160, 141, 99),
    );
    s.crystal(vec3(2.95, 1.13, -1.06), 0.23, 0.64, rgb(236, 183, 85));
    // The distant corners retain recognizable ruin silhouettes.
    s.frustum(
        vec3(4.34, 0.46, 0.43),
        0.23,
        0.19,
        0.68,
        7,
        0.2,
        rgb(161, 143, 104),
    );
    s.block(
        vec3(4.39, 1.16, 0.45),
        vec3(0.47, 0.14, 0.42),
        0.25,
        rgb(189, 166, 112),
    );
    s.block(
        vec3(1.18, 0.57, -0.64),
        vec3(0.38, 0.21, 0.72),
        -0.5,
        rgb(156, 137, 97),
    );
    for (i, (x, z)) in [
        (1.06, 0.54),
        (1.34, 1.13),
        (3.85, 1.07),
        (4.08, -0.65),
        (2.32, 1.27),
        (2.05, 0.57),
    ]
    .iter()
    .enumerate()
    {
        s.block(
            vec3(*x, 0.48, *z),
            vec3(0.4, 0.07, 0.27),
            i as f32 * 0.4,
            rgb(170, 147, 100),
        );
    }
    // Opposite bases are smaller islands connected by fine engraved routes.
    for (owner, z) in [(0, 2.78), (1, -2.78)] {
        let trim = if owner == 0 {
            rgb(77, 155, 153)
        } else {
            rgb(166, 101, 96)
        };
        slab(
            &mut s,
            vec3(0., -0.18, z),
            2.26,
            0.64,
            0.2,
            rgb(32, 49, 53),
            rgb(21, 31, 38),
        );
        s.outline(&octagon(vec3(0., -0.173, z), 2.12, 0.53), 0.022, trim);
        for x in [-1.68, -0.84, 0., 0.84, 1.68] {
            s.frustum(
                vec3(x, -0.169, z),
                0.25,
                0.25,
                0.025,
                6,
                PI / 6.,
                rgb(43, 61, 64),
            );
            s.ring(vec3(x, -0.14, z), 0.22, 0.23, 6, tint(trim, 0.68));
        }
    }
    s.meshes
}

/// Draw the tabletop in a physical-pixel viewport and return physical-pixel hit areas.
/// The caller's 2D camera is intentionally reset to Macroquad's default afterwards.
pub fn draw_scene(rect: Rect, state: &SceneState) -> SceneHits {
    set_default_camera();
    let scale = rect.w / 900.;
    // A restrained mist gradient makes the dark stone silhouettes readable.
    for band in 0..48 {
        let t = band as f32 / 48.;
        let glow = (1. - (t - 0.42).abs() * 1.8).max(0.);
        draw_rectangle(
            rect.x,
            rect.y + t * rect.h,
            rect.w,
            rect.h / 48. + 1.,
            Color::new(
                0.025 + glow * 0.018,
                0.046 + glow * 0.036,
                0.065 + glow * 0.042,
                1.,
            ),
        );
    }
    for i in 0..64 {
        let x = ((i * 137 + 23) % 997) as f32 / 997.;
        let y = ((i * 311 + 67) % 911) as f32 / 911.;
        let shimmer = 0.35 + (state.time * 0.45 + i as f32 * 1.7).sin() * 0.15;
        draw_circle(
            rect.x + x * rect.w,
            rect.y + y * rect.h,
            (if i % 7 == 0 { 1.2 } else { 0.65 }) * scale,
            Color::new(0.42, 0.64, 0.67, shimmer),
        );
    }
    let angle = state.orbit.clamp(-0.45, 0.45);
    let camera = Camera3D {
        position: vec3(angle.sin() * 13.6, 10.7, angle.cos() * 13.6),
        target: vec3(0., 0.12, 0.),
        up: Vec3::Y,
        fovy: 30_f32.to_radians(),
        aspect: Some(rect.w / rect.h.max(1.)),
        viewport: Some((
            rect.x.round() as i32,
            (screen_height() - rect.y - rect.h).round() as i32,
            rect.w.round() as i32,
            rect.h.round() as i32,
        )),
        z_near: 0.1,
        z_far: 80.,
        ..Default::default()
    };
    set_camera(&camera);
    ENVIRONMENT.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.is_none() {
            *cache = Some(build_environment());
        }
        for mesh in cache.as_ref().unwrap() {
            draw_mesh(mesh);
        }
    });
    let mut animated = Sculptor::new();
    for (index, owner) in state.owners.iter().enumerate() {
        let center = vec3(if index == 0 { -2.67 } else { 2.67 }, 0.485, 0.18);
        let color = match owner {
            Some(0) => rgb(102, 211, 197),
            Some(_) => rgb(231, 139, 125),
            None => rgb(164, 154, 109),
        };
        animated.ring(center, 1.005, 1.02, 64, tint(color, 0.8));
        // Four compass ticks are deliberately brighter than the perimeter.
        for j in 0..4 {
            let a = j as f32 * PI * 0.5;
            let p = center + vec3(a.cos(), 0., a.sin()) * 1.015;
            animated.block(p + Vec3::Y * 0.007, vec3(0.17, 0.014, 0.04), -a, color);
        }
    }
    // The mat's usable card zones stay flat and stable; scenery remains a backdrop.
    let factor = rect.w / 910.0;
    let screen = |r: Rect| {
        Rect::new(
            rect.x + (r.x - 232.0) * factor,
            rect.y + (r.y - 105.0) * factor,
            r.w * factor,
            r.h * factor,
        )
    };
    let fields = [
        Rect::new(245.0, 258.0, 435.0, 232.0),
        Rect::new(693.0, 258.0, 435.0, 232.0),
    ];
    let bases = [
        Rect::new(245.0, 494.0, 883.0, 76.0),
        Rect::new(245.0, 178.0, 883.0, 76.0),
    ];
    let mut hits = SceneHits {
        battlefields: fields.map(screen),
        bases: bases.map(screen),
        units: Vec::new(),
    };
    for (owner, base) in bases.iter().enumerate() {
        for location in 0..3 {
            let group: Vec<_> = state
                .units
                .iter()
                .filter(|unit| unit.owner == owner && unit.location == location)
                .collect();
            let max = if location == 0 { 10 } else { 5 };
            let count = group.len().min(max);
            for (index, unit) in group.iter().take(max).enumerate() {
                let r = if location == 0 {
                    let width = (770.0 / count.max(1) as f32 - 5.0).min(132.0);
                    Rect::new(
                        base.x + 105.0 + index as f32 * (width + 5.0),
                        base.y + 2.0,
                        width,
                        72.0,
                    )
                } else {
                    let field = fields[location - 1];
                    let width = (384.0 / count.max(1) as f32 - 6.0).min(132.0);
                    Rect::new(
                        field.x
                            + 45.0
                            + (384.0 - count as f32 * (width + 6.0)) * 0.5
                            + index as f32 * (width + 6.0),
                        field.y + if owner == 1 { 66.0 } else { 150.0 },
                        width,
                        76.0,
                    )
                };
                hits.units.push((unit.id, screen(r)));
            }
        }
    }
    // Fireflies and warm sparks float above the miniature landscapes.
    for i in 0..16 {
        let f = i as f32;
        let side = if i < 9 { -1. } else { 1. };
        let p = vec3(
            side * 2.65 + (f * 2.7 + state.time * 0.07).sin() * 1.3,
            0.7 + (f * 1.3 + state.time * 0.3).sin() * 0.23 + (i % 4) as f32 * 0.2,
            (f * 1.7 + state.time * 0.08).cos() * 1.25,
        );
        let color = if side < 0. {
            rgb(135, 239, 194)
        } else {
            rgb(252, 204, 110)
        };
        animated.crystal(p, 0.013, 0.036, color);
    }
    for mesh in &animated.meshes {
        draw_mesh(mesh);
    }
    set_default_camera();
    hits
}
