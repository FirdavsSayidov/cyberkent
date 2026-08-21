# Slint Rust Template

A template for a Rust application that's using [Slint](https://slint.rs/) for the user interface.

## About

This template helps you get started developing a Rust application with Slint as toolkit
for the user interface. It demonstrates the integration between the `.slint` UI markup and
Rust code, how to react to callbacks, get and set properties, and use basic widgets.

## Usage

1. Install Rust by following its [getting-started guide](https://www.rust-lang.org/learn/get-started).
   Once this is done, you should have the `rustc` compiler and the `cargo` build system installed in your `PATH`.
2. Download and extract the [ZIP archive of this repository](https://github.com/slint-ui/slint-rust-template/archive/refs/heads/main.zip).
3. Rename the extracted directory and change into it:
    ```
    mv slint-rust-template-main my-project
    cd my-project    
    ```
4. Build with `cargo`:
    ```
    cargo build
    ```
5. Run the application binary:
    ```
    cargo run
    ```

## Sahifalar

Butun ilova sof Slint'da. Rust'da faqat `CyberKentApp::new()?.run()` bor —
tartib, holat, navigatsiya va animatsiyaning hammasi `.slint` ichida.

| Fayl | Nima |
| --- | --- |
| `ui/ck-app.slint` | Qobiq: fon, 5 sahifa, tab paneli. `build.rs` shuni quradi |
| `ui/ck-common.slint` | Dizayn tizimi: tokenlar, ikonkalar, umumiy bo'laklar |
| `ui/ck-home.slint` | 05 — Bosh sahifa |
| `ui/ck-events.slint` | 06 — Tadbirlar |
| `ui/ck-wallet.slint` | 11 — Hamyon |
| `ui/ck-shop.slint` | 12 — Do'kon |
| `ui/ck-profile.slint` | 15 — Profil |

Navigatsiya: barcha tab sahifasi bir marta yaratiladi va `visible` bilan
almashadi, shuning uchun tab almashganda skroll holati, qidiruv matni va
tanlangan filtr saqlanib qoladi.

Android'da oyna butun ekranga chiziladi, shuning uchun `Ck` globalida
`safe-top` va `safe-bottom` bor — status bar va navigatsiya paneli balandligi.
Boshqa telefonda chegara boshqacha bo'lsa, shu ikki sonni o'zgartirish kifoya.

Qolgan `.slint` fayllar (`app-window.slint`, `todo-row.slint`, `pages.slint`)
Slint shablonidan qolgan namunalar — kompilyatsiyaga kirmaydi.

## ZirhMobil SDK

`zirh/jni/arm64-v8a/libmobil.so` — `zirhlib-release-v2.0.3.aar` ichidan olingan
mahalliy kutubxona. `Cargo.toml`dagi `runtime_libs = "zirh/jni"` uni APK'ning
`lib/<abi>/` katalogiga qo'yadi, `src/zirh.rs` esa `dlopen` bilan ochadi.

AAR'ning Java sinflari (`uz.zirh.zirhlib`) ishlatilmaydi — ularni APK'ga
qo'shish Gradle talab qiladi, cargo-apk esa dex yasay olmaydi. O'rniga SDK'ning
JNI'siz C interfeysi olinadi: `flutter_malumot_olish`,
`flutter_malumot_almashish`, `flutter_xotirani_tozalash`.

**Hozir passiv rejimda.** SDK'ning root / frida / emulyator / VPN / imzo /
Play Market tekshiruvlari faqat JavaVM berilganda ishga tushadi, biz esa uni
bermayapmiz. Sababi: tekshiruv o'tmasa SDK jarayonni ataylab `SIGSEGV` bilan
o'ldiradi, debug kalit bilan imzolangan va `adb install` bilan o'rnatilgan APK
esa imzo va Play Market tekshiruvidan o'ta olmaydi.

Release'da yoqish uchun `Zirh::attach_vm()`ni chaqiring va SDK configini
qo'shing (configsiz tekshiruvlar baribir o'tkazib yuboriladi).

## Android qurilmada ishga tushirish

Kirish nuqtasi: `src/lib.rs`dagi `android_main`. Desktop `main` ham, Android ham
bir xil `run()`ni chaqiradi, shuning uchun UI kodi bitta nusxada qoladi.

Bir marta o'rnatiladigan narsalar:

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk
```

Muhit o'zgaruvchilari (`~/.zshrc`ga qo'shib qo'ysa bo'ladi):

```sh
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/28.2.13676358"
```

Telefonni USB bilan ulang, "Developer options → USB debugging"ni yoqing va
`adb devices` ro'yxatda `device` deb ko'rsatishiga ishonch hosil qiling. Keyin:

```sh
cargo apk run --release --target aarch64-linux-android --lib
```

**`--release` shart.** Debug build sezilarli sekin: bir xil skroll sinovida
debug 149, release 90 jiffy CPU sarfladi (−40%), cold start esa 358 ms.
Qurilmada sinaganda doim release ishlating.

Bu APK'ni quradi, debug kalit bilan imzolaydi, o'rnatadi va ochadi
(`cargo apk build ...` — faqat qurish uchun; APK
`target/debug/apk/slint-app.apk`da yotadi). Xatolarni ko'rish uchun:
`adb logcat | grep -i slint`.

Paket nomi, ilova nomi va SDK versiyalari `Cargo.toml`dagi
`[package.metadata.android]` bo'limida.

We recommend using an IDE for development, along with our [LSP-based IDE integration for `.slint` files](https://github.com/slint-ui/slint/blob/master/tools/lsp/README.md). You can also load this project directly in [Visual Studio Code](https://code.visualstudio.com) and install our [Slint extension](https://marketplace.visualstudio.com/items?itemName=Slint.slint).

## Next Steps

We hope that this template helps you get started, and that you enjoy exploring making user interfaces with Slint. To learn more
about the Slint APIs and the `.slint` markup language, check out our [online documentation](https://slint.dev/docs).

Don't forget to edit this readme to replace it by yours, and edit the `name =` field in `Cargo.toml` to match the name of your
project.
