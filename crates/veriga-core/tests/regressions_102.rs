//! Regresii 1.0.2: falsuri „0 rupte” la robots.txt indisponibil / site mort,
//! scheme greșite ignorate, capcane fără fund.
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::{Config, Control, LinkKind, Status};

async fn serve(listener: TcpListener) {
    loop {
        let Ok((mut s, _)) = listener.accept().await else {
            return;
        };
        tokio::spawn(async move {
            let mut buf = vec![0u8; 8192];
            let n = s.read(&mut buf).await.unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = req
                .lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("/")
                .to_string();
            let (status, body) = if path == "/robots.txt" {
                ("503 Service Unavailable", "jos".to_string())
            } else if path == "/" {
                (
                    "200 OK",
                    r#"<a href="htp://exemplu.ro/contact">t</a> <a href="tel:+40123">tel</a>
                       <a href="javascript:void(0)">js</a> <a href="/lipsa">l</a>
                       <a href="/inf/1">inf</a>"#
                        .to_string(),
                )
            } else if let Some(n) = path.strip_prefix("/inf/") {
                let n: u64 = n.parse().unwrap_or(0);
                ("200 OK", format!(r#"<a href="/inf/{}">urm</a>"#, n + 1))
            } else {
                ("404 Not Found", "nu".to_string())
            };
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(resp.as_bytes()).await;
            let _ = s.shutdown().await;
        });
    }
}

async fn start() -> String {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = l.local_addr().unwrap();
    tokio::spawn(serve(l));
    format!("http://{a}/")
}

#[tokio::test]
async fn robots_503_does_not_hide_the_site() {
    let base = start().await;
    let mut cfg = Config::with_start(base.clone());
    cfg.max_per_pattern = 20;
    let s = veriga_core::run(cfg, Control::new(), None).await.unwrap();
    let get = |u: &str| s.results.iter().find(|r| r.url == u);
    assert_eq!(get(&base).unwrap().status, Status::Ok);
    assert_eq!(
        get(&format!("{base}lipsa")).unwrap().status,
        Status::NotFound
    );
    assert!(!s.results.iter().any(|r| r.status == Status::RobotsBlocked));
    // schema greșită apare ca link rupt; tel:/javascript: se ignoră
    let typo = get("htp://exemplu.ro/contact").expect("htp: raportat");
    assert_eq!(typo.status, Status::InvalidUrl);
    assert!(typo.status.is_error());
    assert!(!s.results.iter().any(|r| r.url.starts_with("tel:")));
    // capcana /inf/N se oprește la limită și marchează verificarea incompletă
    let inf = s.results.iter().filter(|r| r.url.contains("/inf/")).count();
    assert!(inf <= 22, "{inf}");
    assert!(s.truncated);
    assert!(s.broken_count() >= 2);
}

#[tokio::test]
async fn dead_start_is_broken_not_skipped() {
    // port închis: rezervăm și eliberăm un port
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l);
    let base = format!("http://127.0.0.1:{port}/");
    let s = veriga_core::run(Config::with_start(base.clone()), Control::new(), None)
        .await
        .unwrap();
    let r = s
        .results
        .iter()
        .find(|r| r.kind == LinkKind::Start)
        .unwrap();
    assert!(r.status.is_error(), "{:?}", r.status);
    assert_eq!(s.broken_count(), 1);
}

#[test]
fn start_input_validation() {
    use veriga_core::urlutil::{parse_start, scheme_typo_target};
    assert!(parse_start("./nu-exista-deloc").is_err());
    assert!(parse_start("check").is_err());
    assert!(parse_start("htp://exemplu.ro").is_err());
    assert!(parse_start("exemplu.ro").is_ok());
    assert!(parse_start("localhost:8080").is_ok());
    assert_eq!(scheme_typo_target("htps"), Some("https"));
    assert_eq!(scheme_typo_target("hxxp"), Some("http"));
    assert_eq!(scheme_typo_target("mialto"), Some("mailto"));
    assert_eq!(scheme_typo_target("tel"), None);
    assert_eq!(scheme_typo_target("https"), None);
}
