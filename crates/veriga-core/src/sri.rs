//! Subresource Integrity: metadatele `integrity` și compararea lor cu rezumatul octeților.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha384, Sha512};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Alg {
    Sha256,
    Sha384,
    Sha512,
}

impl Alg {
    fn from_token(t: &str) -> Option<Alg> {
        match t.to_ascii_lowercase().as_str() {
            "sha256" => Some(Alg::Sha256),
            "sha384" => Some(Alg::Sha384),
            "sha512" => Some(Alg::Sha512),
            _ => None,
        }
    }
    fn len(self) -> usize {
        match self {
            Alg::Sha256 => 32,
            Alg::Sha384 => 48,
            Alg::Sha512 => 64,
        }
    }
}

/// Rezumatele SHA-2 ale unui corp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Digests {
    pub sha256: Vec<u8>,
    pub sha384: Vec<u8>,
    pub sha512: Vec<u8>,
}

impl Digests {
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            sha256: Sha256::digest(bytes).to_vec(),
            sha384: Sha384::digest(bytes).to_vec(),
            sha512: Sha512::digest(bytes).to_vec(),
        }
    }
    fn get(&self, a: Alg) -> &[u8] {
        match a {
            Alg::Sha256 => &self.sha256,
            Alg::Sha384 => &self.sha384,
            Alg::Sha512 => &self.sha512,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SriVerdict {
    Match,
    Mismatch,
    InvalidSyntax,
    NoAlgorithm,
    NoCors,
    /// Octeții nu au putut fi comparați (resursă prea mare sau descărcată fără rezumat).
    NotVerified,
    /// `integrity` pe un element la care HTML nu îl definește.
    Inapplicable,
}

impl SriVerdict {
    pub fn label(self, lang: &str) -> &'static str {
        let en = lang == "en";
        match (self, en) {
            (Self::Match, false) => "corespunde",
            (Self::Match, true) => "matches",
            (Self::Mismatch, false) => "nu corespunde",
            (Self::Mismatch, true) => "does not match",
            (Self::InvalidSyntax, false) => "sintaxă invalidă",
            (Self::InvalidSyntax, true) => "invalid syntax",
            (Self::NoAlgorithm, false) => "niciun algoritm recunoscut",
            (Self::NoAlgorithm, true) => "no recognised algorithm",
            (Self::NoCors, false) => "fără CORS",
            (Self::NoCors, true) => "no CORS",
            (Self::NotVerified, false) => "neverificat",
            (Self::NotVerified, true) => "not verified",
            (Self::Inapplicable, false) => "atribut neaplicabil",
            (Self::Inapplicable, true) => "attribute not applicable",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SriFinding {
    pub page: String,
    /// Adresa subresursei (sau numele elementului, pentru `Inapplicable`).
    pub target: String,
    pub verdict: SriVerdict,
}

/// Decodare base64 standard (RFC 4648 §4), cu umplutură obligatorie.
pub fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    if b.is_empty() || !b.len().is_multiple_of(4) {
        return None;
    }
    let val = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(b.len() / 4 * 3);
    let chunks = b.len() / 4;
    for (i, ch) in b.chunks(4).enumerate() {
        let last = i + 1 == chunks;
        let pad = ch.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return None;
        }
        let mut n = 0u32;
        for (k, &c) in ch.iter().enumerate() {
            let v = if k >= 4 - pad { 0 } else { val(c)? };
            n = (n << 6) | v;
        }
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..3 - pad]);
    }
    Some(out)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Metadata {
    /// Atribut gol: nu impune verificare.
    Empty,
    /// Nicio expresie cu algoritm recunoscut.
    NoAlgorithm,
    /// Expresii recunoscute (algoritm, rezumat decodat).
    Entries(Vec<(Alg, Vec<u8>)>),
}

/// Sintaxă invalidă în metadatele `integrity`.
#[derive(Debug, PartialEq, Eq)]
pub struct SyntaxError;

/// Parsează valoarea atributului `integrity`; `Err` = sintaxă invalidă.
pub fn parse_metadata(v: &str) -> Result<Metadata, SyntaxError> {
    let toks: Vec<&str> = v
        .split(['\u{20}', '\t', '\n', '\u{0C}', '\r'])
        .filter(|t| !t.is_empty())
        .collect();
    if toks.is_empty() {
        return Ok(Metadata::Empty);
    }
    let mut entries = vec![];
    for t in toks {
        let Some((alg, rest)) = t.split_once('-') else {
            continue; // nu are forma algoritm-valoare: ignorată
        };
        let Some(alg) = Alg::from_token(alg) else {
            continue; // algoritm necunoscut: ignorat
        };
        let b64 = rest.split('?').next().unwrap_or("");
        let d = b64_decode(b64).ok_or(SyntaxError)?;
        if d.len() != alg.len() {
            return Err(SyntaxError);
        }
        entries.push((alg, d));
    }
    if entries.is_empty() {
        Ok(Metadata::NoAlgorithm)
    } else {
        Ok(Metadata::Entries(entries))
    }
}

/// Compară cu rezumatele corpului: contează doar cel mai puternic algoritm prezent.
pub fn matches(entries: &[(Alg, Vec<u8>)], d: &Digests) -> bool {
    let Some(best) = entries.iter().map(|(a, _)| *a).max() else {
        return false;
    };
    entries
        .iter()
        .any(|(a, v)| *a == best && v.as_slice() == d.get(best))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC256: &str = "sha256-ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=";

    fn verdict(integrity: &str, body: &[u8]) -> Option<SriVerdict> {
        match parse_metadata(integrity) {
            Err(SyntaxError) => Some(SriVerdict::InvalidSyntax),
            Ok(Metadata::Empty) => None,
            Ok(Metadata::NoAlgorithm) => Some(SriVerdict::NoAlgorithm),
            Ok(Metadata::Entries(e)) => Some(if matches(&e, &Digests::of(body)) {
                SriVerdict::Match
            } else {
                SriVerdict::Mismatch
            }),
        }
    }

    #[test]
    fn t1_match() {
        assert_eq!(verdict(ABC256, b"abc"), Some(SriVerdict::Match));
    }

    #[test]
    fn t2_mismatch() {
        assert_eq!(verdict(ABC256, b"abd"), Some(SriVerdict::Mismatch));
    }

    #[test]
    fn t3_unknown_algorithm() {
        assert_eq!(verdict("md5-AAAA", b"abc"), Some(SriVerdict::NoAlgorithm));
    }

    #[test]
    fn t4_strongest_algorithm_wins() {
        let bad512 = format!("sha512-{}", "A".repeat(86) + "==");
        let both = format!("{ABC256} {bad512}");
        assert_eq!(verdict(&both, b"abc"), Some(SriVerdict::Mismatch));
        // fără sha512 greșit: sha256 corect → potrivire
        assert_eq!(verdict(ABC256, b"abc"), Some(SriVerdict::Match));
        // oricare dintre valorile celui mai puternic algoritm
        let good512 = {
            let d = Digests::of(b"abc").sha512;
            format!("sha512-{}", b64_encode(&d))
        };
        let multi = format!("{bad512} {good512}");
        assert_eq!(verdict(&multi, b"abc"), Some(SriVerdict::Match));
    }

    fn b64_encode(b: &[u8]) -> String {
        const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::new();
        for ch in b.chunks(3) {
            let n = (ch[0] as u32) << 16
                | (*ch.get(1).unwrap_or(&0) as u32) << 8
                | *ch.get(2).unwrap_or(&0) as u32;
            o.push(A[(n >> 18) as usize & 63] as char);
            o.push(A[(n >> 12) as usize & 63] as char);
            o.push(if ch.len() > 1 {
                A[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            o.push(if ch.len() > 2 {
                A[n as usize & 63] as char
            } else {
                '='
            });
        }
        o
    }

    #[test]
    fn t5_wrong_length_is_invalid_syntax() {
        assert_eq!(
            verdict("sha256-QUJD", b"abc"),
            Some(SriVerdict::InvalidSyntax)
        );
        assert_eq!(
            verdict("sha256-!!!!", b"abc"),
            Some(SriVerdict::InvalidSyntax)
        );
        // varianta pentru adrese ('-', '_') nu este acceptată
        assert_eq!(
            verdict(
                "sha256-ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0=",
                b"abc"
            ),
            Some(SriVerdict::InvalidSyntax)
        );
    }

    #[test]
    fn empty_attribute_is_ignored_and_options_kept() {
        assert_eq!(verdict("   ", b"abc"), None);
        assert_eq!(
            verdict(&format!("{ABC256}?ct=application/js"), b"abc"),
            Some(SriVerdict::Match)
        );
    }

    #[test]
    fn b64_roundtrip_and_padding_rules() {
        assert_eq!(b64_decode("YQ==").unwrap(), b"a");
        assert_eq!(b64_decode("YWI=").unwrap(), b"ab");
        assert_eq!(b64_decode("YWJj").unwrap(), b"abc");
        assert!(b64_decode("YQ=").is_none());
        assert!(b64_decode("Y===").is_none());
        assert!(b64_decode("YQ==YWJj").is_none());
        assert!(b64_decode("").is_none());
    }
}
