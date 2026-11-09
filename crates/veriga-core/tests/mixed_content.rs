//! Conținut mixt pe o pagină servită prin TLS (certificat de test auto-semnat din `fixtures/`).

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use veriga_core::mixed::MixedCategory;
use veriga_core::{Config, Control};

async fn run(enabled: bool, csp: bool) -> veriga_core::Session {
    let cert = CertificateDer::from_pem_slice(include_bytes!("fixtures/test-cert.pem")).unwrap();
    let key = PrivateKeyDer::from_pem_slice(include_bytes!("fixtures/test-key.pem")).unwrap();
    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(cfg));
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("https://{}/", l.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = l.accept().await else {
                return;
            };
            let acc = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut s) = acc.accept(tcp).await else {
                    return;
                };
                let mut b = vec![0u8; 4096];
                let _ = s.read(&mut b).await;
                let body = r#"<img src="http://cdn.invalid/a.png"><script src="http://cdn.invalid/a.js"></script><a href="http://alt.invalid/">n</a><img src="http://localhost:9/x.png">"#;
                let extra = if csp {
                    "Content-Security-Policy: default-src 'self'; upgrade-insecure-requests\r\n"
                } else {
                    ""
                };
                let r = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(r.as_bytes()).await;
                let _ = s.shutdown().await;
            });
        }
    });
    let mut cfg = Config::with_start(base);
    cfg.ignore_tls_errors = true;
    cfg.check_mixed_content = enabled;
    cfg.check_external = false; // fără rețea: ținta http:// nu se contactează
    cfg.max_depth = Some(0);
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    veriga_core::run(cfg, Control::new(), None).await.unwrap()
}

#[tokio::test]
async fn mixed_content_on_https_page() {
    let s = run(true, false).await;
    assert_eq!(s.mixed.len(), 2, "{:?}", s.mixed);
    let img = s
        .mixed
        .iter()
        .find(|m| m.target.ends_with("a.png"))
        .unwrap();
    assert_eq!(img.category, MixedCategory::OptionallyBlockable);
    let js = s.mixed.iter().find(|m| m.target.ends_with("a.js")).unwrap();
    assert_eq!(js.category, MixedCategory::Blockable);
    assert!(img.note.is_none());
    let txt = {
        let mut c = s.clone();
        c.config.lang = "en".into();
        veriga_core::report::to_text(&c)
    };
    assert!(
        txt.lines()
            .any(|l| l == "== Mixed content (http in https) =="),
        "{txt}"
    );
    assert!(
        txt.lines().any(|l| l == format!("    {}", img.target)),
        "{txt}"
    );
}

#[tokio::test]
async fn csp_upgrade_adds_note() {
    let s = run(true, true).await;
    assert_eq!(s.mixed.len(), 2);
    assert!(s.mixed.iter().all(|m| m.note.is_some()));
}

#[tokio::test]
async fn off_by_default() {
    assert!(run(false, false).await.mixed.is_empty());
}
