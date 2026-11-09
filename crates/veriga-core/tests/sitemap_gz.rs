use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use veriga_core::{Config, Control};

fn gz(d: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(d).unwrap();
    e.finish().unwrap()
}

async fn serve(l: TcpListener, base: String) {
    loop {
        let Ok((mut s, _)) = l.accept().await else {
            return;
        };
        let base = base.clone();
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
            let (ctype, extra, body): (&str, &str, Vec<u8>) = match path.as_str() {
                "/robots.txt" => ("text/plain", "", format!("Sitemap: {base}sitemap_index.xml\n").into_bytes()),
                "/sitemap_index.xml" => ("application/xml", "", format!("<sitemapindex><sitemap><loc>{base}one.xml.gz</loc></sitemap><sitemap><loc>{base}two</loc></sitemap><sitemap><loc>{base}three.xml</loc></sitemap><sitemap><loc>{base}sitemap_index.xml</loc></sitemap></sitemapindex>").into_bytes()),
                "/one.xml.gz" => ("application/gzip", "", gz(format!("<urlset><url><loc>{base}p1.html</loc></url><url><loc>{base}p2.html</loc></url></urlset>").as_bytes())),
                // fără extensie, doar octeții gzip
                "/two" => ("application/octet-stream", "", gz(format!("<urlset><url><loc>{base}p3.html</loc></url></urlset>").as_bytes())),
                // Content-Encoding gzip pe .xml: o singură decodare
                "/three.xml" => ("application/xml", "Content-Encoding: gzip\r\n", gz(format!("<urlset><url><loc>{base}p4.html</loc></url></urlset>").as_bytes())),
                "/" => ("text/html", "", b"<html><body>home</body></html>".to_vec()),
                p if p.starts_with("/p") => ("text/html", "", b"<html>ok</html>".to_vec()),
                _ => ("text/html", "", b"nu".to_vec()),
            };
            let status = if path.starts_with("/robots")
                || path.contains("xml")
                || path == "/two"
                || path == "/"
                || path.starts_with("/p")
            {
                "200 OK"
            } else {
                "404 Not Found"
            };
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n",
                body.len()
            );
            let _ = s.write_all(head.as_bytes()).await;
            let _ = s.write_all(&body).await;
            let _ = s.shutdown().await;
        });
    }
}

#[tokio::test]
async fn gz_sitemaps_seed_the_crawl() {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", l.local_addr().unwrap());
    tokio::spawn(serve(l, base.clone()));
    let mut cfg = Config::with_start(base.clone());
    cfg.seed_sitemaps = true;
    cfg.retries = 0;
    cfg.timeout_secs = 5;
    let s = veriga_core::run(cfg, Control::new(), None).await.unwrap();
    for p in ["p1.html", "p2.html", "p3.html", "p4.html"] {
        assert!(s.results.iter().any(|r| r.url.ends_with(p)), "lipsește {p}");
    }
}
