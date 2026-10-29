## Link Guardian

If a site was entered at install time, `veriga watch --once` runs daily (systemd timer `veriga-watch.timer`).
Configuration: `/home/yunohost.app/veriga/guardian.json` (webhook, `sendmail`/`email_to`, depth, rules).
It is preserved across upgrades. Enable/disable:

```
sudo systemctl enable --now veriga-watch.timer
sudo systemctl disable --now veriga-watch.timer
```

The unit runs as the `veriga` system user on a read-only file system; only the data directory is writable.
To let it repair local files (`auto_fix` + `sources`), add a drop-in with `ReadWritePaths=` for those paths.

## Command line

```
sudo -u veriga /var/www/veriga/veriga check https://example.org
```

The HTML edition is static: browsers restrict cross-origin requests, so it is best suited to the pages it can reach; use the command line or Guardian for complete crawls.
