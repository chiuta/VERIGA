//! Limba interfeței în linie de comandă: română sau engleză.
//! Ordinea: `--lang`, apoi `VERIGA_LANG`, apoi `LC_ALL` / `LC_MESSAGES` / `LANG`
//! (`ro…` ⇒ română, orice altceva, inclusiv `C`/`POSIX` ⇒ engleză); fără nicio variabilă ⇒ română.

use std::sync::atomic::{AtomicU8, Ordering};

static LANG: AtomicU8 = AtomicU8::new(0); // 0 = nedecis, 1 = ro, 2 = en

/// Alege limba din valori de mediu, în ordinea de precedență dată.
pub fn detect_from<'a>(vals: impl IntoIterator<Item = Option<&'a str>>) -> &'static str {
    for v in vals.into_iter().flatten() {
        let v = v.trim();
        if v.is_empty() {
            continue;
        }
        return if v.to_ascii_lowercase().starts_with("ro") {
            "ro"
        } else {
            "en"
        };
    }
    "ro"
}

pub fn detect() -> &'static str {
    let g = |k: &str| std::env::var(k).ok();
    let (a, b, c, d) = (g("VERIGA_LANG"), g("LC_ALL"), g("LC_MESSAGES"), g("LANG"));
    detect_from([a.as_deref(), b.as_deref(), c.as_deref(), d.as_deref()])
}

/// Fixează limba (`"en"` ⇒ engleză, orice altceva ⇒ română).
pub fn set(l: &str) {
    LANG.store(
        if l.eq_ignore_ascii_case("en") { 2 } else { 1 },
        Ordering::Relaxed,
    );
}

pub fn is_en() -> bool {
    if LANG.load(Ordering::Relaxed) == 0 {
        set(detect());
    }
    LANG.load(Ordering::Relaxed) == 2
}

pub fn code() -> &'static str {
    if is_en() { "en" } else { "ro" }
}

/// Alege textul în limba curentă.
pub fn tr<'a>(ro: &'a str, en: &'a str) -> &'a str {
    if is_en() { en } else { ro }
}

/// `format!` în limba curentă: `trf!("ro {x}", "en {x}", args…)`.
#[macro_export]
macro_rules! trf {
    ($ro:literal, $en:literal $(, $a:expr)* $(,)?) => {
        if $crate::i18n::is_en() { format!($en $(, $a)*) } else { format!($ro $(, $a)*) }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_order_and_defaults() {
        assert_eq!(detect_from([None, None]), "ro");
        assert_eq!(detect_from([Some("ro_RO.UTF-8")]), "ro");
        assert_eq!(detect_from([Some("en_US.UTF-8"), Some("ro_RO")]), "en");
        assert_eq!(detect_from([Some(""), Some("ro_RO")]), "ro");
        assert_eq!(detect_from([Some("C")]), "en");
        assert_eq!(detect_from([Some("POSIX")]), "en");
    }
}
