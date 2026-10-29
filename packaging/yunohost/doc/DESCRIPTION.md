VERIGA is a clean-room link checker in the spirit of Xenu Link Sleuth: it crawls a site, reports broken links, redirect chains, slow pages and orphans, and can propose (and, if asked, apply) repairs.

This package provides:

* the **single-file HTML edition** at the chosen URL (everything runs in the visitor's browser; no server component, no database);
* the static **`veriga` command line** in the app directory, for ad-hoc checks over SSH;
* an optional **Link Guardian**: a daily systemd timer that checks a site of your choice and notifies by webhook or `sendmail`.
