// ═══════════════════════════════════════════════════════════════════════════
// RO'YXATDAN O'TISH VA KIRISH MANTIG'I
//
// UI faqat maydonlarni ko'rsatadi — tekshiruv va hisob yaratish shu yerda.
// Slint tomondan `Auth` globali orqali chaqiriladi (src/lib.rs'da ulanadi).
//
// Backend hali yo'q, shuning uchun hisoblar ilova ishlab turgan vaqtda
// XOTIRADA saqlanadi: ro'yxatdan o'tilgan hisobga kirish mumkin, o'tmaganiga
// yo'q. Ilova yopilsa ro'yxat ham yo'qoladi — bu ataylab: diskka zaif parol
// xeshini yozib qo'ymaymiz.
//
// DIQQAT: parol bu yerda oddiy xesh bilan taqqoslanadi. Bu HAQIQIY himoya
// EMAS — real tizimda parol serverda Argon2id/bcrypt bilan xeshlanadi va
// tekshiruv ham server tomonda bo'ladi. Bu yerdagi maqsad — oqim mantig'i
// to'g'ri bo'lishi (parol so'raladi, tekshiriladi, kirishda mos kelishi kerak).
// ═══════════════════════════════════════════════════════════════════════════

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct Account {
    pub email: String,
    pub name: String,
    pub workplace: String,
    /// `None` — hisob Google orqali ochilgan, paroli yo'q.
    pub pass: Option<u64>,
}

thread_local! {
    static DB: RefCell<Vec<Account>> = RefCell::new(Vec::new());
}

// ── TEKSHIRUVLAR (UI ularni har bosishda emas, matn o'zgarganda chaqiradi) ──

pub fn norm_email(e: &str) -> String {
    e.trim().to_lowercase()
}

/// E-pochta shakli to'g'rimi: bitta `@`, bo'sh joysiz, domenda nuqta bor va
/// oxirgi bo'lagi (TLD) kamida ikki harf.
pub fn email_ok(e: &str) -> bool {
    let e = e.trim();
    if e.chars().count() < 6 || e.len() > 254 || e.chars().any(char::is_whitespace) {
        return false;
    }
    let mut parts = e.split('@');
    let (local, domain) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    if parts.next().is_some() || local.is_empty() || domain.is_empty() {
        return false;
    }
    if local.starts_with('.') || local.ends_with('.') {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        return false;
    }
    let tld = labels[labels.len() - 1];
    tld.chars().count() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
}

/// Ism va familiya: kamida ikki so'z, har biri ikki belgidan uzun.
/// O'zbek ismlarida apostrof va chiziqcha uchraydi (G'ulom, Sayid-Ali).
pub fn name_ok(n: &str) -> bool {
    let words: Vec<&str> = n.trim().split_whitespace().collect();
    words.len() >= 2
        && words.iter().all(|w| w.chars().count() >= 2)
        && n.chars().all(|c| {
            c.is_alphabetic() || c.is_whitespace() || c == '\'' || c == '\u{2019}' || c == '-'
        })
}

/// Parol kuchi 0..4 — UI shu raqamdan chizma va izoh yasaydi.
pub fn password_score(p: &str) -> i32 {
    let n = p.chars().count();
    if n == 0 {
        return 0;
    }
    let mut s = 0;
    if n >= 8 {
        s += 1;
    }
    if n >= 12 {
        s += 1;
    }
    if p.chars().any(char::is_lowercase) && p.chars().any(char::is_uppercase) {
        s += 1;
    }
    if p.chars().any(|c| c.is_ascii_digit()) {
        s += 1;
    }
    if p.chars().any(|c| !c.is_alphanumeric()) {
        s += 1;
    }
    // Sakkiz belgidan qisqa parol hech qachon "yaxshi" bo'lmaydi.
    if n < 8 { s.min(1) } else { s.min(4) }
}

/// Ismning birinchi so'zi — "Xush kelibsiz, Aziz!" uchun.
pub fn first_name(n: &str) -> String {
    n.trim().split_whitespace().next().unwrap_or("").to_string()
}

/// Avatar uchun bosh harflar: "Aziz Bekmurodov" → "AB".
pub fn initials(n: &str) -> String {
    n.split_whitespace()
        .take(2)
        .filter_map(|w| w.chars().next())
        .flat_map(|c| c.to_uppercase())
        .collect()
}

// Har hisobga alohida "tuz" (e-pochta) — bir xil parollar bir xil xesh
// bermasin. Yana takrorlaymiz: bu KDF emas, demo taqqoslash.
fn digest(email: &str, pass: &str) -> u64 {
    let mut h = DefaultHasher::new();
    ("cyberkent-demo", email, pass).hash(&mut h);
    h.finish()
}

// ── AMALLAR ────────────────────────────────────────────────────────────────

/// Yangi hisob. `google` — ma'lumotlar Google hisobidan olingan, u holda
/// parol so'ralmaydi (kirishni Google tasdiqlaydi).
pub fn register(
    email: &str,
    name: &str,
    workplace: &str,
    password: &str,
    google: bool,
) -> Result<Account, String> {
    let email = norm_email(email);
    let name = name.trim().to_string();
    let workplace = workplace.trim().to_string();

    if !email_ok(&email) {
        return Err("E-pochta noto'g'ri — masalan: siz@gmail.com".into());
    }
    if !name_ok(&name) {
        return Err("Ism va familiyani to'liq kiriting".into());
    }
    if workplace.chars().count() < 2 {
        return Err("Ish yoki o'qish joyini kiriting".into());
    }

    let pass = if google {
        None
    } else {
        if password.chars().count() < 8 {
            return Err("Parol kamida 8 belgi bo'lishi kerak".into());
        }
        if password_score(password) < 2 {
            return Err("Parol juda oddiy — harf, raqam va belgi qo'shing".into());
        }
        Some(digest(&email, password))
    };

    DB.with(|db| {
        let mut db = db.borrow_mut();
        if db.iter().any(|a| a.email == email) {
            return Err("Bu e-pochta allaqachon ro'yxatdan o'tgan — kirishga o'ting".to_string());
        }
        let acc = Account { email, name, workplace, pass };
        db.push(acc.clone());
        Ok(acc)
    })
}

/// E-pochta + parol bilan kirish.
pub fn login(email: &str, password: &str) -> Result<Account, String> {
    let email = norm_email(email);
    if !email_ok(&email) {
        return Err("E-pochta noto'g'ri — masalan: siz@gmail.com".into());
    }
    DB.with(|db| {
        let db = db.borrow();
        let Some(acc) = db.iter().find(|a| a.email == email) else {
            return Err("Bu e-pochta bilan hisob topilmadi — ro'yxatdan o'ting".to_string());
        };
        match acc.pass {
            None => Err("Bu hisob Google orqali ochilgan — \"Google bilan kirish\"ni bosing".into()),
            Some(h) if h == digest(&email, password) => Ok(acc.clone()),
            Some(_) => Err("Parol noto'g'ri".into()),
        }
    })
}

/// Google hisobi bilan kirish. Parol bilan ochilgan hisobga ham ruxsat —
/// e-pochtani Google tasdiqlagan, demak egasi shu.
pub fn google_login(email: &str) -> Result<Account, String> {
    let email = norm_email(email);
    DB.with(|db| {
        db.borrow()
            .iter()
            .find(|a| a.email == email)
            .cloned()
            .ok_or_else(|| {
                "Bu Google hisobi ro'yxatdan o'tmagan — avval ro'yxatdan o'ting".to_string()
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_shapes() {
        assert!(email_ok("aziz@gmail.com"));
        assert!(email_ok(" Aziz@Gmail.COM "));
        assert!(!email_ok("aziz@gmail"));
        assert!(!email_ok("aziz@@gmail.com"));
        assert!(!email_ok("@gmail.com"));
        assert!(!email_ok("aziz gmail.com"));
        assert!(!email_ok("aziz@gmail.c"));
    }

    #[test]
    fn avatar_initials() {
        assert_eq!(initials("Aziz Bekmurodov"), "AB");
        assert_eq!(initials("malika yusupova ovna"), "MY");
        assert_eq!(initials(""), "");
    }

    #[test]
    fn names() {
        assert!(name_ok("Aziz Bekmurodov"));
        assert!(name_ok("G'ulom O'ktam-Ali"));
        assert!(!name_ok("Aziz"));
        assert!(!name_ok("Aziz B"));
        assert!(!name_ok("Aziz 123"));
    }

    #[test]
    fn scores() {
        assert_eq!(password_score(""), 0);
        assert_eq!(password_score("abc"), 0);
        assert!(password_score("parol123") >= 2);
        assert!(password_score("Cyber!2026kent") >= 4);
    }

    #[test]
    fn register_then_login() {
        assert!(register("a@b.uz", "Aziz Bek", "TATU", "short", false).is_err());
        assert!(register("a@b.uz", "Aziz", "TATU", "parol1234", false).is_err());
        assert!(register("a@b.uz", "Aziz Bek", "TATU", "parol1234", false).is_ok());
        // Ikkinchi marta bo'lmaydi.
        assert!(register("A@B.uz", "Aziz Bek", "TATU", "parol1234", false).is_err());
        assert!(login("a@b.uz", "parol1234").is_ok());
        assert!(login("a@b.uz", "boshqa1234").is_err());
        assert!(login("yoq@b.uz", "parol1234").is_err());
        // Google hisobi — parolsiz.
        assert!(register("g@b.uz", "Malika Yusupova", "IT Park", "", true).is_ok());
        assert!(login("g@b.uz", "parol1234").is_err());
        assert!(google_login("g@b.uz").is_ok());
        assert!(google_login("yoq@b.uz").is_err());
    }
}
