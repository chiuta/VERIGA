//! Utilitare pentru URL-uri: normalizare, intrări de start, tipuri de fișiere.

use crate::i18n::tr;
use crate::trf;
use anyhow::{Result, anyhow};
use std::path::Path;
use url::Url;

/// Cheia de deduplicare: URL fără fragment.
pub fn normalize(u: &Url) -> String {
    let mut c = u.clone();
    c.set_fragment(None);
    c.to_string()
}

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

/// Normalizare sintactică a procent-codărilor (RFC 3986 §6.2.2.1–2): hexazecimalele devin
/// majuscule, iar caracterele neprotejate codate inutil sunt decodate. Restul rămâne neatins
/// (de pildă `%2F`, delimitator rezervat).
pub fn normalize_escapes(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && b[i + 1].is_ascii_hexdigit()
            && b[i + 2].is_ascii_hexdigit()
        {
            let v = u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap_or(0);
            if is_unreserved(v) {
                out.push(v as char);
            } else {
                out.push('%');
                out.push_str(&s[i + 1..i + 3].to_ascii_uppercase());
            }
            i += 3;
        } else {
            // copiem un caracter întreg (UTF-8)
            let ch = s[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Aduce adresa la forma canonică folosită pentru a unifica scrierile echivalente.
/// Schema, gazda, portul implicit, segmentele cu puncte și calea goală sunt deja tratate de
/// parser; aici se adaugă normalizarea procent-codărilor în cale și interogare.
pub fn canonicalize(u: &mut Url) {
    if !matches!(u.scheme(), "http" | "https") {
        return;
    }
    let path = normalize_escapes(u.path());
    if path != u.path() {
        u.set_path(&path);
    }
    if let Some(q) = u.query() {
        let nq = normalize_escapes(q);
        if nq != q {
            u.set_query(Some(&nq));
        }
    }
}

/// Forma canonică (fără fragment) a unei adrese http/https scrise de autor, sau `None`.
pub fn alias_key(written: &str) -> Option<String> {
    let mut u = Url::parse(written.trim()).ok()?;
    if !matches!(u.scheme(), "http" | "https") {
        return None;
    }
    u.set_fragment(None);
    canonicalize(&mut u);
    Some(u.to_string())
}

/// Scrierea fără fragment, așa cum a apărut în sursă.
pub fn written_without_fragment(written: &str) -> String {
    written.trim().split('#').next().unwrap_or("").to_string()
}

/// Cheia de gazdă pentru limitarea conexiunilor.
pub fn host_key(u: &Url) -> String {
    match u.scheme() {
        "http" | "https" | "ftp" => {
            let h = u.host_str().unwrap_or("").to_ascii_lowercase();
            match u.port_or_known_default() {
                Some(p) => format!("{h}:{p}"),
                None => h,
            }
        }
        s => s.to_string(),
    }
}

/// Gazdă comparabilă (fără „www.”).
pub fn site_key(host: &str) -> String {
    let h = host.to_ascii_lowercase();
    h.strip_prefix("www.").map(str::to_string).unwrap_or(h)
}

const INDEX_NAMES: [&str; 5] = [
    "index.html",
    "index.htm",
    "default.htm",
    "default.html",
    "index.xhtml",
];

pub fn is_index_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    INDEX_NAMES.contains(&n.as_str())
}

/// Scheme non-web legitime: nu se pot verifica, deci se ignoră fără zgomot.
pub const NON_WEB_SCHEMES: &[&str] = &[
    "javascript",
    "data",
    "tel",
    "sms",
    "mms",
    "callto",
    "skype",
    "whatsapp",
    "viber",
    "tg",
    "facetime",
    "facetime-audio",
    "geo",
    "maps",
    "magnet",
    "webcal",
    "sip",
    "sips",
    "xmpp",
    "irc",
    "ircs",
    "news",
    "nntp",
    "urn",
    "about",
    "blob",
    "intent",
    "market",
    "itms-apps",
    "itms-services",
    "bitcoin",
    "spotify",
    "steam",
    "slack",
    "zoommtg",
    "msteams",
    "fb",
    "twitter",
    "ms-settings",
    "view-source",
    "chrome",
    "edge",
    "ws",
    "wss",
    "ssh",
    "sftp",
    "git",
    "svn",
    "smb",
    "rtsp",
    "rtmp",
    "feed",
    "cid",
    "mid",
    "ldap",
    "nfs",
    "vnc",
    "rdp",
];

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != *cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Dacă schema unui URL pare o greșeală de tastare a lui http/https/ftp/mailto
/// (`htp`, `htps`, `hhtp`, `hxxp`, `mialto`…), întoarce schema probabil intenționată.
pub fn scheme_typo_target(scheme: &str) -> Option<&'static str> {
    let s = scheme.to_ascii_lowercase();
    if NON_WEB_SCHEMES.contains(&s.as_str()) || s.len() < 3 {
        return None;
    }
    for target in ["https", "http", "mailto", "ftp"] {
        if s == target {
            return None;
        }
    }
    if s.starts_with("hxxp") || s.starts_with("http") || s.starts_with("htp") {
        return Some(if s.contains('s') { "https" } else { "http" });
    }
    let best = ["https", "http", "mailto", "ftp"]
        .into_iter()
        .map(|t| (edit_distance(&s, t), t))
        .min()?;
    let limit = if best.1 == "ftp" { 1 } else { 2 };
    (best.0 <= limit).then_some(best.1)
}

fn looks_like_path(s: &str) -> bool {
    s.starts_with('.')
        || s.starts_with('/')
        || s.starts_with('\\')
        || s.starts_with('~')
        || s.contains('\\')
        || (s.len() >= 2 && s.as_bytes()[1] == b':' && s.as_bytes()[0].is_ascii_alphabetic())
}

/// Transformă o intrare de start (URL, fișier sau director) într-un URL.
pub fn parse_start(s: &str) -> Result<Url> {
    let s = s.trim();
    if s.is_empty() {
        return Err(anyhow!(tr("intrare de start goală", "empty start entry")));
    }
    let lower = s.to_ascii_lowercase();
    for scheme in ["http://", "https://", "ftp://", "file://"] {
        if lower.starts_with(scheme) {
            return Url::parse(s)
                .map_err(|e| anyhow!(trf!("URL invalid „{s}”: {e}", "invalid URL “{s}”: {e}")));
        }
    }
    let p = Path::new(s);
    if p.exists() {
        let abs = std::fs::canonicalize(p)?;
        // Windows: canonicalize produce prefix \\?\ — îl curățăm.
        let abs_str = abs.to_string_lossy().to_string();
        let abs = std::path::PathBuf::from(abs_str.trim_start_matches(r"\\?\"));
        if abs.is_dir() {
            for n in INDEX_NAMES {
                let f = abs.join(n);
                if f.is_file() {
                    return Url::from_file_path(&f).map_err(|_| {
                        anyhow!(trf!(
                            "cale de fișier invalidă: {}",
                            "invalid file path: {}",
                            f.display()
                        ))
                    });
                }
            }
            return Err(anyhow!(trf!(
                "directorul „{}” nu conține index.html / index.htm",
                "the directory “{}” contains no index.html / index.htm",
                abs.display()
            )));
        }
        return Url::from_file_path(&abs).map_err(|_| {
            anyhow!(trf!(
                "cale de fișier invalidă: {}",
                "invalid file path: {}",
                abs.display()
            ))
        });
    }
    // Arată a cale locală, dar nu există: eroare clară, nu „https://./x”.
    if looks_like_path(s) {
        return Err(anyhow!(trf!(
            "calea „{s}” nu există",
            "the path “{s}” does not exist"
        )));
    }
    if let Some((sch, _)) = s.split_once(':')
        && let Some(t) = scheme_typo_target(sch)
    {
        return Err(anyhow!(trf!(
            "schemă greșită „{sch}:” în „{s}” — probabil ați vrut {t}:",
            "wrong scheme “{sch}:” in “{s}” — did you mean {t}:"
        )));
    }
    // Un cuvânt simplu („check”, „sitee”) nu e nici fișier existent, nici domeniu.
    let host_part = s.split(['/', '?', '#']).next().unwrap_or("");
    let host_only = host_part.rsplit_once('@').map(|x| x.1).unwrap_or(host_part);
    let host_only = host_only.split(':').next().unwrap_or("");
    if !host_only.contains('.')
        && !host_only.eq_ignore_ascii_case("localhost")
        && !host_only.starts_with('[')
    {
        return Err(anyhow!(
            "„{s}” nu e nici fișier/director existent, nici URL (ex.: https://exemplu.ro)"
        ));
    }
    // Fără schemă: presupunem https.
    Url::parse(&format!("https://{s}"))
        .map_err(|e| anyhow!(trf!("URL invalid „{s}”: {e}", "invalid URL “{s}”: {e}")))
}

/// Prefixul de director al unui URL de start (până la ultimul „/”).
pub fn dir_prefix(u: &Url) -> String {
    let p = u.path();
    match p.rfind('/') {
        Some(i) => p[..=i].to_string(),
        None => "/".to_string(),
    }
}

/// Extensii care nu sunt documente HTML (nu se descarcă pentru parcurgere).
pub fn looks_like_html(u: &Url) -> bool {
    let path = u.path().to_ascii_lowercase();
    let last = path.rsplit('/').next().unwrap_or("");
    let ext = match last.rsplit_once('.') {
        Some((_, e)) => e,
        None => return true,
    };
    !matches!(
        ext,
        "pdf"
            | "zip"
            | "rar"
            | "7z"
            | "gz"
            | "tgz"
            | "tar"
            | "bz2"
            | "xz"
            | "exe"
            | "msi"
            | "dmg"
            | "iso"
            | "apk"
            | "deb"
            | "rpm"
            | "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "webp"
            | "avif"
            | "bmp"
            | "ico"
            | "svg"
            | "tif"
            | "tiff"
            | "mp3"
            | "mp4"
            | "m4a"
            | "m4v"
            | "avi"
            | "mov"
            | "mkv"
            | "webm"
            | "ogg"
            | "wav"
            | "flac"
            | "doc"
            | "docx"
            | "xls"
            | "xlsx"
            | "ppt"
            | "pptx"
            | "odt"
            | "ods"
            | "odp"
            | "epub"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "eot"
            | "json"
            | "js"
            | "css"
            | "xml"
            | "txt"
            | "csv"
    )
}

pub fn is_css_path(u: &Url) -> bool {
    u.path().to_ascii_lowercase().ends_with(".css")
}

pub fn is_html_ext(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("html" | "htm" | "xhtml" | "shtml")
    )
}

pub fn truncate_chars(s: &str, n: usize) -> String {
    let t: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() <= n {
        t
    } else {
        let mut o: String = t.chars().take(n).collect();
        o.push('…');
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_fragment() {
        let u = Url::parse("https://Exemplu.RO/a/b?x=1#sectiune").unwrap();
        assert_eq!(normalize(&u), "https://exemplu.ro/a/b?x=1");
    }

    #[test]
    fn site_key_strips_www() {
        assert_eq!(site_key("WWW.Exemplu.ro"), "exemplu.ro");
    }

    #[test]
    fn html_detection() {
        assert!(looks_like_html(&Url::parse("https://a.ro/x/").unwrap()));
        assert!(looks_like_html(&Url::parse("https://a.ro/x.php").unwrap()));
        assert!(!looks_like_html(&Url::parse("https://a.ro/x.PDF").unwrap()));
    }

    #[test]
    fn dir_prefix_works() {
        let u = Url::parse("https://a.ro/docs/index.html").unwrap();
        assert_eq!(dir_prefix(&u), "/docs/");
    }
}

#[cfg(test)]
mod alias_tests {
    use super::*;

    fn same(a: &str, b: &str) -> bool {
        alias_key(a).unwrap() == alias_key(b).unwrap()
    }

    #[test]
    fn spec13_equivalences() {
        assert!(same("HTTP://Ex.ORG/a", "http://ex.org/a"));
        assert!(same("http://ex.org:80/a", "http://ex.org/a"));
        assert!(same("https://ex.org:443/", "https://ex.org"));
        assert!(same("http://ex.org/%7Euser", "http://ex.org/~user"));
        assert!(same("http://ex.org/%7euser", "http://ex.org/~user"));
        assert!(same("http://ex.org/a/b/../c", "http://ex.org/a/c"));
        assert!(same("http://ex.org/a#x", "http://ex.org/a#y"));
        assert!(same("http://ex.org/../a", "http://ex.org/a"));
    }

    #[test]
    fn spec13_distinct() {
        assert!(!same("http://ex.org/A", "http://ex.org/a"));
        assert!(!same("http://ex.org/a%2Fb", "http://ex.org/a/b"));
        assert!(!same("http://ex.org/a?x=1&y=2", "http://ex.org/a?y=2&x=1"));
        assert!(!same("http://ex.org/a", "http://ex.org/a/"));
        assert!(!same("http://www.ex.org/a", "http://ex.org/a"));
        assert!(alias_key("not a url").is_none());
    }

    #[test]
    fn lowercase_escape_hex_is_uppercased() {
        assert_eq!(normalize_escapes("/a%2fb%7e"), "/a%2Fb~");
    }
}
