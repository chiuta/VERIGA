use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::{Config, Control, OrphanConfig, Status};

async fn serve(listener: TcpListener) {
    loop {
        let (mut s, _) = match listener.accept().await {
            Ok(x) => x,
            Err(_) => return,
        };
        tokio::spawn(async move {
            let mut buf = vec![0u8; 8192];
            let n = s.read(&mut buf).await.unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let first = req.lines().next().unwrap_or("");
            let mut it = first.split_whitespace();
            let method = it.next().unwrap_or("GET");
            let path = it.next().unwrap_or("/");
            let (status, ctype, extra, body): (&str, &str, &str, String) = match path {
                "/" => ("200 OK", "text/html; charset=utf-8", "",
                    r##"<html><head><title>Acasă</title><link rel="stylesheet" href="/s.css"></head><body>
                    <a href="/a.html">A</a> <a href="/lipsa.html">Lipsa</a> <a href="/mutat">Mutat</a>
                    <a href="/a.html#exista">cu ancora</a> <a href="/a.html#nu-exista">ancora rea</a>
                    <a href="/privat/x.html">privat</a> <a href="mailto:bun@exemplu.ro">m</a><a href="mailto:rau@">r</a>
                    <img src="/i.png"><img src="/lipsa.png"></body></html>"##.to_string()),
                "/a.html" => ("200 OK", "text/html", "", r#"<title>A</title><h1 id="exista">x</h1><a href="/">inapoi</a>"#.to_string()),
                "/mutat" => ("301 Moved Permanently", "text/html", "Location: /a.html\r\n", String::new()),
                "/s.css" => ("200 OK", "text/css", "", "body{background:url(/bg.png)} .x{background:url(/lipsa-bg.png)}".to_string()),
                "/bg.png" | "/i.png" => ("200 OK", "image/png", "", "PNG".to_string()),
                "/robots.txt" => ("200 OK", "text/plain", "", "User-agent: *\nDisallow: /privat/\n".to_string()),
                _ => ("404 Not Found", "text/html", "", "nu exista".to_string()),
            };
            let body_out = if method == "HEAD" {
                String::new()
            } else {
                body.clone()
            };
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body_out}",
                body.len()
            );
            let _ = s.write_all(resp.as_bytes()).await;
            let _ = s.shutdown().await;
        });
    }
}

async fn start_server() -> String {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(serve(l));
    format!("http://{addr}/")
}

#[tokio::test]
async fn crawls_local_server() {
    let base = start_server().await;
    let mut cfg = Config::with_start(base.clone());
    cfg.check_fragments = true;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    let s = veriga_core::run(cfg, Control::new(), None).await.unwrap();

    let find = |suffix: &str| {
        s.results
            .iter()
            .find(|r| r.url.ends_with(suffix))
            .unwrap_or_else(|| {
                panic!(
                    "lipsește {suffix}: {:#?}",
                    s.results.iter().map(|r| &r.url).collect::<Vec<_>>()
                )
            })
    };
    assert_eq!(find("/a.html").status, Status::Ok);
    assert_eq!(find("/lipsa.html").status, Status::NotFound);
    assert_eq!(find("/lipsa.png").status, Status::NotFound);
    assert_eq!(find("/lipsa-bg.png").status, Status::NotFound);
    assert_eq!(find("/bg.png").status, Status::Ok);
    let mutat = find("/mutat");
    assert_eq!(mutat.status, Status::Redirected);
    assert!(mutat.final_url.as_deref().unwrap().ends_with("/a.html"));
    assert_eq!(mutat.chain[0].code, 301);
    assert_eq!(find("/privat/x.html").status, Status::RobotsBlocked);
    assert_eq!(find("mailto:rau@").status, Status::InvalidEmail);
    assert_eq!(find("mailto:bun@exemplu.ro").status, Status::Ok);
    let anc = s
        .results
        .iter()
        .find(|r| r.status == Status::FragmentMissing)
        .unwrap();
    assert!(anc.url.ends_with("#nu-exista"));
    assert_eq!(
        s.results
            .iter()
            .filter(|r| r.status == Status::FragmentMissing)
            .count(),
        1
    );
    // titlu extras și părinți înregistrați
    assert_eq!(find("/a.html").title.as_deref(), Some("A"));
    assert!(
        find("/lipsa.html")
            .parents
            .iter()
            .any(|p| p.text == "Lipsa")
    );
}

#[tokio::test]
async fn orphans_and_local_crawl() {
    let dir = std::env::temp_dir().join(format!("veriga-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(
        dir.join("index.html"),
        r#"<a href="sub/p.html">p</a><a href="nu-exista.html">x</a><img src="img/a b.png">"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("sub/p.html"),
        r#"<a href="../index.html#sus">sus</a>"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("img")).unwrap();
    std::fs::write(dir.join("img/a b.png"), "x").unwrap();
    std::fs::write(dir.join("orfan.html"), "<p>nimeni nu ma leaga</p>").unwrap();

    let mut cfg = Config::with_start(dir.to_string_lossy().to_string());
    cfg.orphan = Some(OrphanConfig {
        dir: dir.clone(),
        ..Default::default()
    });
    cfg.check_fragments = true;
    let s = veriga_core::run(cfg, Control::new(), None).await.unwrap();

    let statuses: Vec<_> = s
        .results
        .iter()
        .map(|r| (r.url.rsplit('/').next().unwrap().to_string(), r.status))
        .collect();
    assert!(
        statuses.contains(&("nu-exista.html".into(), Status::LocalMissing)),
        "{statuses:?}"
    );
    assert!(
        statuses.contains(&("orfan.html".into(), Status::Orphan)),
        "{statuses:?}"
    );
    assert!(
        statuses.contains(&("a%20b.png".into(), Status::Ok)),
        "{statuses:?}"
    );
    assert_eq!(
        s.results
            .iter()
            .filter(|r| r.status == Status::Orphan)
            .count(),
        1
    );
    assert!(
        s.results
            .iter()
            .any(|r| r.status == Status::FragmentMissing && r.url.ends_with("#sus"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn recheck_and_session_roundtrip() {
    let base = start_server().await;
    let mut cfg = Config::with_start(base);
    cfg.retries = 0;
    let s = veriga_core::run(cfg, Control::new(), None).await.unwrap();
    let before = s.broken_count();
    assert!(before >= 3);
    let p = std::env::temp_dir().join(format!("veriga-sess-{}.json", std::process::id()));
    s.save(&p).unwrap();
    let loaded = veriga_core::Session::load(&p).unwrap();
    assert_eq!(loaded.results.len(), s.results.len());
    let re = veriga_core::recheck(loaded, Arc::new(Control::default()), None)
        .await
        .unwrap();
    assert_eq!(re.broken_count(), before);
    let _ = std::fs::remove_file(&p);
}

/// O pagină la distanță care trimite spre file:// nu are voie să declanșeze citirea discului.
#[tokio::test]
async fn remote_page_cannot_read_local_files() {
    let secret = std::env::temp_dir().join(format!("veriga-secret-{}.html", std::process::id()));
    std::fs::write(&secret, "<title>SECRET-TITLU</title>").unwrap();
    let file_url = url::Url::from_file_path(&secret).unwrap().to_string();
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let page = format!(r#"<a href="{file_url}">x</a><img src="{file_url}">"#);
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = l.accept().await else {
                return;
            };
            let page = page.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let _ = s.read(&mut buf).await;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                    page.len()
                );
                let _ = s.write_all(resp.as_bytes()).await;
                let _ = s.shutdown().await;
            });
        }
    });
    let mut cfg = Config::with_start(format!("http://{addr}/"));
    cfg.respect_robots = false;
    let s = veriga_core::run(cfg, Control::new(), None).await.unwrap();
    let r = s
        .results
        .iter()
        .find(|r| r.url.starts_with("file:"))
        .expect("file link reported");
    assert_eq!(r.status, Status::Skipped, "{}", r.message);
    assert!(r.title.is_none() && r.size.is_none());
    assert!(
        !s.results
            .iter()
            .any(|r| r.title.as_deref() == Some("SECRET-TITLU"))
    );
    let _ = std::fs::remove_file(&secret);
}
