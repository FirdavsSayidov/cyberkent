// Barcha ekranlarni PNG'ga tushiradi — UI ustida tez ishlash uchun.
//
//   cargo run --example shots -- <papka>
//
// APK yig'ib emulyatorga o'rnatishdan ancha tez: bitta ishga tushirishda hamma
// sahifa rasmga olinadi, haqiqiy renderer bilan (ko'rinish aynan shunday).
//
// Diqqat: xususiyat o'zgargandan keyin joylashuv (layout) hodisa halqasining
// keyingi qadamida qayta hisoblanadi. Shuning uchun bir qadamda holat
// o'rnatiladi, KEYINGI qadamda rasmga olinadi.

use slint_app::*;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

/// Bitta ekran holati.
struct Screen {
    name: &'static str,
    phase: CkPhase,
    page: i32,
    overlay: CkOverlay,
    insp: CkInsp,
    sell_tab: i32,
    sell: CkSell,
}

fn s(name: &'static str, phase: CkPhase) -> Screen {
    Screen {
        name,
        phase,
        page: 0,
        overlay: CkOverlay::None,
        insp: CkInsp::Home,
        sell_tab: 0,
        sell: CkSell::None,
    }
}

fn tab(name: &'static str, page: i32) -> Screen {
    Screen { page, ..s(name, CkPhase::App) }
}

fn ov(name: &'static str, page: i32, overlay: CkOverlay) -> Screen {
    Screen { overlay, ..tab(name, page) }
}

fn insp(name: &'static str, insp: CkInsp) -> Screen {
    Screen { insp, ..s(name, CkPhase::Inspector) }
}

fn sell(name: &'static str, sell_tab: i32, sell: CkSell) -> Screen {
    Screen { sell_tab, sell, ..s(name, CkPhase::Seller) }
}

fn screens() -> Vec<Screen> {
    vec![
        s("01-splash", CkPhase::Splash),
        s("02-onboarding", CkPhase::Onboarding),
        s("03-entry", CkPhase::Entry),
        s("04-roles", CkPhase::Roles),
        s("05-register", CkPhase::Register),
        s("06-login", CkPhase::Login),
        s("07-welcome", CkPhase::Welcome),
        tab("10-home", 0),
        tab("11-events", 1),
        tab("12-wallet", 2),
        tab("13-shop", 3),
        tab("14-profile", 4),
        tab("15-map", 5),
        ov("20-event-detail", 1, CkOverlay::EventDetail),
        ov("21-event-register", 1, CkOverlay::EventRegister),
        ov("22-entry-qr", 0, CkOverlay::EntryQr),
        ov("23-schedule", 1, CkOverlay::Schedule),
        ov("24-stream", 0, CkOverlay::Stream),
        ov("25-tasks", 0, CkOverlay::Tasks),
        ov("26-games", 0, CkOverlay::Games),
        ov("27-game-play", 0, CkOverlay::GamePlay),
        ov("28-product", 3, CkOverlay::Product),
        ov("29-payment-qr", 3, CkOverlay::PaymentQr),
        ov("30-orders", 3, CkOverlay::Orders),
        ov("31-order-detail", 3, CkOverlay::OrderDetail),
        ov("32-order-success", 3, CkOverlay::OrderSuccess),
        ov("33-tx-history", 2, CkOverlay::TxHistory),
        ov("34-send", 2, CkOverlay::Send),
        ov("35-receive", 2, CkOverlay::Receive),
        ov("36-settings", 4, CkOverlay::Settings),
        ov("37-news", 4, CkOverlay::News),
        ov("38-notifications", 0, CkOverlay::Notifications),
        ov("40-quiz", 0, CkOverlay::Quiz),
        ov("41-quiz-tests", 0, CkOverlay::QuizTests),
        ov("42-quiz-editor", 0, CkOverlay::QuizEditor),
        ov("43-quiz-scan", 0, CkOverlay::QuizScan),
        ov("44-quiz-result", 0, CkOverlay::QuizResult),
        s("50-insp-login", CkPhase::InspectorLogin),
        insp("51-insp-home", CkInsp::Home),
        insp("52-insp-scan", CkInsp::Scan),
        insp("53-insp-result", CkInsp::Result),
        insp("54-insp-success", CkInsp::Success),
        insp("55-insp-rejected", CkInsp::Rejected),
        insp("56-insp-visitors", CkInsp::Visitors),
        s("60-sell-login", CkPhase::SellerLogin),
        sell("61-sell-orders", 0, CkSell::None),
        sell("62-sell-sales", 2, CkSell::None),
        sell("63-sell-stock", 3, CkSell::None),
        sell("64-sell-scanner", 0, CkSell::Scanner),
        sell("65-sell-completed", 0, CkSell::Completed),
    ]
}

type Buf = slint::SharedPixelBuffer<slint::Rgba8Pixel>;

fn write_png(path: &PathBuf, w: u32, h: u32, data: &[u8]) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(data).unwrap();
}

/// Hamma ekranni bitta setkaga joylaydi — birdan ko'rib chiqish uchun.
fn sheet(path: &PathBuf, cells: &[Buf], cols: u32, tw: u32, th: u32) {
    if cells.is_empty() {
        return;
    }
    let gap = 8u32;
    let rows = (cells.len() as u32).div_ceil(cols);
    let (w, h) = (cols * tw + (cols + 1) * gap, rows * th + (rows + 1) * gap);
    let mut out = vec![0u8; (w * h * 4) as usize];
    for i in (3..out.len()).step_by(4) {
        out[i] = 255;
    }
    for (i, buf) in cells.iter().enumerate() {
        let (cx, cy) = (i as u32 % cols, i as u32 / cols);
        let (ox, oy) = (gap + cx * (tw + gap), gap + cy * (th + gap));
        let (sw, sh) = (buf.width(), buf.height());
        let src = buf.as_bytes();
        for y in 0..th {
            for x in 0..tw {
                let s = ((y * sh / th * sw + x * sw / tw) * 4) as usize;
                let d = (((oy + y) * w + ox + x) * 4) as usize;
                out[d..d + 4].copy_from_slice(&src[s..s + 4]);
            }
        }
    }
    write_png(path, w, h, &out);
    println!("SHEET {}", path.display());
}

/// Bo'laklarni ustma-ust (vertikal) ulaydi.
fn stitch(parts: &[Buf]) -> (u32, u32, Vec<u8>) {
    let w = parts[0].width();
    let h: u32 = parts.iter().map(|p| p.height()).sum();
    let mut out = vec![0u8; (w * h * 4) as usize];
    let mut y0 = 0u32;
    for p in parts {
        let src = p.as_bytes();
        for y in 0..p.height() {
            let d = (((y0 + y) * w) * 4) as usize;
            let s = ((y * w) * 4) as usize;
            out[d..d + (w * 4) as usize].copy_from_slice(&src[s..s + (w * 4) as usize]);
        }
        y0 += p.height();
    }
    (w, h, out)
}

// Har sahifadan nechta ko'rinish olinadi (pastga surib).
const SLICES: usize = 1;

fn main() {
    let dir =
        PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "/tmp/ck-shots".into()));
    std::fs::create_dir_all(&dir).unwrap();
    // Faqat bitta ekran kerak bo'lsa: cargo run --example shots -- <papka> home
    let only = std::env::args().nth(2);
    // Uchinchi argument — oyna balandligi. Baland oyna butun aylanadigan
    // sahifani bitta rasmga sig'diradi (masalan 2800).
    let win_h: f32 = std::env::args()
        .nth(3)
        .and_then(|v| v.parse().ok())
        .unwrap_or(844.0);

    let app = CyberKentApp::new().unwrap();
    // Rust tomonidagi ko'priklar (3D xarita, auth) — aks holda xarita bo'sh chiqadi.
    slint_app::wire_auth(&app);
    slint_app::wire_map3d(&app);
    app.window().set_size(slint::LogicalSize::new(390.0, win_h));
    app.show().unwrap();

    let list: Vec<Screen> = match &only {
        Some(f) => screens().into_iter().filter(|s| s.name.contains(f.as_str())).collect(),
        None => screens(),
    };
    let list = Rc::new(list);
    let shots: Rc<std::cell::RefCell<Vec<Buf>>> = Rc::new(std::cell::RefCell::new(Vec::new()));
    let slices: Rc<std::cell::RefCell<Vec<Buf>>> = Rc::new(std::cell::RefCell::new(Vec::new()));
    let step = Rc::new(Cell::new(0usize));

    let timer = Box::leak(Box::new(slint::Timer::default()));
    let weak = app.as_weak();
    let (list2, shots2, step2, dir2) = (list.clone(), shots.clone(), step.clone(), dir.clone());
    let slices2 = slices.clone();
    let win_h2 = win_h;

    timer.start(slint::TimerMode::Repeated, Duration::from_millis(200), move || {
        let app = weak.upgrade().unwrap();
        let raw = step2.get();
        // Boshlang'ich bo'sh qadamlar.
        if raw < 4 {
            step2.set(raw + 1);
            let _ = app.window().take_snapshot();
            return;
        }
        let step = raw - 4;
        // Har ekranga TERS (STEPS) sub-qadam: 0=holat o'rnat, 1..STEPS-2=
        // sokinlashtir (discard), oxirgi=rasmga ol. Overlay repeater'lari
        // birinchi qadamda yaratiladi, keyingilarda joylashadi.
        const STEPS: usize = 6;
        let slot = step / STEPS;
        let i = slot / SLICES;
        let slice = slot % SLICES;
        let phase = step % STEPS;
        step2.set(raw + 1);

        if phase == 0 {
            if i >= list2.len() {
                sheet(&dir2.join("00-sheet.png"), &shots2.borrow(), 6, 260, 562);
                slint::quit_event_loop().unwrap();
                return;
            }
            let sc = &list2[i];
            app.set_phase(sc.phase);
            app.set_page(sc.page);
            app.set_overlay(sc.overlay);
            app.set_insp(sc.insp);
            app.set_sell_tab(sc.sell_tab);
            app.set_sell(sc.sell);
            app.global::<CkDebug>()
                .set_scroll(slice as f32 * (win_h2 - 40.0));
            return;
        }

        if phase != STEPS - 1 {
            // Joylashuvni sokinlashtiruvchi bo'sh qadamlar.
            let _ = app.window().take_snapshot();
            return;
        }
        if i >= list2.len() {
            return;
        }
        let sc = &list2[i];
        // Birinchi chaqiruv sahnani qayta chizdiradi, ikkinchisi yangi kadrni beradi.
        let _ = app.window().take_snapshot();
        let _ = app.window().take_snapshot();
        let buf = app.window().take_snapshot().unwrap();
        let mut acc = shots2.borrow_mut();
        let last_same = slices2
            .borrow()
            .last()
            .map(|p: &Buf| p.as_bytes() == buf.as_bytes())
            .unwrap_or(false);
        if !last_same {
            slices2.borrow_mut().push(buf);
        }
        // Oxirgi bo'lakdan keyin ulab saqlaymiz.
        if slice + 1 == SLICES {
            let parts = std::mem::take(&mut *slices2.borrow_mut());
            let full = stitch(&parts);
            write_png(
                &dir2.join(format!("{}.png", sc.name)),
                full.0,
                full.1,
                &full.2,
            );
            // Kontakt-varaq uchun faqat birinchi ekran ko'rinishi.
            acc.push(parts.into_iter().next().unwrap());
            println!("  -> {}", sc.name);
        }
    });

    slint::run_event_loop().unwrap();
}
