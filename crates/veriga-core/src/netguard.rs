//! Protecție anti-SSRF: adrese private / loopback / link-local nu sunt contactate
//! decât dacă utilizatorul a cerut explicit (sau dacă punctul de start este deja privat).

use std::net::IpAddr;

pub fn ip_is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let o = v.octets();
            v.is_loopback()
                || v.is_private()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_broadcast()
                || v.is_multicast()
                || (o[0] == 100 && (o[1] & 0xc0) == 64) // CGNAT 100.64/10
                || o[0] == 0
                || (o[0] == 192 && o[1] == 0 && o[2] == 0) // 192.0.0.0/24
                || (o[0] == 198 && (o[1] & 0xfe) == 18) // benchmark 198.18/15
        }
        IpAddr::V6(v) => {
            if let Some(m) = v.to_ipv4_mapped() {
                return ip_is_private(IpAddr::V4(m));
            }
            let s = v.segments();
            v.is_loopback()
                || v.is_unspecified()
                || v.is_multicast()
                || (s[0] & 0xfe00) == 0xfc00 // ULA fc00::/7
                || (s[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
        }
    }
}

/// Nume care indică rețele locale, fără DNS.
pub fn name_is_local(host: &str) -> bool {
    let h = host.trim_end_matches('.').to_ascii_lowercase();
    h == "localhost"
        || h.ends_with(".localhost")
        || h.ends_with(".local")
        || h.ends_with(".internal")
        || h.ends_with(".lan")
        || h.ends_with(".home.arpa")
        || h.ends_with(".intranet")
        || h.ends_with(".corp")
        || !h.contains('.') && h.parse::<IpAddr>().is_err()
}

/// Verificare sincronă (literal IP sau nume local). Pentru DNS vezi [`host_is_private`].
pub fn host_is_private_syntactic(host: &str) -> bool {
    let h = host.trim_matches(|c| c == '[' || c == ']');
    if let Ok(ip) = h.parse::<IpAddr>() {
        return ip_is_private(ip);
    }
    name_is_local(h)
}

/// Verificare completă: literal, nume local sau rezolvare DNS spre adrese private.
pub async fn host_is_private(host: &str, port: u16) -> bool {
    if host_is_private_syntactic(host) {
        return true;
    }
    let h = host.trim_matches(|c| c == '[' || c == ']');
    if h.parse::<IpAddr>().is_ok() {
        return false;
    }
    match tokio::net::lookup_host((h, port)).await {
        Ok(addrs) => addrs.into_iter().any(|a| ip_is_private(a.ip())),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies() {
        for h in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.0.1",
            "169.254.169.254",
            "[::1]",
            "::ffff:127.0.0.1",
            "fd00::1",
            "fe80::1",
            "100.64.0.1",
            "0.0.0.0",
            "localhost",
            "foo.local",
            "router",
            "metadata.internal",
        ] {
            assert!(host_is_private_syntactic(h), "{h}");
        }
        for h in [
            "8.8.8.8",
            "example.com",
            "www.exemplu.ro",
            "2606:4700::1111",
        ] {
            assert!(!host_is_private_syntactic(h), "{h}");
        }
    }
}
