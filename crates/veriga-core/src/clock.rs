//! Ceasul: momentul curent și formatele de dată/oră (RFC 3339, RFC 2822/HTTP) într-un singur loc.
//!
//! Construit pe `jiff`; înlocuiește `chrono`, declarat nemenținut (RUSTSEC-2026-0543). Formatele
//! produse sunt cele de până acum — RFC 3339 cu `+00:00` sau decalajul local, RFC 2822 cu `+0000` —
//! ca sesiunile, jurnalul de reparații și istoricul salvate de versiunile anterioare să rămână
//! citibile, iar cele noi să poată fi citite de versiunile vechi.
//!
//! Nicio funcție de aici nu intră în panică: o dată în afara intervalului sau un text care nu e o
//! dată dă `None`, niciodată o excepție (înainte, `chrono` dădea panic la `--days 4294967295`).

use jiff::{SignedDuration, Timestamp, ZonedDateTime, civil::DateTime, fmt::rfc2822, tz::TimeZone};
use std::time::SystemTime;

/// Fracțiunea de secundă ca în RFC 3339 «automat»: nimic dacă e zero, altfel 3, 6 sau 9 cifre.
fn frac(ns: i32) -> &'static str {
    if ns == 0 {
        ""
    } else if ns % 1_000_000 == 0 {
        "%.3f"
    } else if ns % 1_000 == 0 {
        "%.6f"
    } else {
        "%.9f"
    }
}

/// Momentul curent, în secunde Unix.
pub fn now_unix_secs() -> i64 {
    Timestamp::now().as_second()
}

/// `2026-11-18T22:10:03.123Z` — UTC, milisecunde, sufix `Z` (obiectele FFF).
pub fn now_utc_millis_z() -> String {
    Timestamp::now()
        .strftime("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

/// RFC 3339 în UTC cu `+00:00`, de ex. `2026-11-18T22:10:03.123+00:00`.
pub fn rfc3339_utc(ts: Timestamp) -> String {
    ts.strftime(&format!(
        "%Y-%m-%dT%H:%M:%S{}+00:00",
        frac(ts.subsec_nanosecond())
    ))
    .to_string()
}

/// [`rfc3339_utc`] pentru momentul curent.
pub fn now_rfc3339_utc() -> String {
    rfc3339_utc(Timestamp::now())
}

/// RFC 3339 cu decalajul fusului local, de ex. `2026-11-18T23:10:03.123+02:00`; dacă fusul local nu
/// poate fi aflat, UTC (`+00:00`).
pub fn now_rfc3339_local() -> String {
    let z = ZonedDateTime::now();
    z.strftime(&format!(
        "%Y-%m-%dT%H:%M:%S{}%:z",
        frac(z.subsec_nanosecond())
    ))
    .to_string()
}

/// `20261118-221003.123` (fus local): baza identificatorului de rulare, sortabilă cronologic.
pub fn local_stamp_millis() -> String {
    ZonedDateTime::now()
        .strftime("%Y%m%d-%H%M%S%.3f")
        .to_string()
}

/// Citește un RFC 3339 (decalaj `Z` sau `±HH:MM` obligatoriu); `None` dacă nu e valid.
pub fn parse_rfc3339(s: &str) -> Option<Timestamp> {
    s.trim().parse().ok()
}

/// Secunde scurse între două RFC 3339 (0 dacă vreunul nu se poate citi sau ordinea e inversă).
pub fn elapsed_secs(started: &str, finished: &str) -> f64 {
    match (parse_rfc3339(started), parse_rfc3339(finished)) {
        (Some(a), Some(b)) => (b - a).as_millis().max(0) as f64 / 1000.0,
        _ => 0.0,
    }
}

/// Începutul perioadei «ultimele `days` zile»; `None` dacă ar ieși din intervalul reprezentabil
/// (atunci perioada acoperă tot istoricul).
pub fn days_ago(days: u32) -> Option<Timestamp> {
    Timestamp::now()
        .checked_sub(SignedDuration::from_hours(i64::from(days) * 24))
        .ok()
}

/// Secunde Unix pentru o dată civilă UTC; `None` pentru date inexistente (30 februarie, ora 24,
/// secunda 60…). Merge pe tot intervalul anilor −9999…9999, deci și pentru «fără expirare» din
/// certificate (`99991231235959Z`), care nu încape într-un `Timestamp`.
pub fn epoch_secs_utc(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> Option<i64> {
    let dt = DateTime::new(
        i16::try_from(y).ok()?,
        i8::try_from(mo).ok()?,
        i8::try_from(d).ok()?,
        i8::try_from(h).ok()?,
        i8::try_from(mi).ok()?,
        i8::try_from(s).ok()?,
        0,
    )
    .ok()?;
    Some(
        dt.duration_since(DateTime::constant(1970, 1, 1, 0, 0, 0, 0))
            .as_secs(),
    )
}

/// RFC 3339 în UTC (`+00:00`) pentru secunde Unix, în anii 0000…9999; altfel `None`.
pub fn rfc3339_from_secs(secs: i64) -> Option<String> {
    let dt = DateTime::constant(1970, 1, 1, 0, 0, 0, 0)
        .checked_add(SignedDuration::from_secs(secs))
        .ok()?;
    if dt.year() < 0 {
        return None;
    }
    Some(dt.strftime("%Y-%m-%dT%H:%M:%S+00:00").to_string())
}

/// RFC 2822 în UTC (`Wed, 18 Nov 2026 22:10:03 +0000`), forma pentru `last_modified`.
pub fn rfc2822_utc(ts: Timestamp) -> Option<String> {
    rfc2822::to_string(&ts.to_zoned_datetime(TimeZone::UTC)).ok()
}

/// [`rfc2822_utc`] pentru data de modificare a unui fișier; `None` dacă data e în afara intervalului.
pub fn http_date_from_system_time(t: SystemTime) -> Option<String> {
    rfc2822_utc(Timestamp::try_from(t).ok()?)
}

/// Citește data dintr-un antet HTTP (IMF-fixdate din RFC 9110 = RFC 2822: `Last-Modified`, `Retry-After`).
pub fn parse_http_date(s: &str) -> Option<Timestamp> {
    rfc2822::parse(s.trim()).ok().map(|z| z.timestamp())
}

/// Data `AAAA-LL-ZZ` dintr-un antet HTTP, în fusul scris în antet (pentru `<lastmod>`).
pub fn http_date_ymd(s: &str) -> Option<String> {
    rfc2822::parse(s.trim())
        .ok()
        .map(|z| z.strftime("%Y-%m-%d").to_string())
}

/// Milisecunde până la data din antet (`Retry-After: <dată>`); 0 dacă e în trecut, `None` dacă nu e dată.
pub fn millis_until_http_date(s: &str) -> Option<u64> {
    let when = parse_http_date(s)?;
    let ms = (when - Timestamp::now()).as_millis().max(0);
    Some(u64::try_from(ms).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    #[test]
    fn rfc3339_keeps_the_old_shape() {
        assert_eq!(
            rfc3339_utc(ts("2026-11-18T22:10:03Z")),
            "2026-11-18T22:10:03+00:00"
        );
        assert_eq!(
            rfc3339_utc(ts("2026-11-18T22:10:03.5Z")),
            "2026-11-18T22:10:03.500+00:00"
        );
        assert_eq!(
            rfc3339_utc(ts("2026-11-18T22:10:03.123456Z")),
            "2026-11-18T22:10:03.123456+00:00"
        );
        assert_eq!(
            rfc3339_utc(ts("2026-11-18T22:10:03.123456789Z")),
            "2026-11-18T22:10:03.123456789+00:00"
        );
    }

    #[test]
    fn rfc3339_round_trips_and_reads_offsets() {
        for s in [
            "2026-11-18T22:10:03Z",
            "1970-01-01T00:00:00Z",
            "2026-11-18T22:10:03.123456789Z",
        ] {
            let t = ts(s);
            assert_eq!(parse_rfc3339(&rfc3339_utc(t)), Some(t));
        }
        // formele scrise de versiunile cu chrono (decalaj local, +00:00) rămân citibile
        assert_eq!(
            parse_rfc3339("2026-11-18T22:10:03+02:00"),
            Some(ts("2026-11-18T20:10:03Z"))
        );
        assert_eq!(
            parse_rfc3339("2026-11-18T22:10:03.123456789+00:00"),
            Some(ts("2026-11-18T22:10:03.123456789Z"))
        );
        assert_eq!(
            parse_rfc3339("  2026-11-18T22:10:03Z "),
            Some(ts("2026-11-18T22:10:03Z"))
        );
        for bad in [
            "",
            "ieri",
            "2026-11-18",
            "2026-11-18T22:10:03",
            "2026-13-18T22:10:03Z",
            "2026-11-18T25:10:03Z",
        ] {
            assert_eq!(parse_rfc3339(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn now_strings_have_the_documented_shape() {
        let z = now_utc_millis_z();
        assert!(
            z.ends_with('Z') && z.len() == 24 && &z[19..20] == ".",
            "{z}"
        );
        let l = now_rfc3339_local();
        let back = parse_rfc3339(&l).unwrap_or_else(|| panic!("nu se citește înapoi: {l}"));
        assert!((back - Timestamp::now()).as_secs().abs() < 5, "{l}");
        let u = now_rfc3339_utc();
        assert!(u.ends_with("+00:00"), "{u}");
        let st = local_stamp_millis();
        let b = st.as_bytes();
        assert!(st.len() == 19 && b[8] == b'-' && b[15] == b'.', "{st}");
        assert!(
            st.bytes()
                .enumerate()
                .all(|(i, c)| matches!(i, 8 | 15) || c.is_ascii_digit()),
            "{st}"
        );
    }

    #[test]
    fn epoch_seconds_validate_the_calendar() {
        assert_eq!(epoch_secs_utc(1970, 1, 1, 0, 0, 1), Some(1));
        assert_eq!(epoch_secs_utc(2027, 1, 15, 8, 0, 0), Some(1_800_000_000));
        assert_eq!(epoch_secs_utc(1969, 12, 31, 23, 59, 59), Some(-1));
        assert_eq!(epoch_secs_utc(2024, 2, 29, 0, 0, 0), Some(1_709_164_800)); // an bisect
        assert_eq!(epoch_secs_utc(2023, 2, 29, 0, 0, 0), None);
        assert_eq!(epoch_secs_utc(1900, 2, 29, 0, 0, 0), None); // 1900 nu e bisect
        assert_eq!(epoch_secs_utc(2000, 2, 29, 0, 0, 0), Some(951_782_400)); // 2000 e bisect
        assert_eq!(epoch_secs_utc(2026, 4, 31, 0, 0, 0), None);
        assert_eq!(epoch_secs_utc(2026, 13, 1, 0, 0, 0), None);
        assert_eq!(epoch_secs_utc(2026, 0, 1, 0, 0, 0), None);
        assert_eq!(epoch_secs_utc(2026, 1, 0, 0, 0, 0), None);
        assert_eq!(epoch_secs_utc(2026, 1, 1, 24, 0, 0), None);
        assert_eq!(epoch_secs_utc(2026, 1, 1, 0, 60, 0), None);
        assert_eq!(epoch_secs_utc(2026, 1, 1, 0, 0, 60), None); // fără secunde intercalare
        assert_eq!(epoch_secs_utc(2026, u32::MAX, 1, 0, 0, 0), None);
        assert_eq!(epoch_secs_utc(i32::MAX, 1, 1, 0, 0, 0), None);
        // «fără expirare» din certificate (RFC 5280 §4.1.2.5) nu încape într-un Timestamp, dar trebuie citit
        assert_eq!(
            epoch_secs_utc(9999, 12, 31, 23, 59, 59),
            Some(253_402_300_799)
        );
        assert_eq!(epoch_secs_utc(0, 1, 1, 0, 0, 0), Some(-62_167_219_200));
    }

    #[test]
    fn rfc3339_from_secs_covers_the_whole_certificate_range() {
        assert_eq!(
            rfc3339_from_secs(0).as_deref(),
            Some("1970-01-01T00:00:00+00:00")
        );
        assert_eq!(
            rfc3339_from_secs(1_800_000_000).as_deref(),
            Some("2027-01-15T08:00:00+00:00")
        );
        assert_eq!(
            rfc3339_from_secs(-1).as_deref(),
            Some("1969-12-31T23:59:59+00:00")
        );
        assert_eq!(
            rfc3339_from_secs(253_402_300_799).as_deref(),
            Some("9999-12-31T23:59:59+00:00")
        );
        assert_eq!(
            rfc3339_from_secs(-62_167_219_200).as_deref(),
            Some("0000-01-01T00:00:00+00:00")
        );
        assert_eq!(rfc3339_from_secs(-62_167_219_201), None); // anul −1 nu e RFC 3339
        assert_eq!(rfc3339_from_secs(253_402_300_800), None); // anul 10000
        assert_eq!(rfc3339_from_secs(i64::MAX), None);
        assert_eq!(rfc3339_from_secs(i64::MIN), None);
    }

    #[test]
    fn http_dates() {
        let t = parse_http_date("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();
        assert_eq!(t.as_second(), 784_111_777);
        assert_eq!(
            parse_http_date("Wed, 18 Nov 2026 22:10:03 +0200"),
            Some(ts("2026-11-18T20:10:03Z"))
        );
        assert_eq!(parse_http_date(" Sun, 06 Nov 1994 08:49:37 GMT "), Some(t));
        for bad in [
            "",
            "curând",
            "3",
            "Mon, 06 Nov 1994 08:49:37 GMT",
            "Sun, 31 Nov 1994 08:49:37 GMT",
            "Sun, 06 Nov 1994 25:49:37 GMT",
        ] {
            assert_eq!(parse_http_date(bad), None, "{bad:?}");
        }
        // <lastmod> păstrează ziua din fusul scris în antet (ca înainte)
        assert_eq!(
            http_date_ymd("Wed, 18 Nov 2026 23:30:00 -0500").as_deref(),
            Some("2026-11-18")
        );
        assert_eq!(
            http_date_ymd("Wed, 18 Nov 2026 01:30:00 +0300").as_deref(),
            Some("2026-11-18")
        );
        assert_eq!(http_date_ymd("nu e dată"), None);
        // RFC 2822 în UTC și înapoi
        let s = rfc2822_utc(t).unwrap();
        assert!(s.ends_with("+0000") && s.contains("Nov 1994"), "{s}");
        assert_eq!(parse_http_date(&s), Some(t));
        assert_eq!(
            http_date_from_system_time(SystemTime::UNIX_EPOCH).as_deref(),
            Some("Thu, 1 Jan 1970 00:00:00 +0000") // ca `to_rfc2822()` din chrono: ziua fără zero în față
        );
    }

    #[test]
    fn retry_after_dates() {
        assert_eq!(
            millis_until_http_date("Sun, 06 Nov 1994 08:49:37 GMT"),
            Some(0)
        ); // în trecut
        let fut = rfc2822_utc(
            Timestamp::now()
                .checked_add(SignedDuration::from_secs(30))
                .unwrap(),
        )
        .unwrap();
        let ms = millis_until_http_date(&fut).unwrap();
        assert!((25_000..=31_000).contains(&ms), "{ms}");
        assert_eq!(millis_until_http_date("curând"), None);
        // o dată foarte îndepărtată nu depășește u64 și nu dă panic
        let far = rfc2822_utc(ts("9000-01-01T00:00:00Z")).unwrap();
        assert!(
            millis_until_http_date(&far).unwrap() > 200_000_000_000_000,
            "{far}"
        );
        // ultimele ~2 zile din anul 9999 nu încap într-un Timestamp: antetul e tratat ca nevalid, fără panic
        assert_eq!(
            millis_until_http_date("Fri, 31 Dec 9999 23:59:59 GMT"),
            None
        );
    }

    #[test]
    fn periods_never_panic() {
        let now = Timestamp::now();
        let a = days_ago(30).unwrap();
        assert!(
            ((now - a).as_secs() - 30 * 86_400).abs() <= 5,
            "{:?}",
            now - a
        );
        assert!(days_ago(0).is_some());
        assert!(days_ago(3650).is_some());
        // înainte: `veriga compliance --days 4294967295` intra în panic în chrono
        assert!(days_ago(u32::MAX).is_none());
        assert!(days_ago(u32::MAX / 2).is_none());
    }

    #[test]
    fn elapsed() {
        assert_eq!(
            elapsed_secs("2026-11-18T22:10:03+00:00", "2026-11-18T22:10:05.500+00:00"),
            2.5
        );
        assert_eq!(
            elapsed_secs("2026-11-18T22:10:05Z", "2026-11-18T22:10:03Z"),
            0.0
        ); // ordine inversă
        assert_eq!(elapsed_secs("", "2026-11-18T22:10:03Z"), 0.0);
        assert_eq!(elapsed_secs("x", "y"), 0.0);
    }
}
