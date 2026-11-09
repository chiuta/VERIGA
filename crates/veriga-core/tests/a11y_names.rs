use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::a11y::A11yKind;
use veriga_core::{Config, Control};

async fn run(names: bool, ambiguous: bool) -> veriga_core::Session {
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
                let body = r#"<a href="/ok">Prețuri</a><a href="/img"><img src="i.png" alt=""></a>
<a href="/dots">...</a><a href="/x">Detalii</a><a href="/y">Detalii</a>"#;
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
    cfg.max_depth = Some(0);
    cfg.check_link_names = names;
    cfg.check_ambiguous_names = ambiguous;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    veriga_core::run(cfg, Control::new(), None).await.unwrap()
}

#[tokio::test]
async fn findings_are_reported_when_enabled() {
    let s = run(true, true).await;
    let kinds: Vec<_> = s.a11y.iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&A11yKind::NoName));
    assert!(kinds.contains(&A11yKind::Meaningless));
    assert!(kinds.contains(&A11yKind::Ambiguous));
    assert_eq!(kinds.len(), 3);
    assert!(veriga_core::report::to_text(&s).contains("== "));
}

#[tokio::test]
async fn names_only_and_off() {
    let s = run(true, false).await;
    assert!(s.a11y.iter().all(|f| f.kind != A11yKind::Ambiguous) && s.a11y.len() == 2);
    let s = run(false, true).await;
    assert!(s.a11y.iter().all(|f| f.kind == A11yKind::Ambiguous) && s.a11y.len() == 1);
    assert!(run(false, false).await.a11y.is_empty());
}
