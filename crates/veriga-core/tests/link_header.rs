use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::{Config, Control, Status};

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
                        "Link: </ok.css>; rel=preload; as=style, </gone.css>; rel=preload; as=style\r\nLink: </next>; rel=\"next\"\r\nLink: <broken; rel=x\r\n",
                        "<html>home</html>",
                    ),
                    "/ok.css" | "/next" => ("200 OK", "", "body{}"),
                    _ => ("404 Not Found", "", "no"),
                };
                let ct = if path.ends_with(".css") {
                    "text/css"
                } else {
                    "text/html"
                };
                let r = format!(
                    "HTTP/1.1 {st}\r\nContent-Type: {ct}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(r.as_bytes()).await;
                let _ = s.shutdown().await;
            });
        }
    });
    let mut cfg = Config::with_start(base);
    cfg.check_link_header = enabled;
    cfg.report_rel = true;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    veriga_core::run(cfg, Control::new(), None).await.unwrap()
}

#[tokio::test]
async fn header_links_are_checked_when_enabled() {
    let s = run(true).await;
    let by = |p: &str| s.results.iter().find(|r| r.url.ends_with(p));
    assert_eq!(by("/ok.css").unwrap().status, Status::Ok);
    assert_eq!(by("/gone.css").unwrap().status, Status::NotFound);
    assert_eq!(by("/next").unwrap().status, Status::Ok);
    assert!(by("/next").unwrap().rel.contains(&"next".to_string()));
    // sursa = pagina cererii
    assert!(
        by("/gone.css").unwrap().parents[0]
            .url
            .starts_with("http://127.0.0.1")
    );
    assert!(
        by("/gone.css").unwrap().parents[0]
            .text
            .starts_with("Link header")
    );
    assert_eq!(s.warnings.len(), 1);
    assert!(
        veriga_core::report::to_text(&s).contains("Avertismente")
            || veriga_core::report::to_text(&s).contains("Warnings")
    );
}

#[tokio::test]
async fn header_ignored_by_default() {
    let s = run(false).await;
    assert!(!s.results.iter().any(|r| r.url.ends_with("/gone.css")));
    assert!(s.warnings.is_empty());
}
