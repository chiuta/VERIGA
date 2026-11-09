use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::{Config, Control};

async fn server() -> String {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", l.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = l.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = s.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req
                    .lines()
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();
                let (st, body) = match path.as_str() {
                    "/" => (
                        "200 OK",
                        r#"<a href="/nf.html" rel="NoFollow noopener">n</a>"#,
                    ),
                    "/nf.html" => ("200 OK", r#"<a href="/deep.html">d</a>"#),
                    "/deep.html" => ("200 OK", "<p>deep</p>"),
                    _ => ("404 Not Found", "x"),
                };
                let r = format!(
                    "HTTP/1.1 {st}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(r.as_bytes()).await;
                let _ = s.shutdown().await;
            });
        }
    });
    base
}

async fn crawl(honor: bool) -> veriga_core::Session {
    let base = server().await;
    let mut cfg = Config::with_start(base);
    cfg.honor_nofollow = honor;
    cfg.report_rel = true;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    veriga_core::run(cfg, Control::new(), None).await.unwrap()
}

#[tokio::test]
async fn nofollow_target_checked_but_not_crawled_when_enabled() {
    let s = crawl(true).await;
    let nf = s
        .results
        .iter()
        .find(|r| r.url.ends_with("/nf.html"))
        .unwrap();
    assert_eq!(nf.status, veriga_core::Status::Ok);
    assert_eq!(nf.rel, vec!["nofollow", "noopener"]);
    assert!(!nf.parsed);
    assert!(!s.results.iter().any(|r| r.url.ends_with("/deep.html")));
}

#[tokio::test]
async fn nofollow_target_crawled_as_before_when_disabled() {
    let s = crawl(false).await;
    assert!(s.results.iter().any(|r| r.url.ends_with("/deep.html")));
    let nf = s
        .results
        .iter()
        .find(|r| r.url.ends_with("/nf.html"))
        .unwrap();
    assert_eq!(nf.rel, vec!["nofollow", "noopener"]);
}
