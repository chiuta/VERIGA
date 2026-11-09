//! Validarea sintactică a adreselor `tel:`, `geo:` și `data:` (nu se face nicio cerere în rețea).

use url::Url;

/// Peste această dimensiune, o adresă `data:` nu mai este analizată.
pub const MAX_DATA_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Sintaxă validă (cu o eventuală observație).
    Valid(String),
    /// Sintaxă invalidă, cu motivul.
    Invalid(String),
    /// Nu s-a verificat (de pildă, prea mare).
    Unchecked(String),
}

fn invalid<T: Into<String>>(s: T) -> Verdict {
    Verdict::Invalid(s.into())
}

/// Schemele tratate de acest modul.
pub fn handles(scheme: &str) -> bool {
    matches!(scheme, "tel" | "geo" | "data")
}

pub fn validate(u: &Url) -> Option<Verdict> {
    match u.scheme() {
        "tel" => Some(tel(rest_of(u))),
        "geo" => Some(geo(rest_of(u))),
        "data" => Some(data(rest_of(u))),
        _ => None,
    }
}

/// Partea de după `schema:`, fără fragment.
fn rest_of(u: &Url) -> &str {
    let s = u.as_str();
    let rest = s.split_once(':').map(|x| x.1).unwrap_or("");
    rest.split('#').next().unwrap_or("")
}

fn is_visual_sep(c: char) -> bool {
    matches!(c, '-' | '.' | '(' | ')')
}

// ───────────────────────────── tel (RFC 3966) ─────────────────────────────

fn pct_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

fn tel(rest: &str) -> Verdict {
    let decoded = pct_decode(rest);
    if decoded.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return invalid("spațiile sunt interzise în adresele tel:");
    }
    let mut parts = decoded.split(';');
    let number = parts.next().unwrap_or("");
    let params: Vec<&str> = parts.collect();
    let mut context = None;
    for p in &params {
        let (name, val) = p.split_once('=').unwrap_or((p, ""));
        match name.to_ascii_lowercase().as_str() {
            "ext" => {
                if val.is_empty() || !val.chars().all(|c| c.is_ascii_digit()) {
                    return invalid("parametrul ext trebuie să conțină doar cifre");
                }
            }
            "phone-context" => {
                if val.is_empty() {
                    return invalid("phone-context este gol");
                }
                let ok_global = val.strip_prefix('+').is_some_and(|d| {
                    d.chars().any(|c| c.is_ascii_digit())
                        && d.chars().all(|c| c.is_ascii_digit() || is_visual_sep(c))
                });
                let ok_domain = val
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
                    && val
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphanumeric());
                if !(ok_global || ok_domain) {
                    return invalid(
                        "phone-context nu este nici număr global, nici nume de domeniu",
                    );
                }
                context = Some(val);
            }
            _ => {}
        }
    }
    if let Some(g) = number.strip_prefix('+') {
        if !g.chars().any(|c| c.is_ascii_digit()) {
            return invalid("numărul global nu conține nicio cifră");
        }
        if !g.chars().all(|c| c.is_ascii_digit() || is_visual_sep(c)) {
            return invalid("caracter nepermis în numărul global");
        }
        return Verdict::Valid("număr global valid".into());
    }
    if !number
        .chars()
        .any(|c| c.is_ascii_hexdigit() || c == '*' || c == '#')
    {
        return invalid("numărul local nu conține cifre");
    }
    if !number
        .chars()
        .all(|c| c.is_ascii_hexdigit() || c == '*' || c == '#' || is_visual_sep(c))
    {
        return invalid("caracter nepermis în numărul local");
    }
    if context.is_none() {
        return invalid("numărul local cere parametrul phone-context");
    }
    Verdict::Valid("număr local valid".into())
}

// ───────────────────────────── geo (RFC 5870) ─────────────────────────────

fn geo_num(s: &str) -> Option<f64> {
    let t = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (int, frac) = match t.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (t, None),
    };
    if int.is_empty() || !int.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if let Some(f) = frac
        && (f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    s.parse::<f64>().ok()
}

fn geo(rest: &str) -> Verdict {
    let mut parts = rest.split(';');
    let coords: Vec<&str> = parts.next().unwrap_or("").split(',').collect();
    if !(2..=3).contains(&coords.len()) {
        return invalid("se așteaptă latitudine,longitudine[,altitudine]");
    }
    let Some(lat) = geo_num(coords[0]) else {
        return invalid("latitudine nenumerică");
    };
    let Some(lon) = geo_num(coords[1]) else {
        return invalid("longitudine nenumerică");
    };
    if !(-90.0..=90.0).contains(&lat) {
        return invalid("latitudinea trebuie să fie în [-90, 90]");
    }
    if !(-180.0..=180.0).contains(&lon) {
        return invalid("longitudinea trebuie să fie în [-180, 180]");
    }
    if coords.len() == 3 && geo_num(coords[2]).is_none() {
        return invalid("altitudine nenumerică");
    }
    let mut note = String::from("coordonate valide");
    for p in parts {
        let (name, val) = p.split_once('=').unwrap_or((p, ""));
        match name.to_ascii_lowercase().as_str() {
            "crs" => {
                if val.is_empty() {
                    return invalid("crs este gol");
                }
                if !val.eq_ignore_ascii_case("wgs84") {
                    note = format!("coordonate valide (crs={val}, doar informativ)");
                }
            }
            "u" => match geo_num(val) {
                Some(x) if x >= 0.0 => {}
                _ => return invalid("incertitudinea u trebuie să fie un număr nenegativ"),
            },
            _ => {}
        }
    }
    Verdict::Valid(note)
}

// ───────────────────────────── data (RFC 2397 + Fetch) ─────────────────────────────

fn is_token(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c))
}

/// Base64 «îngăduitor»: spațiile se ignoră, umplutura poate lipsi, restul 1 (mod 4) e invalid.
fn forgiving_base64_ok(s: &str) -> bool {
    let mut v: Vec<char> = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if v.len().is_multiple_of(4) {
        for _ in 0..2 {
            if v.last() == Some(&'=') {
                v.pop();
            }
        }
    }
    if v.len() % 4 == 1 {
        return false;
    }
    v.iter()
        .all(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/')
}

fn data(rest: &str) -> Verdict {
    if rest.len() > MAX_DATA_BYTES {
        return Verdict::Unchecked("adresă data: prea mare, nu s-a verificat".into());
    }
    let Some((head, body)) = rest.split_once(',') else {
        return invalid("lipsește virgula dintre antet și date");
    };
    let mut segs: Vec<&str> = head.split(';').map(str::trim).collect();
    let is_b64 = segs.len() > 1
        && segs
            .last()
            .is_some_and(|s| s.eq_ignore_ascii_case("base64"));
    if is_b64 {
        segs.pop();
    }
    let mut note = String::new();
    let mime = segs.first().copied().unwrap_or("");
    if mime.is_empty() {
        note.push_str("tip implicit text/plain;charset=US-ASCII");
    } else {
        let ok = mime
            .split_once('/')
            .is_some_and(|(t, s)| is_token(t) && is_token(s));
        if !ok {
            note.push_str("tip media invalid, se folosește text/plain");
        }
    }
    if is_b64 {
        if !forgiving_base64_ok(&pct_decode(body)) {
            return invalid("date base64 invalide");
        }
    } else {
        let b = body.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' {
                let ok = b.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                    && b.get(i + 2).is_some_and(u8::is_ascii_hexdigit);
                if !ok {
                    return invalid("procent-codare incompletă (se cere %HH)");
                }
                i += 3;
            } else {
                i += 1;
            }
        }
    }
    Verdict::Valid(if note.is_empty() {
        "adresă data: validă".into()
    } else {
        format!("adresă data: validă ({note})")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Verdict {
        validate(&Url::parse(s).unwrap()).unwrap()
    }
    fn ok(s: &str) -> bool {
        matches!(v(s), Verdict::Valid(_))
    }
    fn bad(s: &str) -> bool {
        matches!(v(s), Verdict::Invalid(_))
    }

    #[test]
    fn t1_tel_global() {
        assert!(ok("tel:+1-201-555-0123"));
        assert!(ok("TEL:+1-201-555-0123"));
        assert!(bad("tel:+abc"));
        assert!(bad("tel:+"));
    }

    #[test]
    fn t2_tel_local_needs_context() {
        assert!(bad("tel:5551234"));
        assert!(ok("tel:5551234;phone-context=+40"));
        assert!(ok("tel:5551234;phone-context=example.com"));
        assert!(bad("tel:5551234;phone-context="));
    }

    #[test]
    fn t3_tel_spaces_rejected() {
        assert!(bad("tel:+40 721 000 000"));
        assert!(bad("tel:+40%20721%20000%20000"));
    }

    #[test]
    fn tel_ext() {
        assert!(ok("tel:+40721000000;ext=123"));
        assert!(bad("tel:+40721000000;ext=12a"));
    }

    #[test]
    fn t4_geo() {
        assert!(ok("geo:48.2010,16.3695,183"));
        assert!(bad("geo:91,0"));
        assert!(ok("geo:13.4125,103.8667;u=20"));
        assert!(bad("geo:1,2,3,4"));
        assert!(bad("geo:1e3,2"));
        assert!(bad("geo:10,200"));
        assert!(ok("geo:10,20;crs=wgs84"));
        assert!(ok("geo:10,20;crs=epsg:4326"));
        assert!(bad("geo:10,20;u=-1"));
        assert!(ok("geo:90,0"));
    }

    #[test]
    fn t5_data_comma() {
        assert!(ok("data:,A%20brief%20note"));
        assert!(bad("data:text/plain"));
        assert!(bad("data:,bad%zz"));
    }

    #[test]
    fn t6_t7_base64() {
        assert!(ok("data:text/plain;base64,SGVsbG8="));
        assert!(bad("data:text/plain;base64,S"));
        assert!(ok("data:text/plain;base64,SGVsbG8"));
        assert!(ok("data:text/plain;base64,SGVs bG8="));
        assert!(bad("data:text/plain;base64,SGV$bG8="));
    }

    #[test]
    fn data_invalid_media_type_is_warning_not_error() {
        match v("data:notamime,abc") {
            Verdict::Valid(n) => assert!(n.contains("text/plain")),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn data_too_large_is_unchecked() {
        let big = format!("data:,{}", "a".repeat(MAX_DATA_BYTES + 1));
        assert!(matches!(v(&big), Verdict::Unchecked(_)));
    }

    #[test]
    fn other_schemes_not_handled() {
        assert!(validate(&Url::parse("http://x.org/").unwrap()).is_none());
    }
}
