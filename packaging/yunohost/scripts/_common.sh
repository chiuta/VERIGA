#!/bin/bash
# Funcții comune pentru scripturile de instalare/actualizare/restaurare.

# Instalează fișierele aplicației din arhiva de lansare: veriga (CLI) + veriga.html (ediția web)
veriga_layout() {
    mkdir -p "$install_dir/www"
    if [ -f "$install_dir/veriga.html" ]; then
        mv -f "$install_dir/veriga.html" "$install_dir/www/index.html"
    fi
    chmod 755 "$install_dir/veriga"
    # binarul și conținutul web aparțin root (aplicația nu își poate modifica propriul executabil);
    # doar directorul de date este al utilizatorului aplicației
    chown -R root:root "$install_dir"
    chown -R "$app:$app" "$data_dir"
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
    fi
    # fără guardian_url nu schimbăm starea cronometrului (la upgrade/restaurare nu dezactivăm
    # ce a activat administratorul manual); la instalare rămâne neactivat
}

veriga_guardian_conf() {
    # nu suprascrie configurarea editată de administrator
    if [ ! -f "$data_dir/guardian.json" ]; then
        ynh_config_add --template="guardian.json" --destination="$data_dir/guardian.json"
        chown "$app:$app" "$data_dir/guardian.json"
        chmod 640 "$data_dir/guardian.json"
    fi
}
