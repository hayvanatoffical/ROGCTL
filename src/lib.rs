//! rogctl'in paylasilan cekirdegi.
//!
//! Donanim katmani, ayarlar ve karar mantigi burada durur; hem komut satiri
//! ikilisi (`main.rs`) hem de arayuz (`rogctl-gui`) ayni koda bakar. Boylece
//! arayuzun gosterdigi deger ile daemon'un uyguladigi deger ayni yerden gelir.

pub mod acpi;
pub mod config;
pub mod control;
pub mod cooling;
pub mod devices;
pub mod gpu;
pub mod kurulum;
pub mod memory;
pub mod nvapi;
pub mod policy;
pub mod power;
pub mod process;
pub mod report;
pub mod services;
pub mod telemetry;
pub mod valorant;
