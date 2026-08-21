// ═══════════════════════════════════════════════════════════════════════════
// XARITA RENDERI — Flutter `interactive_map_screen.dart` bilan 1:1.
//
// Tadbir binosining neon-izometrik plani: tashqi kontur, yo'laklar, olti xona
// (usti shaffof rang bilan to'ldirilgan, qirralari yorqin, pastga "etak"
// chiziqlari), kirish markeri va tanlangan xonaga marshrut.
//
// Sof CPU: alfa-aralashtiruvchi 2D kanvas (chiziq/ko'pburchak/doira), natija
// `slint::Image` bo'lib `Map3D.scene`ga boradi. Aylanish (rot) Flutter'dagi
// `_rotate` bilan bir xil — markaz (0.5,0.5) atrofida.
//
// Yorliq (pill) joylari Rust'da proyeksiya qilinib `Map3D.labels` modeliga
// yoziladi — Slint ularni rasm ustida chizadi (shrift Slint tomonda).
// ═══════════════════════════════════════════════════════════════════════════

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

#[derive(Clone, Copy)]
pub struct Room {
    pub code: &'static str,
    pub name: &'static str,
    // normallashgan reja: (x, y, w, h)
    pub rect: (f32, f32, f32, f32),
    pub color: [u8; 3],
    pub entrance: bool,
}

// Kirishdan har xonaga marshrut (normallashgan nuqtalar) — Flutter bilan aynan.
pub const ROUTES: [&[(f32, f32)]; 6] = [
    &[(0.50, 0.86), (0.50, 0.27)],                     // B2
    &[(0.50, 0.86), (0.50, 0.50), (0.43, 0.50)],       // D4
    &[(0.50, 0.86), (0.50, 0.50), (0.57, 0.50)],       // C3
    &[(0.50, 0.86), (0.50, 0.73), (0.43, 0.73)],       // E5
    &[(0.50, 0.86), (0.50, 0.73), (0.57, 0.73)],       // F6
    &[],                                               // A1 (kirish)
];

pub const ROOMS: [Room; 6] = [
    Room { code: "B2", name: "Asosiy sahna", rect: (0.10, 0.05, 0.80, 0.22), color: [0x00, 0xE5, 0xFF], entrance: false },
    Room { code: "D4", name: "Workshop zonasi", rect: (0.10, 0.40, 0.33, 0.20), color: [0xFF, 0xB5, 0x47], entrance: false },
    Room { code: "C3", name: "CTF arena", rect: (0.57, 0.40, 0.33, 0.20), color: [0xFF, 0x55, 0x77], entrance: false },
    Room { code: "E5", name: "Quiz zonasi", rect: (0.10, 0.64, 0.33, 0.18), color: [0x7C, 0x4D, 0xFF], entrance: false },
    Room { code: "F6", name: "Sponsorlar", rect: (0.57, 0.64, 0.33, 0.18), color: [0xFF, 0x2D, 0x95], entrance: false },
    Room { code: "A1", name: "Kirish", rect: (0.42, 0.87, 0.16, 0.10), color: [0x5B, 0xFF, 0xC2], entrance: true },
];

// Yo'laklar (normallashgan to'rtburchaklar).
const CORRIDORS: [(f32, f32, f32, f32); 2] = [
    (0.46, 0.30, 0.08, 0.56),
    (0.08, 0.30, 0.84, 0.06),
];

// Old qirralardan pastga tushadigan "etak" balandligi (piksel).
const SKIRT: f32 = 38.0;

// ── PROYEKSIYA (Flutter bilan bir xil) ──────────────────────────────────────

struct Proj {
    ux: f32,
    uy: f32,
    ox: f32,
    oy: f32,
    rot: f32,
}

impl Proj {
    fn new(w: f32, h: f32, rot: f32) -> Self {
        let unit = (w * 0.46).min(h * 0.34);
        let uy = unit * 1.27;
        Proj { ux: unit, uy, ox: w / 2.0, oy: h / 2.0 - uy, rot }
    }

    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        let (dx, dy) = (x - 0.5, y - 0.5);
        let (co, si) = (self.rot.cos(), self.rot.sin());
        let (rx, ry) = (0.5 + dx * co - dy * si, 0.5 + dx * si + dy * co);
        (self.ox + (rx - ry) * self.ux, self.oy + (rx + ry) * self.uy)
    }
}

// ── KANVAS ──────────────────────────────────────────────────────────────────

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Canvas { w, h, px: vec![[0, 0, 0, 0]; w * h] }
    }

    // Alfa bilan ustiga chizish.
    fn blend(&mut self, x: i32, y: i32, c: [u8; 3], a: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 || a <= 0.0 {
            return;
        }
        let i = y as usize * self.w + x as usize;
        let d = &mut self.px[i];
        let a = a.min(1.0);
        let na = a + d[3] as f32 / 255.0 * (1.0 - a);
        if na <= 0.0 {
            return;
        }
        for k in 0..3 {
            let s = c[k] as f32 * a + d[k] as f32 * (d[3] as f32 / 255.0) * (1.0 - a);
            d[k] = (s / na) as u8;
        }
        d[3] = (na * 255.0) as u8;
    }

    // Qalinlik va yumshoq chetli chiziq.
    fn line(&mut self, a: (f32, f32), b: (f32, f32), width: f32, c: [u8; 3], alpha: f32) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(0.0001);
        let (nx, ny) = (dx / len, dy / len);
        let r = width / 2.0 + 0.75;
        let (min_x, max_x) = ((a.0.min(b.0) - r) as i32, (a.0.max(b.0) + r) as i32);
        let (min_y, max_y) = ((a.1.min(b.1) - r) as i32, (a.1.max(b.1) + r) as i32);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (px, py) = (x as f32 - a.0, y as f32 - a.1);
                let t = (px * nx + py * ny).clamp(0.0, len);
                let (cx, cy) = (a.0 + nx * t, a.1 + ny * t);
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                let cov = (width / 2.0 + 0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, c, alpha * cov);
                }
            }
        }
    }

    // Neon chiziq: keng xira + o'rta + ingichka yorqin qatlamlar.
    fn neon(&mut self, a: (f32, f32), b: (f32, f32), c: [u8; 3], strength: f32) {
        self.line(a, b, 7.0, c, 0.10 * strength);
        self.line(a, b, 3.6, c, 0.25 * strength);
        self.line(a, b, 1.6, c, 0.95 * strength);
    }

    // Qavariq to'rtburchakni to'ldirish (skanlash usuli).
    fn fill_quad(&mut self, p: [(f32, f32); 4], c: [u8; 3], alpha: f32) {
        let min_y = p.iter().map(|v| v.1).fold(f32::MAX, f32::min).floor() as i32;
        let max_y = p.iter().map(|v| v.1).fold(f32::MIN, f32::max).ceil() as i32;
        for y in min_y..=max_y {
            let fy = y as f32 + 0.5;
            let mut xs: Vec<f32> = Vec::with_capacity(4);
            for i in 0..4 {
                let (a1, b1) = (p[i], p[(i + 1) % 4]);
                if (a1.1 <= fy && b1.1 > fy) || (b1.1 <= fy && a1.1 > fy) {
                    let t = (fy - a1.1) / (b1.1 - a1.1);
                    xs.push(a1.0 + (b1.0 - a1.0) * t);
                }
            }
            if xs.len() >= 2 {
                xs.sort_by(|m, n| m.partial_cmp(n).unwrap());
                let (x0, x1) = (xs[0].round() as i32, xs[xs.len() - 1].round() as i32);
                for x in x0..=x1 {
                    self.blend(x, y, c, alpha);
                }
            }
        }
    }

    fn circle(&mut self, cx: f32, cy: f32, radius: f32, c: [u8; 3], alpha: f32) {
        let r = radius + 1.0;
        for y in (cy - r) as i32..=(cy + r) as i32 {
            for x in (cx - r) as i32..=(cx + r) as i32 {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                let cov = (radius + 0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, c, alpha * cov);
                }
            }
        }
    }

    fn ring(&mut self, cx: f32, cy: f32, radius: f32, width: f32, c: [u8; 3], alpha: f32) {
        let r = radius + width;
        for y in (cy - r) as i32..=(cy + r) as i32 {
            for x in (cx - r) as i32..=(cx + r) as i32 {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                let cov = (width / 2.0 + 0.5 - (d - radius).abs()).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, c, alpha * cov);
                }
            }
        }
    }
}

const CYAN: [u8; 3] = [0x00, 0xE5, 0xFF];
const MINT: [u8; 3] = [0x5B, 0xFF, 0xC2];

// To'rtburchak (normallashgan) burchaklarini proyeksiya qiladi.
fn quad(p: &Proj, r: (f32, f32, f32, f32)) -> [(f32, f32); 4] {
    let (x, y, w, h) = r;
    [
        p.point(x, y),
        p.point(x + w, y),
        p.point(x + w, y + h),
        p.point(x, y + h),
    ]
}

/// Yorliq (pill) ankeri — Slint tomonda chiziladi.
pub struct Label {
    pub x: f32,
    pub y: f32,
    pub code: &'static str,
    pub name: &'static str,
    pub color: [u8; 3],
    pub entrance: bool,
}

/// Sahnani chizadi va yorliq joylarini qaytaradi.
/// `rot` — radianda aylanish; `selected` — marshrut ko'rsatiladigan xona
/// (0..5, salbiy bo'lsa yo'q).
pub fn render_with_labels(rot: f32, selected: i32, w: u32, h: u32) -> (Image, Vec<Label>) {
    let mut cv = Canvas::new(w as usize, h as usize);
    let p = Proj::new(w as f32, h as f32, rot);

    // ── Tashqi kontur (bino) ────────────────────────────────────────────
    let outer = quad(&p, (0.0, 0.0, 1.0, 1.0));
    // ostki xira to'ldirish
    cv.fill_quad(outer, [0x0a, 0x12, 0x2a], 0.55);
    for i in 0..4 {
        cv.neon(outer[i], outer[(i + 1) % 4], CYAN, 0.8);
    }
    // etak (old ikki qirra pastga)
    for i in 1..3 {
        let a = outer[i];
        let b = outer[(i + 1) % 4];
        cv.line(a, (a.0, a.1 + SKIRT), 1.4, CYAN, 0.5);
        cv.line(b, (b.0, b.1 + SKIRT), 1.4, CYAN, 0.5);
        cv.line((a.0, a.1 + SKIRT), (b.0, b.1 + SKIRT), 1.4, CYAN, 0.35);
    }

    // ── Yo'laklar ───────────────────────────────────────────────────────
    for c in CORRIDORS {
        let q = quad(&p, c);
        cv.fill_quad(q, CYAN, 0.05);
        for i in 0..4 {
            cv.line(q[i], q[(i + 1) % 4], 1.0, CYAN, 0.22);
        }
    }

    // ── Xonalar (orqadan oldinga — reja bo'yicha y kichigi avval) ───────
    for (ri, room) in ROOMS.iter().enumerate() {
        let q = quad(&p, room.rect);
        // usti
        cv.fill_quad(q, room.color, 0.16);
        // etak: old qirralar pastga (2 va 3-qirralar old tomonda)
        for i in 1..3 {
            let a = q[i];
            let b = q[(i + 1) % 4];
            // yon yuzani xira to'ldirish
            cv.fill_quad([a, b, (b.0, b.1 + SKIRT), (a.0, a.1 + SKIRT)], room.color, 0.07);
            cv.line(a, (a.0, a.1 + SKIRT), 1.3, room.color, 0.55);
            cv.line(b, (b.0, b.1 + SKIRT), 1.3, room.color, 0.55);
            cv.line((a.0, a.1 + SKIRT), (b.0, b.1 + SKIRT), 1.3, room.color, 0.4);
        }
        // yorqin qirralar
        for i in 0..4 {
            cv.neon(q[i], q[(i + 1) % 4], room.color, 1.0);
        }
        // xona ichidagi kichik nuqta (eshik)
        if !room.entrance {
            let door = ROUTES[ri].last().copied().unwrap_or((
                room.rect.0 + room.rect.2 / 2.0,
                room.rect.1 + room.rect.3 / 2.0,
            ));
            let d = p.point(door.0, door.1);
            cv.ring(d.0, d.1, 5.0, 1.6, room.color, 0.9);
            cv.circle(d.0, d.1, 2.0, room.color, 1.0);
        }
    }

    // ── Kirish markeri ("Siz shu yerdasiz") ─────────────────────────────
    let a1 = ROOMS[5];
    let m = p.point(
        a1.rect.0 + a1.rect.2 / 2.0,
        a1.rect.1 + a1.rect.3 / 2.0,
    );
    cv.circle(m.0, m.1, 16.0, MINT, 0.12);
    cv.ring(m.0, m.1, 11.0, 1.6, MINT, 0.7);
    cv.circle(m.0, m.1, 4.5, MINT, 1.0);

    // ── Tanlangan xonaga marshrut ───────────────────────────────────────
    if selected >= 0 && (selected as usize) < ROUTES.len() {
        let route = ROUTES[selected as usize];
        if route.len() >= 2 {
            for seg in route.windows(2) {
                let a = p.point(seg[0].0, seg[0].1);
                let b = p.point(seg[1].0, seg[1].1);
                // punktir: segmentni bo'laklarga bo'lamiz
                let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
                let n = (len / 14.0).max(1.0) as i32;
                for k in 0..n {
                    let t0 = k as f32 / n as f32;
                    let t1 = t0 + 0.55 / n as f32;
                    let pa = (a.0 + (b.0 - a.0) * t0, a.1 + (b.1 - a.1) * t0);
                    let pb = (a.0 + (b.0 - a.0) * t1, a.1 + (b.1 - a.1) * t1);
                    cv.neon(pa, pb, MINT, 0.9);
                }
            }
            // manzil nuqtasi
            let last = route.last().unwrap();
            let d = p.point(last.0, last.1);
            cv.ring(d.0, d.1, 8.0, 2.0, MINT, 0.9);
        }
    }

    // ── Yorliq ankerlarini yig'ish ──────────────────────────────────────
    let labels = ROOMS
        .iter()
        .map(|r| {
            let c = p.point(r.rect.0 + r.rect.2 / 2.0, r.rect.1 + r.rect.3 / 2.0);
            Label {
                x: c.0 / w as f32,
                y: c.1 / h as f32,
                code: r.code,
                name: r.name,
                color: r.color,
                entrance: r.entrance,
            }
        })
        .collect();

    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    let out = buf.make_mut_slice();
    for (i, px) in cv.px.iter().enumerate() {
        out[i] = Rgba8Pixel { r: px[0], g: px[1], b: px[2], a: px[3] };
    }
    (Image::from_rgba8(buf), labels)
}
