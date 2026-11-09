use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::sri::SriVerdict;
use veriga_core::{Config, Control};

const ABC: &str = "sha256-ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=";

async fn serve(l: TcpListener, page: String) {
    loop {
        let Ok((mut s, _)) = l.accept().await else {
            return;
        };
        let page = page.clone();
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
            let (ct, extra, body) = match path.as_str() {
                "/" => ("text/html", "", page),
                "/ok.js" | "/open.js" | "/closed.js" => (
                    "text/javascript",
                    if path == "/open.js" {
                        "Access-Control-Allow-Origin: *\r\n"
                    } else {
                        ""
                    },
                    "abc".to_string(),
                ),
                "/bad.js" => ("text/javascript", "", "abd".to_string()),
                _ => ("text/plain", "", "x".to_string()),
            };
            let r = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(r.as_bytes()).await;
            let _ = s.shutdown().await;
        });
    }
}

async fn run(enabled: bool) -> veriga_core::Session {
    let cdn = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cdn_base = format!("http://{}", cdn.local_addr().unwrap());
    let main = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", main.local_addr().unwrap());
    let page = format!(
        r#"<script src="/ok.js" integrity="{ABC}"></script>
<script src="/bad.js" integrity="{ABC}"></script>
<script src="{cdn_base}/open.js" integrity="{ABC}" crossorigin="anonymous"></script>
<script src="{cdn_base}/closed.js" integrity="{ABC}" crossorigin="anonymous"></script>
<script src="{cdn_base}/open.js?nocors=1" integrity="{ABC}"></script>
<link rel="stylesheet" href="/s1.css" integrity="md5-AAAA">
<link rel="stylesheet" href="/s2.css" integrity="sha256-QUJD">
<img src="/i.png" integrity="{ABC}">"#
    );
    tokio::spawn(serve(cdn, String::new()));
    tokio::spawn(serve(main, page));
    let mut cfg = Config::with_start(base);
    cfg.check_sri = enabled;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    cfg.max_depth = Some(0);
    veriga_core::run(cfg, Control::new(), None).await.unwrap()
}

#[tokio::test]
async fn sri_verdicts() {
    let s = run(true).await;
    let v = |suffix: &str| {
        s.sri
            .iter()
            .find(|f| f.target.ends_with(suffix))
            .map(|f| f.verdict)
    };
    assert_eq!(v("/ok.js"), Some(SriVerdict::Match));
    assert_eq!(v("/bad.js"), Some(SriVerdict::Mismatch));
    assert_eq!(v("/open.js"), Some(SriVerdict::Match));
    assert_eq!(v("/closed.js"), Some(SriVerdict::NoCors));
    assert_eq!(v("/open.js?nocors=1"), Some(SriVerdict::NoCors));
    assert_eq!(v("/s1.css"), Some(SriVerdict::NoAlgorithm));
    assert_eq!(v("/s2.css"), Some(SriVerdict::InvalidSyntax));
    assert_eq!(v("img"), Some(SriVerdict::Inapplicable));
    let txt = veriga_core::report::to_text(&s);
    assert!(txt.contains("SRI"));
}

#[tokio::test]
async fn off_by_default() {
    assert!(run(false).await.sri.is_empty());
}
