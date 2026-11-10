=== VERIGA Link Checker ===
Contributors: alexio
Tags: broken links, link checker, seo, maintenance, trade-free
Requires at least: 6.2
Tested up to: 7.1.4
Requires PHP: 7.4
Stable tag: 1.3.0
License: MIT
License URI: https://opensource.org/licenses/MIT

Link checker (Xenu replica) inside the WordPress admin, plus an optional scheduled server-side check with e-mail reports. Trade-free: no SaaS, no tracking.

== Description ==

**VERIGA** is a clean-room link checker (a replica of Xenu Link Sleuth). This plugin brings it to WordPress in two parts:

1. **Tools → VERIGA → Link checker** — the full single-file VERIGA app, running entirely in your browser. Pick "Online site", enter your site address and start. It is served from the plugin folder, so it has the same origin as the admin and can read your public site without any cross-site permission. Nothing is sent to any third party. The app has table headers that stay fixed while scrolling, zoom buttons on the bottom bar and automatic zoom to fit the screen.
2. **Tools → VERIGA → Scheduled check** (optional) — runs the VERIGA command-line program on the server (daily or weekly, or on demand), stores an HTML and JSON report and e-mails a summary. It needs the static VERIGA binary from the GitHub releases (for example `veriga-x86_64-unknown-linux-musl`) placed outside the web root, and a host that allows running programs. Without the binary, only part 1 is available and nothing else is active.

Security design:

* Only users with `manage_options` can use it; every action is protected by a nonce.
* No shell is involved: the binary is started with an argument array, with a minimal environment and a time limit. The path must be absolute, the file name must start with "veriga", the file must be executable and not world-writable.
* The binary path cannot be edited from the admin when `DISALLOW_FILE_EDIT` is set or on multisite (unless super admin). You can fix it in `wp-config.php` with `define( 'VERIGA_BINARY', '/path/to/veriga' );`.
* Reports are stored under `uploads/veriga-lc-<random>/`, closed to direct web access, and are served only to administrators with a sandboxing Content-Security-Policy.
* The plugin makes no external requests of its own. The checker itself only fetches the addresses you ask it to check.

= What it does not do =

It does not edit your posts or fix links (use the VERIGA desktop/CLI Guardian features for that, on exported files), and it does not replace the WordPress "old slug" redirects.

== Installation ==

1. Upload the `veriga-link-checker` folder to `/wp-content/plugins/` (or install the ZIP from Plugins → Add New → Upload).
2. Activate the plugin.
3. Open **Tools → VERIGA**.
4. Optional: for scheduled checks, download the VERIGA binary for your server from https://github.com/chiuta/VERIGA/releases, place it outside the web root (for example `/usr/local/bin/veriga`), make it executable, and enter its path on the **Scheduled check** tab.

== Frequently Asked Questions ==

= Does it work on shared hosting? =

The in-browser checker does. The scheduled check needs `proc_open` and a binary that runs on your server; many shared hosts do not allow this, and the plugin tells you so.

= My site resolves to a private address (Docker, intranet). =

Tick "Allow private/local addresses" on the Scheduled check tab. VERIGA blocks private addresses by default (SSRF protection).

= Is data sent anywhere? =

No. There is no telemetry and no account. E-mails go through your site's own `wp_mail`.

== Changelog ==

= 1.3.0 =
* First WordPress release. Tested up to WordPress 7.1.4. Bundles VERIGA 1.3.0.
