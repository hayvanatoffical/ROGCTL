# deneysel/ — bağlanmamış iskelet / unwired scaffold

**TR:** Buradaki hiçbir şey çalışmıyor ve derlemenin parçası değil. Bir GUI
(Tauri) ve bir Inno Setup kurulumu için başlanmış, sonra yarıda bırakılmış
taslaklar. `setup.iss` var olmayan bir `rogctl-gui/src-tauri` ağacına işaret
ediyor; `Cargo.toml`'da tauri bağımlılığı yok. Çalışan kurulum yolu kökteki
`install.ps1` ve programın içindeki `rogctl kurulum` sihirbazıdır.

Depoyu klonlayan biri buradaki bir dosyayı çalıştırmayı denemesin diye
silinmedi ama kökten ayrıldı — emek dursun, yol yanlış görünmesin.

**EN:** Nothing here works or is part of the build. These are abandoned drafts
for a Tauri GUI and an Inno Setup installer. `setup.iss` points at a
`rogctl-gui/src-tauri` tree that does not exist, and there is no tauri
dependency in `Cargo.toml`. The working install path is `install.ps1` in the
repo root plus the built-in `rogctl kurulum` wizard.

Kept rather than deleted, but moved out of the root so no one mistakes it for
a real entry point.
