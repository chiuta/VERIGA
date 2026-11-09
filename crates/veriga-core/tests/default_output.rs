//! The new features are additive: with a default `Config` the outputs, URL handling and
//! session format are exactly those of v1.1.0.
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::model::LinkKind;
use veriga_core::{Config, Control, Session};

async fn crawl(tweak: impl FnOnce(&mut Config)) -> (Session, String) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", l.local_addr().unwrap());
    let b2 = base.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = l.accept().await else {
                return;
            };
            let b = b2.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = s.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (status, body) = if path == "/" {
                    (
                        "200 OK",
                        format!(
                            "<html><head><link rel=\"alternate\" type=\"application/rss+xml\" href=\"{b}feed.xml\"></head><body>\
                             <a href=\"{b}u/%7euser\" rel=\"nofollow\">1</a><a href=\"{b}u/~user\">2</a></body></html>"
                        ),
                    )
                } else if path == "/robots.txt" {
                    ("404 Not Found", String::new())
                } else {
                    ("200 OK", "<html>ok</html>".to_string())
                };
                let r = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(r.as_bytes()).await;
                let _ = s.shutdown().await;
            });
        }
    });
    let mut cfg = Config::with_start(base.clone());
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    tweak(&mut cfg);
    (
        veriga_core::run(cfg, Control::new(), None).await.unwrap(),
        base,
    )
}

#[tokio::test]
async fn default_outputs_have_no_new_columns_keys_or_sections() {
    let (s, base) = crawl(|_| {}).await;
    // CSV: same header as v1.1.0, no Rel column
    let csv = veriga_core::report::to_csv(&s, b',', false);
    assert_eq!(
        csv.lines().next().unwrap(),
        "Address,Status-Code,Status,Type,Internal,Content-Type,Size,Title,Level,Redirected-To,Last-Modified,Time-ms,Message,Found-On"
    );
    // JSON: the top-level keys are exactly those of v1.1.0
    let v: serde_json::Value = serde_json::from_str(&veriga_core::report::to_json(&s)).unwrap();
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(|k| k.as_str()).collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "cancelled",
            "config",
            "finished",
            "ignored",
            "results",
            "started",
            "tool",
            "truncated",
            "version"
        ]
    );
    assert!(v["results"][0].get("rel").is_none());
    // config: none of the new options is serialised when off
    for k in [
        "honor_nofollow",
        "check_feeds",
        "canonical_urls",
        "report_aliases",
        "report_rel",
        "check_sri",
        "cert_warn_days",
        "max_link_header_links",
        "xml_declaration_encoding",
    ] {
        assert!(v["config"].get(k).is_none(), "{k}");
    }
    // text: no alias section and no rel lines
    let mut c = s.clone();
    c.config.lang = "en".into();
    let txt = veriga_core::report::to_text(&c);
    assert!(!txt.contains("Equivalent addresses"), "{txt}");
    assert!(
        !txt.lines().any(|l| l.trim_start().starts_with("rel:")),
        "{txt}"
    );
    // URLs are not rewritten: both spellings are separate results, written as in the source
    let urls: Vec<&str> = s.results.iter().map(|r| r.url.as_str()).collect();
    assert!(
        urls.contains(&format!("{base}u/%7euser").as_str()),
        "{urls:?}"
    );
    assert!(
        urls.contains(&format!("{base}u/~user").as_str()),
        "{urls:?}"
    );
    assert!(s.aliases.is_empty() && s.results.iter().all(|r| r.rel.is_empty()));
    // feed link keeps its v1.1.0 type
    let feed = s
        .results
        .iter()
        .find(|r| r.url.ends_with("/feed.xml"))
        .unwrap();
    assert_eq!(feed.kind, LinkKind::Link);
}

#[tokio::test]
async fn options_turn_the_extras_on() {
    let (s, base) = crawl(|c| {
        c.report_rel = true;
        c.report_aliases = true;
        c.canonical_urls = true;
        c.check_feeds = true;
    })
    .await;
    let csv = veriga_core::report::to_csv(&s, b',', false);
    assert!(csv.lines().next().unwrap().ends_with(",Found-On,Rel"));
    let urls: Vec<&str> = s.results.iter().map(|r| r.url.as_str()).collect();
    assert!(
        urls.contains(&format!("{base}u/~user").as_str()),
        "{urls:?}"
    );
    assert!(!urls.iter().any(|u| u.contains("%7e")), "{urls:?}");
    assert_eq!(s.aliases.len(), 1);
    let feed = s
        .results
        .iter()
        .find(|r| r.url.ends_with("/feed.xml"))
        .unwrap();
    assert_eq!(feed.kind, LinkKind::Feed);
}

#[tokio::test]
async fn session_without_new_kinds_serialises_like_v1_1_0() {
    let (s, _) = crawl(|_| {}).await;
    let json = serde_json::to_string(&s).unwrap();
    assert!(!json.contains("\"Feed\""));
    for k in [
        "\"aliases\"",
        "\"a11y\"",
        "\"warnings\"",
        "\"seo\"",
        "\"mixed\"",
        "\"sri\"",
        "\"certs\"",
        "\"rel\"",
    ] {
        assert!(!json.contains(k), "{k}");
    }
}
