VERIGA Portable — PortableApps.com Format 3.9
=============================================

Lansatorul / Launcher: VERIGAPortable.exe (cod sursă / source: crates/veriga-launcher din depozitul https://github.com/chiuta/VERIGA).

Ce face / What it does
- alege App\VERIGA\ (x64) sau App\VERIGAARM64\ (ARM64) după arhitectura nativă;
  picks App\VERIGA\ (x64) or App\VERIGAARM64\ (ARM64) by native architecture;
- creează Data\ și copiază din App\DefaultData\ ce lipsește (nu suprascrie niciodată);
  creates Data\ and copies missing files from App\DefaultData\ (never overwrites);
- pornește veriga-gui.exe cu directorul curent Data\ (starea Guardian rămâne în Data\.veriga);
  starts veriga-gui.exe with Data\ as working directory (Guardian state stays in Data\.veriga);
- nu citește și nu scrie registrul Windows și nu scrie în %APPDATA% / does not touch the registry or %APPDATA%.

Linia de comandă / Command line
- VERIGAPortable.exe sesiune.veriga     deschide o sesiune salvată / opens a saved session
- Varianta CLI / the CLI: App\VERIGA\veriga.exe (rulează dintr-un terminal / run from a terminal)

Licență / License: MIT (vezi / see LICENSE). Fonturile și pictogramele proprii sunt incluse sub aceeași licență.
Sursa completă / Full source: https://github.com/chiuta/VERIGA
