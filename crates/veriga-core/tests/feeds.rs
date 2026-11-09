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
                let (st, ct, body): (&str, &str, Vec<u8>) = match path.as_str() {
                    "/" => (
                        "200 OK",
                        "text/html",
                        br#"<link rel="alternate" type="application/rss+xml" href="/feed">"#.to_vec(),
                    ),
                    "/feed" => (
                        "200 OK",
                        "application/rss+xml",
                        b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><rss><channel><title>caf\xe9</title><item><link>/good</link></item><item><link>/missing</link></item></channel></rss>".to_vec(),
                    ),
                    "/good" => ("200 OK", "text/html", b"ok".to_vec()),
                    _ => ("404 Not Found", "text/html", b"no".to_vec()),
                };
                let head = format!(
                    "HTTP/1.1 {st}\r\nContent-Type: {ct}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = s.write_all(head.as_bytes()).await;
                let _ = s.write_all(&body).await;
                let _ = s.shutdown().await;
            });
        }
    });
    let mut cfg = Config::with_start(base);
    cfg.check_feeds = enabled;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    veriga_core::run(cfg, Control::new(), None).await.unwrap()
}

#[tokio::test]
async fn feed_links_are_checked() {
    let s = run(true).await;
    let by = |p: &str| s.results.iter().find(|r| r.url.ends_with(p));
    assert_eq!(by("/feed").unwrap().status, Status::Ok);
    assert_eq!(by("/good").unwrap().status, Status::Ok);
    assert_eq!(by("/missing").unwrap().status, Status::NotFound);
    assert!(by("/missing").unwrap().parents[0].url.ends_with("/feed"));
}

#[tokio::test]
async fn feed_not_read_by_default() {
    let s = run(false).await;
    assert!(!s.results.iter().any(|r| r.url.ends_with("/missing")));
}
