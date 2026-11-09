//! The CLI must accept (and write) every output format advertised by `--help` and the README.
use std::path::{Path, PathBuf};
use std::process::Command;

fn site(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("veriga-cli-fmt-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("index.html"),
        "<html><body><a href=\"ok.html\">ok</a> <a href=\"missing.html\">gone</a></body></html>",
    )
    .unwrap();
    std::fs::write(d.join("ok.html"), "<html><body>ok</body></html>").unwrap();
    d
}

fn run(site: &Path, out: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_veriga"))
        .arg(site)
        .arg("--lang")
        .arg("en")
        .arg("-o")
        .arg(out)
        .output()
        .unwrap()
}

#[test]
fn sarif_output_is_accepted_and_valid_json() {
    let d = site("sarif");
    let out = d.join("report.sarif");
    let r = run(&d, &out);
    let err = String::from_utf8_lossy(&r.stderr);
    assert!(!err.contains("unknown output extension"), "{err}");
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(v["version"], "2.1.0");
    let results = v["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 1, "exactly the one broken link: {v}");
    assert_eq!(results[0]["level"], "error");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn junit_output_is_accepted_and_has_one_failure() {
    let d = site("junit");
    let out = d.join("report.junit");
    let r = run(&d, &out);
    let err = String::from_utf8_lossy(&r.stderr);
    assert!(!err.contains("unknown output extension"), "{err}");
    let x = std::fs::read_to_string(&out).unwrap();
    assert!(x.starts_with("<?xml"), "{x}");
    assert_eq!(x.matches("<failure").count(), 1, "{x}");
    assert_eq!(x.matches("<testcase").count(), 3, "{x}"); // start page + ok + missing
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn unknown_extension_is_still_rejected_before_crawling() {
    let d = site("bad");
    let r = run(&d, &d.join("report.bogus"));
    assert!(!r.status.success());
    let err = String::from_utf8_lossy(&r.stderr);
    assert!(err.contains(".sarif .junit"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}
