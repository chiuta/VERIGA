use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::{Config, Control, Status};

async fn run(check: bool) -> veriga_core::Session {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", l.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = l.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut b = vec![0u8; 4096];
                let _ = s.read(&mut b).await;
                let body = r#"<a href="tel:+1-201-555-0123">a</a><a href="tel:5551234">b</a>
<a href="geo:91,0">c</a><a href="geo:48.2,16.3">d</a>
<img src="data:text/plain;base64,SGVsbG8=">
<img src="data:text/plain;base64,S">"#;
                let r = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(r.as_bytes()).await;
                let _ = s.shutdown().await;
            });
        }
    });
    let mut cfg = Config::with_start(base);
    cfg.check_nonweb = check;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    veriga_core::run(cfg, Control::new(), None).await.unwrap()
}

#[tokio::test]
async fn schemes_validated_when_enabled() {
    let s = run(true).await;
    let by = |needle: &str| {
        s.results
            .iter()
            .find(|r| r.url.contains(needle))
            .unwrap()
            .status
    };
    assert_eq!(by("tel:+1-201"), Status::Ok);
    assert_eq!(by("tel:5551234"), Status::InvalidUrl);
    assert_eq!(by("geo:91,0"), Status::InvalidUrl);
    assert_eq!(by("geo:48.2"), Status::Ok);
    assert_eq!(by("SGVsbG8="), Status::Ok);
    let short = s
        .results
        .iter()
        .find(|r| r.url.ends_with("base64,S"))
        .unwrap();
    assert_eq!(short.status, Status::InvalidUrl);
}

#[tokio::test]
async fn schemes_ignored_by_default() {
    let s = run(false).await;
    assert!(!s.results.iter().any(|r| r.url.starts_with("tel:")
        || r.url.starts_with("geo:")
        || r.url.starts_with("data:")));
}
