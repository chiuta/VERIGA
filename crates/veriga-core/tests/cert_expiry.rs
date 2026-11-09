//! Avertizări de certificat pe un server TLS local (certificat de test din `fixtures/`).

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use veriga_core::cert::CertKind;
use veriga_core::{Config, Control};

async fn run(enabled: bool, warn_days: u32) -> veriga_core::Session {
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
                let body = if path == "/" {
                    r#"<a href="/a">a</a><a href="/b">b</a><a href="/c">c</a>"#
                } else {
                    "<p>x</p>"
                };
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
    cfg.ignore_tls_errors = true;
    cfg.check_cert_expiry = enabled;
    cfg.cert_warn_days = warn_days;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    veriga_core::run(cfg, Control::new(), None).await.unwrap()
}

#[tokio::test]
async fn one_finding_per_host_port_with_link_count() {
    let s = run(true, 1_000_000).await;
    assert_eq!(s.certs.len(), 1, "{:?}", s.certs);
    let f = &s.certs[0];
    assert_eq!(f.kind, CertKind::ExpiresSoon);
    assert!(f.days.unwrap() > 1000);
    assert_eq!(f.links, 4); // pagina de start + 3 legături
    assert!(f.host_port.starts_with("127.0.0.1:"));
    assert!(veriga_core::report::to_text(&s).contains("127.0.0.1"));
}

#[tokio::test]
async fn nothing_when_far_from_expiry_or_disabled() {
    assert!(run(true, 14).await.certs.is_empty());
    assert!(run(false, 1_000_000).await.certs.is_empty());
}
