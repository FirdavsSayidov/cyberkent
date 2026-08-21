//! ZirhMobil SDK ulanishi (Android).
//!
//! SDK `zirh/jni/<abi>/libmobil.so` ichida. U ikki xil interfeys ochadi:
//!
//!   * JNI — `Java_uz_zirh_zirhlib_ZirhMilliy_*`, `uz.zirh.zirhlib` Java
//!     sinflari uchun. Bizga to'g'ri kelmaydi: Java sinflarini APK'ga qo'shish
//!     Gradle talab qiladi, cargo-apk esa dex yasay olmaydi.
//!   * C FFI — `flutter_malumot_olish`, `flutter_malumot_almashish`,
//!     `flutter_xotirani_tozalash`. Java'siz ishlaydi, biz shulardan
//!     foydalanamiz.
//!
//! ## Nega passiv
//!
//! SDK'ning root / frida / emulyator / VPN / imzo / Play Market tekshiruvlari
//! faqat JavaVM mavjud bo'lganda ishga tushadi (`get_env_from_vm`). VM esa
//! `JNI_OnLoad` orqali beriladi, u ham faqat Java `System.loadLibrary` bilan
//! yuklanganda chaqiriladi. Biz kutubxonani `dlopen` bilan ochamiz, shuning
//! uchun VM berilmaydi va tekshiruvlar o'tkazib yuboriladi.
//!
//! Bu ataylab. Tekshiruv o'tmasa SDK jarayonni `SIGSEGV` bilan o'ldiradi, debug
//! kalit bilan imzolangan va `adb install` bilan o'rnatilgan APK esa imzo va
//! Play Market tekshiruvidan o'ta olmaydi — ilova ishga tushishda o'lardi.
//!
//! Release'da yoqish uchun [`Zirh::attach_vm`]ni chaqirish kifoya (va SDK
//! configini qo'shish — configsiz tekshiruvlar baribir o'tkazib yuboriladi).

use std::ffi::{c_char, c_int, c_void, CStr, CString};

// ── libmobil.so eksport qiladigan C funksiyalar ─────────────────────────────

type MalumotOlish = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type Tozalash = unsafe extern "C" fn(*mut c_char);
type JniOnLoad = unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int;

#[allow(clippy::type_complexity)]
type MalumotAlmashish = unsafe extern "C" fn(
    *const c_char, // url
    *const c_char, // method
    *const c_char, // body
    *const c_char, // headers (JSON)
    *const c_char, // file_path
    *const u8,     // file_bytes
    i32,           // bytes_len
    *const c_char, // file_name
    *const c_char, // file_field
) -> *mut c_char;

// ── logcat ──────────────────────────────────────────────────────────────────
// `log` va `android_logger` crate'larini qo'shmaslik uchun to'g'ridan-to'g'ri
// liblog. U Android'da har doim mavjud.

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

fn logcat(text: &str) {
    let tag = c"cyberkent";
    if let Ok(msg) = CString::new(text) {
        // 4 = ANDROID_LOG_INFO
        unsafe { __android_log_write(4, tag.as_ptr(), msg.as_ptr()) };
    }
}

/// Yuklangan SDK. `Library` tushib ketsa funksiya ko'rsatkichlari yaroqsiz
/// bo'ladi, shuning uchun u shu yerda ushlab turiladi.
// `lib` va `config`/`request` hozircha ilova ichida chaqirilmaydi — ular
// SDK'ning ochiq yuzasi, backend ulanganda ishlatiladi.
#[allow(dead_code)]
pub struct Zirh {
    lib: libloading::Library,
    olish: MalumotOlish,
    almashish: MalumotAlmashish,
    tozalash: Tozalash,
}

impl Zirh {
    /// `libmobil.so`ni ochadi. Kutubxona APK ichida `lib/<abi>/` da yotadi —
    /// uni o'sha yerga `Cargo.toml`dagi `runtime_libs` qo'yadi.
    pub fn load() -> Result<Self, libloading::Error> {
        unsafe {
            let lib = libloading::Library::new("libmobil.so")?;
            let olish = *lib.get::<MalumotOlish>(b"flutter_malumot_olish\0")?;
            let almashish = *lib.get::<MalumotAlmashish>(b"flutter_malumot_almashish\0")?;
            let tozalash = *lib.get::<Tozalash>(b"flutter_xotirani_tozalash\0")?;

            Ok(Self { lib, olish, almashish, tozalash })
        }
    }

    /// SDK qaytargan C satrini Rust satriga o'girib, xotirani SDK'ning o'z
    /// funksiyasi bilan bo'shatadi — `free` bilan emas, chunki uni ajratgan
    /// allokator boshqa.
    unsafe fn take_string(&self, raw: *mut c_char) -> Option<String> {
        if raw.is_null() {
            return None;
        }
        let value = unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned();
        unsafe { (self.tozalash)(raw) };
        if value.is_empty() { None } else { Some(value) }
    }

    /// Shifrlangan configdan qiymat o'qish. Yo'l nuqta bilan ajratiladi,
    /// masalan `"server.base_url"`.
    pub fn config(&self, path: &str) -> Option<String> {
        let path = CString::new(path).ok()?;
        unsafe { self.take_string((self.olish)(path.as_ptr())) }
    }

    /// SDK'ning himoyalangan HTTP mijozi — sertifikat pinning bilan.
    /// `headers` JSON obyekt satri bo'lishi kerak.
    #[allow(dead_code)]
    pub fn request(
        &self,
        url: &str,
        method: &str,
        body: Option<&str>,
        headers: Option<&str>,
    ) -> Option<String> {
        let url = CString::new(url).ok()?;
        let method = CString::new(method).ok()?;
        let body = body.and_then(|s| CString::new(s).ok());
        let headers = headers.and_then(|s| CString::new(s).ok());

        let as_ptr = |s: &Option<CString>| s.as_ref().map_or(std::ptr::null(), |v| v.as_ptr());

        unsafe {
            let raw = (self.almashish)(
                url.as_ptr(),
                method.as_ptr(),
                as_ptr(&body),
                as_ptr(&headers),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
            );
            self.take_string(raw)
        }
    }

    /// SDK'ga JavaVM'ni beradi va shu bilan uning xavfsizlik tekshiruvlarini
    /// yoqadi.
    ///
    /// **Ogohlantirish:** tekshiruv o'tmasa SDK jarayonni ataylab o'ldiradi.
    /// Buni faqat Play Store uchun imzolangan release build'da chaqiring.
    #[allow(dead_code)]
    pub fn attach_vm(&self) -> bool {
        let vm = ndk_context::android_context().vm();
        if vm.is_null() {
            return false;
        }
        unsafe {
            match self.lib.get::<JniOnLoad>(b"JNI_OnLoad\0") {
                Ok(jni_on_load) => {
                    jni_on_load(vm, std::ptr::null_mut());
                    true
                }
                Err(_) => false,
            }
        }
    }
}

/// Ishga tushishda bir marta chaqiriladi. Xatolik bo'lsa ilova baribir
/// ishlayveradi — SDK yo'qligi UI uchun halokatli emas.
pub fn init() -> Option<Zirh> {
    match Zirh::load() {
        Ok(zirh) => {
            logcat("zirh: libmobil.so yuklandi (passiv rejim, JavaVM ulanmagan)");
            Some(zirh)
        }
        Err(err) => {
            logcat(&format!("zirh: yuklab bo'lmadi — {err}"));
            None
        }
    }
}
