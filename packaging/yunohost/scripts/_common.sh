#!/bin/bash
# Funcții comune pentru scripturile de instalare/actualizare/restaurare.

# Instalează fișierele aplicației din arhiva de lansare: veriga (CLI) + veriga.html (ediția web)
veriga_layout() {
    mkdir -p "$install_dir/www"
    if [ -f "$install_dir/veriga.html" ]; then
        mv -f "$install_dir/veriga.html" "$install_dir/www/index.html"
    fi
    chmod 755 "$install_dir/veriga"
    chown -R "$app:$app" "$install_dir" "$data_dir"
    # conținutul web este doar citit de nginx (www-data)
    chmod -R o+rX "$install_dir/www"
    chmod o+x "$install_dir"
}

# Unitățile systemd ale Guardian (oneshot + timer). Activate numai dacă a fost dat un site.
veriga_guardian() {
    ynh_config_add --template="veriga-watch.service" --destination="/etc/systemd/system/$app-watch.service"
    ynh_config_add --template="veriga-watch.timer" --destination="/etc/systemd/system/$app-watch.timer"
    systemctl daemon-reload
    if [ -n "${guardian_url:-}" ]; then
        systemctl enable --now "$app-watch.timer"
    else
        systemctl disable --now "$app-watch.timer" 2>/dev/null || true
    fi
}

veriga_guardian_conf() {
    # nu suprascrie configurarea editată de administrator
    if [ ! -f "$data_dir/guardian.json" ]; then
        ynh_config_add --template="guardian.json" --destination="$data_dir/guardian.json"
        chown "$app:$app" "$data_dir/guardian.json"
        chmod 640 "$data_dir/guardian.json"
    fi
}
