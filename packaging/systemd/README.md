# systemd (server)

```sh
sudo install -m0755 veriga /usr/bin/veriga
sudo install -d /etc/veriga
sudo install -m0644 guardian.example.json /etc/veriga/guardian.json   # edit "start"
sudo install -m0644 veriga-watch.service veriga-watch.timer /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now veriga-watch.timer
systemctl list-timers veriga-watch.timer
journalctl -u veriga-watch
```

The unit runs as a transient `DynamicUser` with a read-only file system; history lives in
`/var/lib/veriga`. To let Guardian rewrite local sources (`auto_fix`), add a drop-in with
`ReadWritePaths=/path/to/site`. The `.deb`/`.rpm` packages install exactly these files.
