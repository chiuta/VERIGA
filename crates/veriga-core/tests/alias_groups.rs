use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::{Config, Control};

#[tokio::test]
async fn alias_forms_are_grouped_and_fetched_once() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let base = format!("http://{addr}/");
    let hits = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let h2 = hits.clone();
    let b2 = base.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = l.accept().await else {
                return;
            };
            let h = h2.clone();
            let b = b2.clone();
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
                h.lock().unwrap().push(path.clone());
                let up = b.to_uppercase();
                let body = if path == "/" {
                    let host_upper = up
                        .trim_start_matches("HTTP://")
                        .trim_end_matches('/')
                        .to_string();
                    format!(
                        "<a href=\"{b}u/%7Euser\">1</a><a href=\"{b}u/~user\">2</a><a href=\"{b}u/%7euser#x\">3</a><a href=\"HTTP://{host_upper}/other\">4</a><a href=\"{b}other\">5</a><a href=\"{b}Case\">6</a><a href=\"{b}case\">7</a>"
                    )
                } else {
                    "<html>ok</html>".to_string()
                };
                let status = if path == "/robots.txt" {
                    "404 Not Found"
                } else {
                    "200 OK"
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
    cfg.report_aliases = true;
    cfg.canonical_urls = true;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    let s = veriga_core::run(cfg, Control::new(), None).await.unwrap();
    let tilde = hits
        .lock()
        .unwrap()
        .iter()
        .filter(|p| p.to_lowercase().contains("user"))
        .count();
    // GET (eventual precedat de HEAD) — dar toate scrierile duc la aceeași cheie
    assert!(
        s.results
            .iter()
            .filter(|r| r.url.ends_with("/u/~user"))
            .count()
            == 1
    );
    assert!(tilde >= 1);
    let g: Vec<_> = s.aliases.iter().map(|g| g.canonical.clone()).collect();
    assert!(g.iter().any(|c| c.ends_with("/u/~user")), "{g:?}");
    assert!(g.iter().any(|c| c.ends_with("/other")), "{g:?}");
    // Case/case sunt adrese distincte, nu alias
    assert!(!g.iter().any(|c| c.to_lowercase().ends_with("/case")));
    let user = s
        .aliases
        .iter()
        .find(|g| g.canonical.ends_with("/u/~user"))
        .unwrap();
    assert_eq!(user.forms.len(), 3); // %7E, ~, %7e (fragmentul nu contează)
    let txt = {
        let mut c = s.clone();
        c.config.lang = "en".into();
        veriga_core::report::to_text(&c)
    };
    let lines: Vec<&str> = txt.lines().collect();
    assert!(
        lines.contains(&"== Equivalent addresses written differently (info) =="),
        "{txt}"
    );
    assert!(
        lines.contains(&format!("{}u/~user", base).as_str()),
        "{txt}"
    );
    for f in ["%7Euser", "~user", "%7euser"] {
        assert!(
            lines.contains(&format!("    = {base}u/{f}").as_str()),
            "{f}: {txt}"
        );
    }
}
