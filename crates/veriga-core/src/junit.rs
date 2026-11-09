//! Export de tip JUnit XML: fiecare legătură verificată este un caz de test, grupat pe pagina sursă.

use crate::model::{LinkResult, Session, Status};
use std::collections::BTreeMap;
use std::fmt::Write;

/// Caracter permis în XML 1.0 (producția `Char`).
fn legal(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
}

/// Scapă un text pentru atribut sau conținut; caracterele ilegale devin U+FFFD.
pub fn xml_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        if !legal(c) {
            o.push('\u{FFFD}');
            continue;
        }
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            // în atribute, spațiile albe altfel ar fi normalizate la spațiu
            '\n' => o.push_str("&#10;"),
            '\r' => o.push_str("&#13;"),
            '\t' => o.push_str("&#9;"),
            c => o.push(c),
        }
    }
    o
}

enum Outcome {
    Pass,
    Failure,
    Error,
    Skipped,
}

fn outcome(r: &LinkResult, s: &Session) -> Outcome {
    match r.status {
        Status::Timeout | Status::NoConnection => Outcome::Error,
        Status::Skipped | Status::RobotsBlocked => Outcome::Skipped,
        st if st.needs_review(&s.config) => Outcome::Skipped,
        st if st.is_broken(&s.config) => Outcome::Failure,
        _ => Outcome::Pass,
    }
}

fn secs(ms: u64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

/// Raportul JUnit al unei sesiuni. Fișierele orfane nu sunt legături verificate și lipsesc.
pub fn to_junit(s: &Session) -> String {
    let s = &s.redacted();
    let lang = "en";
    // pagina sursă (prima care conține legătura) -> cazuri
    let mut suites: BTreeMap<String, Vec<&LinkResult>> = BTreeMap::new();
    for r in s.results.iter().filter(|r| r.status != Status::Orphan) {
        let src = r
            .parents
            .first()
            .map(|p| p.url.clone())
            .unwrap_or_else(|| r.url.clone());
        suites.entry(src).or_default().push(r);
    }
    let mut body = String::new();
    let (mut t_all, mut f_all, mut e_all, mut k_all, mut ms_all) = (0, 0, 0, 0, 0u64);
    for (src, rs) in &suites {
        let (mut f, mut e, mut k, mut ms) = (0, 0, 0, 0u64);
        let mut cases = String::new();
        for r in rs {
            ms += r.elapsed_ms;
            let _ = write!(
                cases,
                "    <testcase name=\"{}\" classname=\"{}\" time=\"{}\"",
                xml_escape(&r.url),
                xml_escape(src),
                secs(r.elapsed_ms)
            );
            let detail = if r.message.is_empty() {
                r.status.label(lang).to_string()
            } else {
                format!("{} — {}", r.status.label(lang), r.message)
            };
            match outcome(r, s) {
                Outcome::Pass => cases.push_str("/>\n"),
                Outcome::Failure => {
                    f += 1;
                    let _ = writeln!(
                        cases,
                        ">\n      <failure message=\"{}\" type=\"{:?}\">{}</failure>\n    </testcase>",
                        xml_escape(r.status.label(lang)),
                        r.status,
                        xml_escape(&detail)
                    );
                }
                Outcome::Error => {
                    e += 1;
                    let _ = writeln!(
                        cases,
                        ">\n      <error message=\"{}\" type=\"{:?}\">{}</error>\n    </testcase>",
                        xml_escape(r.status.label(lang)),
                        r.status,
                        xml_escape(&detail)
                    );
                }
                Outcome::Skipped => {
                    k += 1;
                    let _ = writeln!(
                        cases,
                        ">\n      <skipped message=\"{}\"/>\n    </testcase>",
                        xml_escape(&detail)
                    );
                }
            }
        }
        let _ = writeln!(
            body,
            "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{f}\" errors=\"{e}\" skipped=\"{k}\" time=\"{}\">\n{cases}  </testsuite>",
            xml_escape(src),
            rs.len(),
            secs(ms)
        );
        t_all += rs.len();
        f_all += f;
        e_all += e;
        k_all += k;
        ms_all += ms;
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites name=\"veriga\" tests=\"{t_all}\" failures=\"{f_all}\" errors=\"{e_all}\" skipped=\"{k_all}\" time=\"{}\">\n{body}</testsuites>\n",
        secs(ms_all)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LinkKind, Parent};
    use quick_xml::Reader;
    use quick_xml::events::Event;

    fn session(items: &[(&str, Status, &str, &str)]) -> Session {
        let mut s = Session {
            version: 1,
            tool: "veriga".into(),
            started: "a".into(),
            finished: "b".into(),
            config: crate::Config::default(),
            results: vec![],
            ignored: 0,
            truncated: false,
            cancelled: false,
            aliases: vec![],
            a11y: vec![],
        };
        for (u, st, parent, msg) in items {
            let mut r = LinkResult::new(u, LinkKind::Page, 1, true);
            r.status = *st;
            r.message = (*msg).into();
            r.elapsed_ms = 1500;
            r.parents.push(Parent {
                url: (*parent).into(),
                text: String::new(),
                kind: LinkKind::Page,
            });
            s.results.push(r);
        }
        s
    }

    /// Parsează cu un parser XML și întoarce (nr. testcase, nr. failure, nr. error, atributele name).
    fn parse(x: &str) -> (usize, usize, usize, Vec<String>) {
        let mut rd = Reader::from_str(x);
        let (mut t, mut f, mut e) = (0, 0, 0);
        let mut names = vec![];
        loop {
            match rd.read_event().expect("XML bine format") {
                Event::Start(b) | Event::Empty(b) => match b.name().as_ref() {
                    b"testcase" => {
                        t += 1;
                        for a in b.attributes() {
                            let a = a.unwrap();
                            if a.key.as_ref() == b"name" {
                                names.push(
                                    a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .unwrap()
                                        .to_string(),
                                );
                            }
                        }
                    }
                    b"failure" => f += 1,
                    b"error" => e += 1,
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
        }
        (t, f, e, names)
    }

    #[test]
    fn t1_three_links_one_failure() {
        let s = session(&[
            ("http://x.org/a", Status::Ok, "http://x.org/", ""),
            ("http://x.org/b", Status::Ok, "http://x.org/", ""),
            ("http://x.org/c", Status::NotFound, "http://x.org/", "404"),
        ]);
        let x = to_junit(&s);
        assert!(x.contains("tests=\"3\" failures=\"1\" errors=\"0\""));
        assert_eq!(parse(&x), (3, 1, 0, parse(&x).3));
    }

    #[test]
    fn t2_timeout_is_error_not_failure() {
        let s = session(&[("http://x.org/t", Status::Timeout, "http://x.org/", "")]);
        let x = to_junit(&s);
        let (t, f, e, _) = parse(&x);
        assert_eq!((t, f, e), (1, 0, 1));
    }

    #[test]
    fn t3_special_chars_roundtrip() {
        let u = "http://x.org/a?b=1&c=\"2\"<x>";
        let s = session(&[(u, Status::NotFound, "http://x.org/p?a=1&b=2", "")]);
        let (_, _, _, names) = parse(&to_junit(&s));
        assert_eq!(names, vec![u.to_string()]);
    }

    #[test]
    fn t4_control_char_stays_well_formed() {
        let s = session(&[(
            "http://x.org/a",
            Status::NotFound,
            "http://x.org/",
            "bad \u{1} byte \u{FFFE}",
        )]);
        let x = to_junit(&s);
        assert!(!x.contains('\u{1}'));
        assert!(x.contains('\u{FFFD}'));
        parse(&x);
    }

    #[test]
    fn t5_zero_links() {
        let x = to_junit(&session(&[]));
        assert!(x.contains("tests=\"0\""));
        assert_eq!(parse(&x).0, 0);
    }

    #[test]
    fn t6_sum_of_cases_equals_links() {
        let s = session(&[
            ("http://x.org/a", Status::Ok, "http://x.org/1", ""),
            ("http://x.org/b", Status::NotFound, "http://x.org/2", ""),
            ("http://x.org/c", Status::Skipped, "http://x.org/2", ""),
        ]);
        let x = to_junit(&s);
        assert_eq!(parse(&x).0, 3);
        assert_eq!(x.matches("<testsuite ").count(), 2);
        assert!(x.contains("skipped=\"1\""));
    }

    #[test]
    fn t7_deterministic() {
        let s = session(&[("http://x.org/a", Status::NotFound, "http://x.org/", "m")]);
        assert_eq!(to_junit(&s), to_junit(&s));
    }
}
