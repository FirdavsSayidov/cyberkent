// Windows'da release build'da ortiqcha konsol oynasi chiqmasligi uchun.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;

// Desktop kirish nuqtasi. Butun mantiq `src/lib.rs`da — Android ham
// o'sha `run()`ni ishlatadi.
fn main() -> Result<(), Box<dyn Error>> {
    slint_app::run()
}
