# VERIGA for YunoHost

YunoHost package (packaging format v2) for [VERIGA](https://veriga.chiuta.ro): the single-file HTML link checker at the chosen URL, the static `veriga` CLI, and an optional daily Link Guardian timer.

This directory is a YunoHost package, but YunoHost installs from a repository root, not from a subdirectory. Copy its contents to the root of a dedicated `veriga_ynh` repository, then: `yunohost app install https://github.com/<you>/veriga_ynh`.

E-mail notifications through `sendmail` do not work with the hardened unit (`NoNewPrivileges`); use the webhook option, or relax the unit yourself.

Status: structure checked in CI (`packaging/yunohost/check.py`); **not yet run through YunoHost's `package_check` on a live instance**. Please run it before submitting to the catalog.
