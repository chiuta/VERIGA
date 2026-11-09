//! Citirea minimă a certificatului frunză X.509 (DER, RFC 5280): interval de valabilitate și
//! nume alternative (SAN), pentru avertizări de expirare și nepotrivire de nume.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Zile în care certificatul este considerat «aproape de expirare», implicit.
pub const DEFAULT_WARN_DAYS: u32 = 14;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertInfo {
    /// Secunde Unix (UTC).
    pub not_before: i64,
    pub not_after: i64,
    pub dns: Vec<String>,
    pub ips: Vec<Vec<u8>>,
}

/// Un element DER: (tag, conținut, restul).
fn tlv(b: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let tag = *b.first()?;
    let l0 = *b.get(1)?;
    let (len, off) = if l0 < 0x80 {
        (l0 as usize, 2)
    } else {
        let n = (l0 & 0x7F) as usize;
        if n == 0 || n > 4 {
            return None;
        }
        let mut len = 0usize;
        for i in 0..n {
            len = (len << 8) | *b.get(2 + i)? as usize;
        }
        (len, 2 + n)
    };
    let end = off.checked_add(len)?;
    Some((tag, b.get(off..end)?, b.get(end..)?))
}

/// `UTCTime` (anii 50–99 → 19xx, 00–49 → 20xx) sau `GeneralizedTime`, în secunde Unix.
pub fn parse_time(tag: u8, s: &str) -> Option<i64> {
    let digits = |r: std::ops::Range<usize>| -> Option<u32> {
        let t = s.get(r)?;
        if t.chars().all(|c| c.is_ascii_digit()) {
            t.parse().ok()
        } else {
            None
        }
    };
    let (year, rest) = match tag {
        0x17 => {
            if s.len() != 13 || !s.ends_with('Z') {
                return None;
            }
            let yy = digits(0..2)? as i32;
            (if yy >= 50 { 1900 + yy } else { 2000 + yy }, 2)
        }
        0x18 => {
            if s.len() != 15 || !s.ends_with('Z') {
                return None;
            }
            (digits(0..4)? as i32, 4)
        }
        _ => return None,
    };
    let (mo, d, h, mi, sec) = (
        digits(rest..rest + 2)?,
        digits(rest + 2..rest + 4)?,
        digits(rest + 4..rest + 6)?,
        digits(rest + 6..rest + 8)?,
        digits(rest + 8..rest + 10)?,
    );
    Some(
        NaiveDate::from_ymd_opt(year, mo, d)?
            .and_hms_opt(h, mi, sec)?
            .and_utc()
            .timestamp(),
    )
}

fn time_of(b: &[u8]) -> Option<(i64, &[u8])> {
    let (tag, c, rest) = tlv(b)?;
    Some((parse_time(tag, std::str::from_utf8(c).ok()?)?, rest))
}

/// Citește certificatul frunză (DER). `None` dacă structura nu poate fi citită.
pub fn parse(der: &[u8]) -> Option<CertInfo> {
    let (t, cert, _) = tlv(der)?;
    if t != 0x30 {
        return None;
    }
    let (t, tbs, _) = tlv(cert)?;
    if t != 0x30 {
        return None;
    }
    let mut rest = tbs;
    let (mut tag, mut c, mut r) = tlv(rest)?;
    if tag == 0xA0 {
        // versiune
        rest = r;
        (tag, c, r) = tlv(rest)?;
    }
    let _ = (tag, c); // serial
    rest = r;
    for _ in 0..2 {
        // algoritm de semnare, emitent
        let (_, _, r) = tlv(rest)?;
        rest = r;
    }
    let (tag, validity, r) = tlv(rest)?;
    if tag != 0x30 {
        return None;
    }
    let (not_before, v2) = time_of(validity)?;
    let (not_after, _) = time_of(v2)?;
    rest = r;
    // subiect, cheie publică
    for _ in 0..2 {
        let (_, _, r) = tlv(rest)?;
        rest = r;
    }
    let mut info = CertInfo {
        not_before,
        not_after,
        dns: vec![],
        ips: vec![],
    };
    // câmpuri opționale: [1], [2], [3] extensii
    while let Some((tag, c, r)) = tlv(rest) {
        rest = r;
        if tag != 0xA3 {
            continue;
        }
        let (_, exts, _) = tlv(c)?;
        let mut e = exts;
        while let Some((_, ext, r)) = tlv(e) {
            e = r;
            let (otag, oid, mut er) = tlv(ext)?;
            if otag != 0x06 || oid != [0x55, 0x1D, 0x11] {
                continue;
            }
            // BOOLEAN critical opțional
            let (mut t2, mut v, mut after) = tlv(er)?;
            if t2 == 0x01 {
                er = after;
                (t2, v, after) = tlv(er)?;
            }
            let _ = after;
            if t2 != 0x04 {
                continue;
            }
            let (_, names, _) = tlv(v)?;
            let mut n = names;
            while let Some((nt, nv, r)) = tlv(n) {
                n = r;
                match nt {
                    0x82 => {
                        if let Ok(s) = std::str::from_utf8(nv) {
                            info.dns.push(s.to_ascii_lowercase());
                        }
                    }
                    0x87 => info.ips.push(nv.to_vec()),
                    _ => {}
                }
            }
        }
    }
    Some(info)
}

/// Potrivirea numelui de gazdă cu SAN (RFC 6125: `*` doar ca etichetă stângă întreagă).
pub fn name_matches(host: &str, info: &CertInfo) -> bool {
    let host = host.trim_matches(['[', ']']);
    if let Ok(ip) = host.parse::<IpAddr>() {
        let bytes = match ip {
            IpAddr::V4(a) => a.octets().to_vec(),
            IpAddr::V6(a) => a.octets().to_vec(),
        };
        return info.ips.contains(&bytes);
    }
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    info.dns.iter().any(|p| {
        if let Some(suffix) = p.strip_prefix("*.") {
            match host.split_once('.') {
                Some((first, rest)) => !first.is_empty() && rest == suffix,
                None => false,
            }
        } else {
            *p == host
        }
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CertKind {
    Expired,
    NotYetValid,
    /// Expiră în `days` zile.
    ExpiresSoon,
    NameMismatch,
    Unreadable,
}

impl CertKind {
    pub fn label(self, lang: &str) -> &'static str {
        match (self, lang == "en") {
            (Self::Expired, false) => "expirat",
            (Self::Expired, true) => "expired",
            (Self::NotYetValid, false) => "încă nevalid",
            (Self::NotYetValid, true) => "not yet valid",
            (Self::ExpiresSoon, false) => "expiră în curând",
            (Self::ExpiresSoon, true) => "expires soon",
            (Self::NameMismatch, false) => "nume nepotrivit",
            (Self::NameMismatch, true) => "name mismatch",
            (Self::Unreadable, false) => "certificat ilizibil",
            (Self::Unreadable, true) => "unreadable certificate",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CertFinding {
    pub host_port: String,
    pub kind: CertKind,
    /// Zilele rămase (doar pentru `ExpiresSoon`).
    pub days: Option<i64>,
    /// `notAfter` ca RFC 3339, dacă certificatul a fost citit.
    pub not_after: Option<String>,
    /// Numărul de legături verificate pe acest host:port.
    pub links: usize,
}

/// Starea de valabilitate în momentul `now` (secunde Unix); pragul 0 raportează doar «expirat».
pub fn validity_status(
    info: &CertInfo,
    now: i64,
    warn_days: u32,
) -> Option<(CertKind, Option<i64>)> {
    if now > info.not_after {
        return Some((CertKind::Expired, None));
    }
    if now < info.not_before {
        return Some((CertKind::NotYetValid, None));
    }
    let days = (info.not_after - now) / 86_400;
    if warn_days > 0 && days <= warn_days as i64 {
        return Some((CertKind::ExpiresSoon, Some(days)));
    }
    None
}

fn rfc3339(secs: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(secs, 0).map(|d| d.to_rfc3339())
}

/// Constatările pentru un host:port: starea de valabilitate și, separat, numele.
pub fn findings_for(
    host_port: &str,
    host: &str,
    der: Option<&[u8]>,
    now: i64,
    warn_days: u32,
    links: usize,
) -> Vec<CertFinding> {
    let mk = |kind, days, not_after: Option<i64>| CertFinding {
        host_port: host_port.to_string(),
        kind,
        days,
        not_after: not_after.and_then(rfc3339),
        links,
    };
    let Some(info) = der.and_then(parse) else {
        return vec![mk(CertKind::Unreadable, None, None)];
    };
    let mut out = vec![];
    if let Some((k, d)) = validity_status(&info, now, warn_days) {
        out.push(mk(k, d, Some(info.not_after)));
    }
    if !name_matches(host, &info) {
        out.push(mk(CertKind::NameMismatch, None, Some(info.not_after)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc_len(n: usize) -> Vec<u8> {
        if n < 0x80 {
            vec![n as u8]
        } else if n < 0x100 {
            vec![0x81, n as u8]
        } else {
            vec![0x82, (n >> 8) as u8, n as u8]
        }
    }
    fn t(tag: u8, c: &[u8]) -> Vec<u8> {
        let mut o = vec![tag];
        o.extend(enc_len(c.len()));
        o.extend_from_slice(c);
        o
    }
    fn cat(parts: &[Vec<u8>]) -> Vec<u8> {
        parts.concat()
    }

    /// Certificat de probă (semnătura nu contează pentru analiza câmpurilor).
    pub fn cert(nb: (u8, &str), na: (u8, &str), dns: &[&str], ips: &[[u8; 4]]) -> Vec<u8> {
        let mut gn = vec![];
        for d in dns {
            gn.push(t(0x82, d.as_bytes()));
        }
        for i in ips {
            gn.push(t(0x87, i));
        }
        let san = t(
            0x30,
            &cat(&[t(0x06, &[0x55, 0x1D, 0x11]), t(0x04, &t(0x30, &cat(&gn)))]),
        );
        let exts = t(0xA3, &t(0x30, &san));
        let tbs = t(
            0x30,
            &cat(&[
                t(0xA0, &t(0x02, &[2])),
                t(0x02, &[1]),
                t(0x30, &t(0x06, &[0x2A, 0x03])),
                t(0x30, &[]),
                t(
                    0x30,
                    &cat(&[t(nb.0, nb.1.as_bytes()), t(na.0, na.1.as_bytes())]),
                ),
                t(0x30, &[]),
                t(0x30, &[]),
                exts,
            ]),
        );
        t(0x30, &cat(&[tbs, t(0x30, &[]), t(0x03, &[0])]))
    }

    fn ts(y: i32, m: u32, d: u32) -> i64 {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    }

    const NOW: i64 = 1_800_000_000; // 2027-01-15

    fn der_valid(na_off_days: i64, nb_off_days: i64) -> Vec<u8> {
        let f = |off: i64| {
            chrono::DateTime::from_timestamp(NOW + off * 86_400, 0)
                .unwrap()
                .format("%Y%m%d%H%M%SZ")
                .to_string()
        };
        cert(
            (0x18, &f(nb_off_days)),
            (0x18, &f(na_off_days)),
            &["ex.test"],
            &[],
        )
    }

    #[test]
    fn t1_expired_yesterday() {
        let i = parse(&der_valid(-1, -90)).unwrap();
        assert_eq!(
            validity_status(&i, NOW, 14),
            Some((CertKind::Expired, None))
        );
    }

    #[test]
    fn t2_threshold() {
        let i = parse(&der_valid(10, -80)).unwrap();
        assert_eq!(
            validity_status(&i, NOW, 14),
            Some((CertKind::ExpiresSoon, Some(10)))
        );
        assert_eq!(validity_status(&i, NOW, 5), None);
    }

    #[test]
    fn t3_not_yet_valid() {
        let i = parse(&der_valid(90, 1)).unwrap();
        assert_eq!(
            validity_status(&i, NOW, 14),
            Some((CertKind::NotYetValid, None))
        );
    }

    #[test]
    fn t5_generalized_time_2051() {
        assert_eq!(parse_time(0x18, "20510101000000Z"), Some(ts(2051, 1, 1)));
    }

    #[test]
    fn t6_utc_time_pivot() {
        assert_eq!(parse_time(0x17, "490101000000Z"), Some(ts(2049, 1, 1)));
        assert_eq!(parse_time(0x17, "500101000000Z"), Some(ts(1950, 1, 1)));
    }

    #[test]
    fn t7_valid_90_days() {
        let i = parse(&der_valid(90, -1)).unwrap();
        assert_eq!(validity_status(&i, NOW, 14), None);
    }

    #[test]
    fn threshold_zero_only_expired() {
        let i = parse(&der_valid(0, -80)).unwrap();
        assert_eq!(validity_status(&i, NOW, 0), None);
        let e = parse(&der_valid(-1, -80)).unwrap();
        assert_eq!(validity_status(&e, NOW, 0), Some((CertKind::Expired, None)));
    }

    #[test]
    fn utc_and_generalized_in_the_same_cert() {
        let c = cert(
            (0x17, "250101000000Z"),
            (0x18, "20510101000000Z"),
            &["a.test"],
            &[],
        );
        let i = parse(&c).unwrap();
        assert_eq!(i.not_before, ts(2025, 1, 1));
        assert_eq!(i.not_after, ts(2051, 1, 1));
    }

    #[test]
    fn san_name_matching() {
        let c = cert(
            (0x17, "250101000000Z"),
            (0x18, "20510101000000Z"),
            &["ex.test", "*.wild.test"],
            &[[127, 0, 0, 1]],
        );
        let i = parse(&c).unwrap();
        assert!(name_matches("EX.test", &i));
        assert!(name_matches("a.wild.test", &i));
        assert!(!name_matches("wild.test", &i));
        assert!(!name_matches("a.b.wild.test", &i));
        assert!(!name_matches("other.test", &i));
        assert!(name_matches("127.0.0.1", &i));
        assert!(!name_matches("127.0.0.2", &i));
    }

    #[test]
    fn findings_name_mismatch_is_separate() {
        let f = findings_for("h:443", "other.test", Some(&der_valid(5, -1)), NOW, 14, 3);
        let kinds: Vec<_> = f.iter().map(|x| x.kind).collect();
        assert_eq!(kinds, vec![CertKind::ExpiresSoon, CertKind::NameMismatch]);
        assert!(f.iter().all(|x| x.links == 3));
    }

    #[test]
    fn unreadable_certificate() {
        let f = findings_for("h:443", "h", Some(b"not der"), NOW, 14, 1);
        assert_eq!(f[0].kind, CertKind::Unreadable);
        let f = findings_for("h:443", "h", None, NOW, 14, 1);
        assert_eq!(f[0].kind, CertKind::Unreadable);
    }

    #[test]
    fn real_fixture_parses() {
        let pem = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/test-cert.pem"
        ))
        .unwrap();
        let b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
        let der = crate::sri::b64_decode(&b64).unwrap();
        let i = parse(&der).unwrap();
        assert!(name_matches("localhost", &i));
        assert!(name_matches("127.0.0.1", &i));
        assert!(i.not_after > i.not_before);
    }
}
