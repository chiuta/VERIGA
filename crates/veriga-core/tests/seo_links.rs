use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::seo::SeoKind;
use veriga_core::{Config, Control};

async fn run(enabled: bool) -> veriga_core::Session {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", l.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = l.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut b = vec![0u8; 4096];
                let n = s.read(&mut b).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&b[..n]).to_string();
                let path = req
                    .lines()
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();
                let (st, extra, body) = match path.as_str() {
                    "/" => (
                        "200 OK",
                        "",
                        r#"<link rel="canonical" href="/gone"><link rel="alternate" hreflang="en_US" href="/en"><link rel="alternate" hreflang="x-default" href="/en"><a href="/p2">2</a><a href="/p3">3</a><a href="/ok1" hreflang="ro">ro</a>"#,
                    ),
                    "/p2" => ("200 OK", "", r#"<link rel="canonical" href="/mv">"#),
                    "/p3" => (
                        "200 OK",
                        "",
                        r#"<link rel="canonical" href="/ok1"><link rel="canonical" href="/ok2">"#,
                    ),
                    "/mv" => ("301 Moved Permanently", "Location: /\r\n", ""),
                    "/gone" => ("404 Not Found", "", "no"),
                    _ => ("200 OK", "", "<html>ok</html>"),
                };
                let r = format!(
                    "HTTP/1.1 {st}\r\nContent-Type: text/html\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(r.as_bytes()).await;
                let _ = s.shutdown().await;
            });
        }
    });
    let mut cfg = Config::with_start(base);
    cfg.check_seo_links = enabled;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    veriga_core::run(cfg, Control::new(), None).await.unwrap()
}

#[tokio::test]
async fn canonical_and_hreflang_findings() {
    let s = run(true).await;
    let has = |k: SeoKind, page_suffix: &str, val: &str| {
        s.seo
            .iter()
            .any(|f| f.kind == k && f.page.ends_with(page_suffix) && f.value.contains(val))
    };
    assert!(has(SeoKind::CanonicalBroken, "/", "/gone"));
    assert!(has(SeoKind::CanonicalRedirected, "/p2", "/mv"));
    assert!(has(SeoKind::CanonicalMultiple, "/p3", "/ok2"));
    assert!(has(SeoKind::HreflangInvalid, "/", "en_US"));
    assert!(has(SeoKind::HreflangConvention, "/", "x-default"));
    // «ro» este valid: nicio constatare
    assert!(!s.seo.iter().any(|f| f.value == "ro"));
    // redirecționata nu apare ca ruptă
    assert!(!has(SeoKind::CanonicalBroken, "/p2", "/mv"));
    assert!(veriga_core::report::to_text(&s).contains("canon"));
}

#[tokio::test]
async fn off_by_default() {
    assert!(run(false).await.seo.is_empty());
}
