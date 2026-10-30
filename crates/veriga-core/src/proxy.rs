// SPDX-License-Identifier: MIT
//! Preseturi de proxy gratuite și etice.
//!
//! Criterii pentru intrarea în listă: serviciul este gratuit, rulează *local* la utilizator
//! (un demon SOCKS5 pe `127.0.0.1`), nu cere cont, iar traficul nu trece prin servere
//! necunoscute. Listele de proxy-uri publice deschise nu sunt incluse și nu vor fi: sunt
//! adesea mașini compromise sau noduri care interceptează traficul.
//!
//! Toate adresele folosesc `socks5h://`, adică numele de gazdă este rezolvat de proxy, nu de
//! sistemul local (altfel interogările DNS ar ocoli proxy-ul).

use crate::config::Config;

/// Un preset de proxy cu descriere și avertismente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProxyPreset {
    /// Identificator folosit în CLI (`--proxy-preset tor`).
    pub id: &'static str,
    pub url: &'static str,
    pub label: &'static str,
    pub label_en: &'static str,
    /// Ce trebuie pornit înainte.
    pub requires_ro: &'static str,
    pub requires_en: &'static str,
    /// Avertisment scurt despre efectele asupra rezultatelor.
    pub caveat_ro: &'static str,
    pub caveat_en: &'static str,
    /// Plafoane de politețe față de o rețea gratuită sau voluntară.
    pub max_threads: usize,
    pub max_per_host: usize,
}

pub const PRESETS: [ProxyPreset; 3] = [
    ProxyPreset {
        id: "tor",
        url: "socks5h://127.0.0.1:9050",
        label: "Tor (serviciul de sistem)",
        label_en: "Tor (system service)",
        requires_ro: "Pornește demonul Tor (pachetul `tor`); ascultă pe 127.0.0.1:9050.",
        requires_en: "Start the Tor daemon (the `tor` package); it listens on 127.0.0.1:9050.",
        caveat_ro: "Rețea de voluntari, fără profit: verificarea a fost limitată la 4 fire și 2 conexiuni per server. Multe site-uri blochează nodurile de ieșire Tor, deci apar false „linkuri rupte” (403, 429, timeout).",
        caveat_en: "A volunteer, non-profit network: the check is capped at 4 threads and 2 connections per server. Many sites block Tor exit nodes, so expect false \"broken links\" (403, 429, timeouts).",
        max_threads: 4,
        max_per_host: 2,
    },
    ProxyPreset {
        id: "tor-browser",
        url: "socks5h://127.0.0.1:9150",
        label: "Tor Browser (deschis în fundal)",
        label_en: "Tor Browser (running in the background)",
        requires_ro: "Lasă Tor Browser pornit; ascultă pe 127.0.0.1:9150.",
        requires_en: "Keep Tor Browser running; it listens on 127.0.0.1:9150.",
        caveat_ro: "La fel ca Tor: limitat la 4 fire și 2 conexiuni per server; false „linkuri rupte” posibile.",
        caveat_en: "Same as Tor: capped at 4 threads and 2 connections per server; false \"broken links\" possible.",
        max_threads: 4,
        max_per_host: 2,
    },
    ProxyPreset {
        id: "warp",
        url: "socks5h://127.0.0.1:40000",
        label: "Cloudflare WARP (mod proxy)",
        label_en: "Cloudflare WARP (proxy mode)",
        requires_ro: "Instalează clientul WARP și rulează `warp-cli mode proxy` apoi `warp-cli connect`; portul implicit este 40000.",
        requires_en: "Install the WARP client, then run `warp-cli mode proxy` and `warp-cli connect`; the default port is 40000.",
        caveat_ro: "Gratuit, dar serviciu comercial cu client închis: Cloudflare vede domeniile verificate. Alege Tor dacă contează confidențialitatea.",
        caveat_en: "Free, but a commercial service with a closed-source client: Cloudflare sees the domains you check. Prefer Tor if privacy matters.",
        max_threads: 16,
        max_per_host: 4,
    },
];

/// Caută un preset după identificator (fără diferență între majuscule și minuscule).
pub fn preset(id: &str) -> Option<&'static ProxyPreset> {
    let id = id.trim();
    PRESETS.iter().find(|p| p.id.eq_ignore_ascii_case(id))
}

/// Aplică presetul pe configurație: setează proxy-ul și coboară plafoanele de conexiuni.
/// Nu mărește niciodată o valoare deja mai mică.
pub fn apply(cfg: &mut Config, p: &ProxyPreset) {
    cfg.proxy = Some(p.url.to_string());
    cfg.threads = cfg.threads.min(p.max_threads).max(1);
    cfg.per_host = cfg.per_host.min(p.max_per_host).max(1);
}

/// Proxy-ul rezolvă singur numele de gazdă (`socks5h://`)? Atunci rezolvarea DNS locală
/// (folosită de verificarea anti-SSRF) ar scurge interogările în afara proxy-ului.
pub fn resolves_remotely(proxy: Option<&str>) -> bool {
    proxy.is_some_and(|p| {
        p.trim()
            .get(..9)
            .is_some_and(|s| s.eq_ignore_ascii_case("socks5h:/"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_local_socks5h() {
        for p in PRESETS {
            assert!(p.url.starts_with("socks5h://127.0.0.1:"), "{}", p.id);
            assert!(resolves_remotely(Some(p.url)));
            assert!(!p.caveat_ro.is_empty() && !p.caveat_en.is_empty());
        }
    }

    #[test]
    fn ids_are_unique_and_found() {
        let mut ids: Vec<_> = PRESETS.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), PRESETS.len());
        assert_eq!(preset(" TOR ").map(|p| p.id), Some("tor"));
        assert!(preset("open-proxy-list").is_none());
        assert!(preset("").is_none());
    }

    #[test]
    fn apply_lowers_but_never_raises() {
        let mut c = Config {
            threads: 32,
            per_host: 8,
            ..Config::default()
        };
        apply(&mut c, preset("tor").unwrap());
        assert_eq!((c.threads, c.per_host), (4, 2));
        assert_eq!(c.proxy.as_deref(), Some("socks5h://127.0.0.1:9050"));
        let mut c = Config {
            threads: 1,
            per_host: 1,
            ..Config::default()
        };
        apply(&mut c, preset("tor").unwrap());
        assert_eq!((c.threads, c.per_host), (1, 1));
        let mut c = Config {
            threads: 0,
            ..Config::default()
        };
        apply(&mut c, preset("warp").unwrap());
        assert_eq!(c.threads, 1);
    }

    #[test]
    fn remote_dns_detection() {
        assert!(resolves_remotely(Some("socks5h://h:1")));
        assert!(resolves_remotely(Some("  SOCKS5H://h:1")));
        assert!(!resolves_remotely(Some("socks5://h:1")));
        assert!(!resolves_remotely(Some("http://h:1")));
        assert!(!resolves_remotely(Some("é")));
        assert!(!resolves_remotely(None));
    }
}
