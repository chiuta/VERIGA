//! Utilitare pentru URL-uri: normalizare, intrări de start, tipuri de fișiere.

use anyhow::{Result, anyhow};
use std::path::Path;
use url::Url;

/// Cheia de deduplicare: URL fără fragment.
pub fn normalize(u: &Url) -> String {
    let mut c = u.clone();
    c.set_fragment(None);
    c.to_string()
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

/// Transformă o intrare de start (URL, fișier sau director) într-un URL.
pub fn parse_start(s: &str) -> Result<Url> {
    let s = s.trim();
    if s.is_empty() {
        return Err(anyhow!("intrare de start goală"));
    }
    let lower = s.to_ascii_lowercase();
    for scheme in ["http://", "https://", "ftp://", "file://"] {
        if lower.starts_with(scheme) {
            return Url::parse(s).map_err(|e| anyhow!("URL invalid „{s}”: {e}"));
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
                    return Url::from_file_path(&f)
                        .map_err(|_| anyhow!("cale de fișier invalidă: {}", f.display()));
                }
            }
            return Err(anyhow!(
                "directorul „{}” nu conține index.html / index.htm",
                abs.display()
            ));
        }
        return Url::from_file_path(&abs)
            .map_err(|_| anyhow!("cale de fișier invalidă: {}", abs.display()));
    }
    // Fără schemă: presupunem https.
    Url::parse(&format!("https://{s}")).map_err(|e| anyhow!("URL invalid „{s}”: {e}"))
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
