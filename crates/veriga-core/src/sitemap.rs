//! Citirea hărților de site (set de adrese sau index), cu sau fără compresie gzip.
//!
//! Conținutul se recunoaște după octeții de început (0x1F 0x8B), nu după extensia adresei.
//! Decomprimarea este plafonată, ca un fișier mic să nu poată ocupa memoria la decomprimare.

use std::collections::HashSet;
use std::io::Read;

use flate2::read::MultiGzDecoder;
use regex::Regex;

/// Dimensiunea maximă (decomprimată) a unui fișier de hartă de site.
pub const MAX_DECOMPRESSED: usize = 52_428_800;
/// Numărul maxim de adrese citite dintr-un fișier.
pub const MAX_LOCS: usize = 50_000;
/// Adâncimea maximă de urmărire a indexurilor.
pub const MAX_INDEX_DEPTH: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SitemapError {
    /// Gzip corupt sau trunchiat (inclusiv CRC32/ISIZE nepotrivite).
    Corrupt(String),
    /// Peste limita de dimensiune după decomprimare.
    TooLarge,
    /// Gzip în interiorul altui gzip.
    NestedGzip,
    /// Conținutul nu este o hartă de site XML.
    NotSitemap,
    /// Mai multe documente concatenate (de ex. membri gzip multipli).
    MultipleDocuments,
}

impl std::fmt::Display for SitemapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Corrupt(e) => write!(f, "gzip corrupt sau trunchiat: {e}"),
            Self::TooLarge => write!(f, "peste {MAX_DECOMPRESSED} octeți după decomprimare"),
            Self::NestedGzip => write!(f, "gzip imbricat, nepermis"),
            Self::NotSitemap => write!(f, "nu este o hartă de site XML"),
            Self::MultipleDocuments => write!(f, "mai multe documente concatenate"),
        }
    }
}

impl std::error::Error for SitemapError {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedSitemap {
    pub is_index: bool,
    pub locs: Vec<String>,
    /// Au existat mai mult de `MAX_LOCS` adrese; doar primele au fost păstrate.
    pub truncated: bool,
}

pub fn is_gzip(b: &[u8]) -> bool {
    b.len() >= 2 && b[0] == 0x1F && b[1] == 0x8B
}

/// Octeții documentului XML (decomprimați dacă e cazul), cu plafonul de dimensiune.
pub fn decode_bytes(bytes: &[u8]) -> Result<Vec<u8>, SitemapError> {
    if !is_gzip(bytes) {
        if bytes.len() > MAX_DECOMPRESSED {
            return Err(SitemapError::TooLarge);
        }
        return Ok(bytes.to_vec());
    }
    let mut out = Vec::new();
    let mut dec = MultiGzDecoder::new(bytes).take(MAX_DECOMPRESSED as u64 + 1);
    dec.read_to_end(&mut out)
        .map_err(|e| SitemapError::Corrupt(e.to_string()))?;
    if out.len() > MAX_DECOMPRESSED {
        return Err(SitemapError::TooLarge);
    }
    if is_gzip(&out) {
        return Err(SitemapError::NestedGzip);
    }
    Ok(out)
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Extrage adresele dintr-un set de adrese sau dintr-un index, comprimat sau nu.
pub fn parse(bytes: &[u8]) -> Result<ParsedSitemap, SitemapError> {
    let raw = decode_bytes(bytes)?;
    let text = String::from_utf8_lossy(&raw);
    let roots = text.matches("<urlset").count() + text.matches("<sitemapindex").count();
    if roots == 0 {
        return Err(SitemapError::NotSitemap);
    }
    if roots > 1 {
        return Err(SitemapError::MultipleDocuments);
    }
    let is_index = text.contains("<sitemapindex");
    let re = Regex::new(r"(?is)<loc>\s*([^<]+?)\s*</loc>").unwrap();
    let mut out = ParsedSitemap {
        is_index,
        ..Default::default()
    };
    for c in re.captures_iter(&text) {
        if out.locs.len() >= MAX_LOCS {
            out.truncated = true;
            break;
        }
        out.locs.push(unescape(&c[1]));
    }
    Ok(out)
}

/// Evidență a hărților deja vizitate (detectarea ciclurilor între indexuri).
#[derive(Default)]
pub struct Visited(HashSet<String>);

impl Visited {
    /// `true` dacă adresa nu fusese văzută până acum.
    pub fn first_time(&mut self, u: &str) -> bool {
        self.0.insert(u.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    fn gz(data: &[u8]) -> Vec<u8> {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    const SET2: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"><url><loc>http://e.org/a</loc></url><url><loc>http://e.org/b?x=1&amp;y=2</loc></url></urlset>";

    #[test]
    fn t1_gzip_set_two_urls() {
        let p = parse(&gz(SET2.as_bytes())).unwrap();
        assert_eq!(p.locs, vec!["http://e.org/a", "http://e.org/b?x=1&y=2"]);
        assert!(!p.is_index);
    }

    #[test]
    fn t2_magic_decides_not_extension() {
        let b = gz(SET2.as_bytes());
        assert_eq!(&b[..2], &[0x1F, 0x8B]);
        assert_eq!(parse(&b).unwrap().locs.len(), 2);
        // și XML simplu (adresă .gz, octeți ne-gzip)
        assert_eq!(parse(SET2.as_bytes()).unwrap().locs.len(), 2);
    }

    #[test]
    fn t3_index_with_compressed_child() {
        let idx =
            "<sitemapindex><sitemap><loc>http://e.org/s.xml.gz</loc></sitemap></sitemapindex>";
        let p = parse(idx.as_bytes()).unwrap();
        assert!(p.is_index);
        assert_eq!(p.locs, vec!["http://e.org/s.xml.gz"]);
        let child = "<urlset><url><loc>http://e.org/1</loc></url><url><loc>http://e.org/2</loc></url><url><loc>http://e.org/3</loc></url></urlset>";
        assert_eq!(parse(&gz(child.as_bytes())).unwrap().locs.len(), 3);
    }

    #[test]
    fn t4_crc_corruption_is_error() {
        let mut b = gz(SET2.as_bytes());
        let i = b.len() - 5; // ultimul octet al CRC32
        b[i] ^= 0xFF;
        assert!(matches!(parse(&b), Err(SitemapError::Corrupt(_))));
    }

    #[test]
    fn t4b_truncated_is_error() {
        let b = gz(SET2.as_bytes());
        assert!(matches!(
            parse(&b[..b.len() - 12]),
            Err(SitemapError::Corrupt(_))
        ));
    }

    #[test]
    fn t5_decompression_bomb_refused() {
        let zeros = vec![b' '; MAX_DECOMPRESSED + 10];
        let b = gz(&zeros);
        assert!(b.len() < 200_000);
        assert_eq!(decode_bytes(&b), Err(SitemapError::TooLarge));
    }

    #[test]
    fn t6_two_members_clear_error() {
        let mut b = gz(SET2.as_bytes());
        b.extend(gz(SET2.as_bytes()));
        assert_eq!(parse(&b), Err(SitemapError::MultipleDocuments));
    }

    #[test]
    fn nested_gzip_refused() {
        let b = gz(&gz(SET2.as_bytes()));
        assert_eq!(parse(&b), Err(SitemapError::NestedGzip));
    }

    #[test]
    fn non_xml_is_error() {
        assert_eq!(parse(b"hello"), Err(SitemapError::NotSitemap));
    }

    #[test]
    fn caps_at_fifty_thousand() {
        let mut s = String::from("<urlset>");
        for i in 0..50_005 {
            s.push_str(&format!("<url><loc>http://e.org/{i}</loc></url>"));
        }
        s.push_str("</urlset>");
        let p = parse(&gz(s.as_bytes())).unwrap();
        assert_eq!(p.locs.len(), MAX_LOCS);
        assert!(p.truncated);
    }

    #[test]
    fn visited_detects_cycle() {
        let mut v = Visited::default();
        assert!(v.first_time("a"));
        assert!(!v.first_time("a"));
    }
}
