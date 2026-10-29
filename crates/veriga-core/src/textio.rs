//! Citire/scriere de fișiere-sursă cu păstrarea codificării originale.
//!
//! Fișierele UTF-8 se citesc ca atare. Fișierele non-UTF-8 (ex. Windows-1250
//! pentru site-uri românești vechi) se decodează după `<meta charset>` /
//! `@charset` / `encoding=` și se re-codifică la scriere în ACEEAȘI codificare.
//! Dacă decodarea nu e reversibilă sau un caracter nou nu poate fi reprezentat,
//! scrierea este refuzată (fișierul rămâne neatins).

use anyhow::{Result, anyhow, bail};
use encoding_rs::{Encoding, UTF_8, WINDOWS_1250};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

/// Text decodat + codificarea în care trebuie scris înapoi.
#[derive(Debug, Clone)]
pub struct Decoded {
    pub text: String,
    pub encoding: &'static Encoding,
}

fn declared(bytes: &[u8]) -> Option<&'static Encoding> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?i)(?:charset|encoding)\s*=?\s*["']?\s*([A-Za-z0-9_\-:.]+)"#).unwrap()
    });
    let head = &bytes[..bytes.len().min(4096)];
    let head = String::from_utf8_lossy(head);
    re.captures_iter(&head)
        .filter_map(|c| Encoding::for_label(c[1].as_bytes()))
        .next()
}

/// Decodează octeții unui fișier-sursă. `None` = codificare nesigură (nu se atinge).
pub fn decode(bytes: &[u8]) -> Option<Decoded> {
    if let Ok(t) = std::str::from_utf8(bytes) {
        return Some(Decoded {
            text: t.to_string(),
            encoding: UTF_8,
        });
    }
    // Implicit pentru fișiere nedeclarate: Windows-1250 (Europa Centrală / română).
    let enc = declared(bytes)
        .filter(|e| *e != UTF_8)
        .unwrap_or(WINDOWS_1250);
    let (text, had_errors) = enc.decode_without_bom_handling(bytes);
    if had_errors {
        return None;
    }
    // reversibilitate: re-codificarea trebuie să dea exact octeții originali
    let (back, _, unmappable) = enc.encode(&text);
    if unmappable || back.as_ref() != bytes {
        return None;
    }
    Some(Decoded {
        text: text.into_owned(),
        encoding: enc,
    })
}

pub fn read(path: &Path) -> Result<Decoded> {
    let bytes = std::fs::read(path)?;
    decode(&bytes).ok_or_else(|| anyhow!("codificare necunoscută sau nereversibilă"))
}

/// Codifică textul în codificarea originală; eroare dacă apar caractere nereprezentabile.
pub fn encode(text: &str, enc: &'static Encoding) -> Result<Vec<u8>> {
    if enc == UTF_8 {
        return Ok(text.as_bytes().to_vec());
    }
    let (out, _, unmappable) = enc.encode(text);
    if unmappable {
        bail!(
            "textul nou conține caractere care nu există în {} — fișier neatins",
            enc.name()
        );
    }
    Ok(out.into_owned())
}

/// `true` dacă fișierul e marcat doar-citire (nu îl modificăm, chiar dacă am putea).
pub fn is_read_only(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.permissions().readonly())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp1250_roundtrip() {
        let src = "<meta charset=\"windows-1250\"><a href=\"htp://a.ro\">Pagină şară ţară</a>";
        let (bytes, _, _) = WINDOWS_1250.encode(src);
        let d = decode(&bytes).unwrap();
        assert_eq!(d.encoding, WINDOWS_1250);
        assert_eq!(d.text, src);
        let fixed = d.text.replace("htp://", "https://");
        let out = encode(&fixed, d.encoding).unwrap();
        assert_eq!(WINDOWS_1250.decode(&out).0, fixed);
    }

    #[test]
    fn utf8_passthrough_and_unmappable_refused() {
        let d = decode("abc ș".as_bytes()).unwrap();
        assert_eq!(d.encoding, UTF_8);
        assert!(encode("中文", WINDOWS_1250).is_err());
    }
}
