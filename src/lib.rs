// Oynani ochadi va 3D xarita ko'prigini (Rust software renderer) ulaydi.
// Qolgan barcha UI, navigatsiya, holat — `ui/ck-*.slint` fayllarida.

use slint::{Model, ModelRc, SharedString, VecModel};
use std::error::Error;

// build.rs `ui/ck-app.slint`ni kompilyatsiya qiladi: CyberKentApp turi va
// Map3D globali paydo bo'ladi.
slint::include_modules!();

// Tadbir maydonining 3D ko'rinishi (sof CPU renderer).
mod map3d;
// Ro'yxatdan o'tish/kirish tekshiruvi va hisoblar.
mod auth;

// Xarita render o'lchami (Slint kartaga moslab cho'zadi).
const MAP_W: u32 = 720;
const MAP_H: u32 = 860;

pub fn run() -> Result<(), Box<dyn Error>> {
    let app = CyberKentApp::new()?;
    wire_auth(&app);
    wire_map3d(&app);
    wire_store(&app);
    wire_shop(&app);
    app.run()?;
    Ok(())
}

// Xaritani chizadi va yorliq ankerlarini Slint modeliga yozadi.
fn draw_map(app: &CyberKentApp) {
    let bridge = app.global::<Map3D>();
    let (scene, labels) =
        map3d::render_with_labels(bridge.get_yaw(), bridge.get_selected(), MAP_W, MAP_H);
    bridge.set_scene(scene);
    let rows: Vec<CkMapLabel> = labels
        .into_iter()
        .map(|l| CkMapLabel {
            x: l.x,
            y: l.y,
            code: l.code.into(),
            name: l.name.into(),
            tint: slint::Color::from_rgb_u8(l.color[0], l.color[1], l.color[2]),
            entrance: l.entrance,
        })
        .collect();
    bridge.set_labels(slint::ModelRc::new(slint::VecModel::from(rows)));
}

// `Map3D.refresh()` chaqirilganda (aylantirish tugmalari yoki xona tanlash)
// joriy rot/selected bilan sahnani qayta chizadi.
pub fn wire_map3d(app: &CyberKentApp) {
    draw_map(app);
    let weak = app.as_weak();
    app.global::<Map3D>().on_refresh(move || {
        if let Some(app) = weak.upgrade() {
            draw_map(&app);
        }
    });
}

// Ro'yxatdan o'tish/kirish. UI hech narsani o'zi tekshirmaydi — maydon
// holati ham, hisob yaratish ham shu callbacklar orqali `src/auth.rs`ga
// boradi. Amallar bo'sh satr qaytarsa muvaffaqiyat, aks holda xato matni.
pub fn wire_auth(app: &CyberKentApp) {
    let auth = app.global::<Auth>();

    // Maydon tekshiruvlari (matn o'zgarganda binding qayta hisoblanadi).
    auth.on_email_ok(|e| auth::email_ok(&e));
    auth.on_name_ok(|n| auth::name_ok(&n));
    auth.on_password_score(|p| auth::password_score(&p));

    let weak = app.as_weak();
    auth.on_register(move |email, name, workplace, password, google| {
        match auth::register(&email, &name, &workplace, &password, google) {
            Ok(acc) => {
                remember(&weak, &acc);
                SharedString::new()
            }
            Err(e) => e.into(),
        }
    });

    let weak = app.as_weak();
    auth.on_login(move |email, password| match auth::login(&email, &password) {
        Ok(acc) => {
            remember(&weak, &acc);
            SharedString::new()
        }
        Err(e) => e.into(),
    });

    let weak = app.as_weak();
    auth.on_google_login(move |email| match auth::google_login(&email) {
        Ok(acc) => {
            remember(&weak, &acc);
            SharedString::new()
        }
        Err(e) => e.into(),
    });
}

// Kirgan foydalanuvchini UI ko'rishi uchun globalga yozadi ("Xush kelibsiz,
// Aziz!", keyinchalik profil sahifasi).
fn remember(weak: &slint::Weak<CyberKentApp>, acc: &auth::Account) {
    if let Some(app) = weak.upgrade() {
        let g = app.global::<Auth>();
        g.set_account_name(acc.name.as_str().into());
        g.set_account_first(auth::first_name(&acc.name).as_str().into());
        g.set_account_email(acc.email.as_str().into());
        g.set_account_workplace(acc.workplace.as_str().into());
        g.set_account_initials(auth::initials(&acc.name).as_str().into());
    }
}

// CyberAqcha iqtisodi — haqiqiy holat (balans/kirim/chiqim) shu yerda.
// `Store.buy(price)` balansdan yechadi (yetsa), `Store.reward(amount)` qo'shadi.
// `Store.money(n)` butun sonni "1,250" ko'rinishida formatlaydi.
pub fn wire_store(app: &CyberKentApp) {
    let s = app.global::<Store>();
    s.on_money(|n| fmt_money(n).into());

    let weak = app.as_weak();
    s.on_buy(move |price| {
        if let Some(app) = weak.upgrade() {
            let st = app.global::<Store>();
            let bal = st.get_balance();
            if price > 0 && bal >= price {
                st.set_balance(bal - price);
                st.set_spent(st.get_spent() + price);
                return true;
            }
        }
        false
    });

    let weak = app.as_weak();
    s.on_reward(move |amount| {
        if let Some(app) = weak.upgrade() {
            let st = app.global::<Store>();
            if amount > 0 {
                st.set_balance(st.get_balance() + amount);
                st.set_earned(st.get_earned() + amount);
            }
        }
    });
}

// Do'kon qidiruvi/filtri — Slint'da massiv filtri yo'q, shuning uchun Rust.
// Master ro'yxatni qidiruv matni va tag bo'yicha filtrlab qaytaradi.
pub fn wire_shop(app: &CyberKentApp) {
    app.global::<ShopFilter>().on_run(|products, query, tag| {
        let q = query.to_lowercase();
        let tf = tag.to_lowercase();
        let out: Vec<CkProduct> = products
            .iter()
            .filter(|p| {
                let tag_ok = tf.is_empty() || p.tag.to_lowercase() == tf;
                let q_ok = q.is_empty() || p.title.to_lowercase().contains(&q);
                tag_ok && q_ok
            })
            .collect();
        ModelRc::new(VecModel::from(out))
    });
}

// Butun sonni minglik ajratgichlar bilan formatlaydi: 1250 → "1,250".
fn fmt_money(n: i32) -> String {
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let len = digits.len();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

// ZirhMobil xavfsizlik SDK'si — faqat Android'da.
#[cfg(target_os = "android")]
mod zirh;

// Android kirish nuqtasi — tizim cdylib ichidagi `android_main`ni chaqiradi.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    let _zirh = zirh::init();
    slint::android::init(app).unwrap();
    run().unwrap();
}
