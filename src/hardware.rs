use crate::i18n::{tr, tf};
use crate::firmware::{self, Event, Profile, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}, sync::mpsc::Sender};

pub struct Device {
    pub name: &'static str,
    pub bus: &'static str,
    pub signals: &'static [&'static str],
    pub help: &'static str,
    pub url: &'static str,
}
pub const DEVICES: &[Device] = &[
    Device { name: "Contact sec / bouton / flotteur / porte", bus: "GPIO entrée", signals: &["Signal"], help: "Lecture numérique intégrée ; contact vers GND, pull-up si nécessaire.", url: "https://docs.espressif.com/projects/arduino-esp32/en/latest/api/gpio.html" },
    Device { name: "Détecteur PIR / présence / alarme", bus: "GPIO entrée", signals: &["Signal"], help: "Lecture numérique intégrée ; vérifier un signal compatible 3,3 V.", url: "https://docs.espressif.com/projects/arduino-esp32/en/latest/api/gpio.html" },
    Device { name: "Relais / MOSFET / LED", bus: "GPIO sortie", signals: &["Signal"], help: "Niveau fixe au démarrage intégré. Utiliser un étage de puissance ; jamais une bobine directement sur GPIO.", url: "https://docs.espressif.com/projects/arduino-esp32/en/latest/api/gpio.html" },
    Device { name: "BME280 · exemple météo facultatif", bus: "I²C", signals: &["SDA", "SCL"], help: "Pilote et bot météo intégrés, répéteur ESP32 à deux bus I²C uniquement. Aucun capteur imposé.", url: "https://www.bosch-sensortec.com/products/environmental-sensors/humidity-sensors-bme280/" },
    Device { name: "BMP280 / BMP390 · pression", bus: "I²C", signals: &["SDA", "SCL"], help: "Pilote spécifique requis ; un BMP280 ne remplace pas le BME280 dans le bot.", url: "https://www.bosch-sensortec.com/products/environmental-sensors/pressure-sensors/" },
    Device { name: "SHT31 / SHT4x · température et humidité", bus: "I²C", signals: &["SDA", "SCL"], help: "Pilote requis ; vérifier l’adresse et les résistances de tirage du bus.", url: "https://sensirion.com/products/catalog/SHT31-DIS-B" },
    Device { name: "INA219 / INA226 · batterie et solaire", bus: "I²C", signals: &["SDA", "SCL"], help: "Pilote et étalonnage du shunt requis pour tension, courant et puissance.", url: "https://www.ti.com/product/INA219" },
    Device { name: "ADS1115 · entrées analogiques", bus: "I²C", signals: &["SDA", "SCL"], help: "Pilote requis ; adapter la tension des sondes à la plage du convertisseur.", url: "https://www.ti.com/product/ADS1115" },
    Device { name: "BH1750 · luminosité", bus: "I²C", signals: &["SDA", "SCL"], help: "Pilote requis pour la mesure de luminosité.", url: "https://www.rohm.com/products/sensors-mems/ambient-light-sensor-ics" },
    Device { name: "SSD1306 / SH1106 · écran OLED", bus: "I²C", signals: &["SDA", "SCL"], help: "Utiliser le pilote de la variante officielle ; vérifier le partage du bus existant.", url: "https://github.com/meshcore-dev/MeshCore/tree/main/src/helpers/ui" },
    Device { name: "DS3231 · horloge RTC", bus: "I²C", signals: &["SDA", "SCL"], help: "Pilote requis pour synchroniser l’horloge ; vérifier les pull-up à 3,3 V.", url: "https://www.analog.com/en/products/ds3231.html" },
    Device { name: "GPS / GNSS u-blox", bus: "UART", signals: &["TX", "RX"], help: "TX carte → RX module, RX carte ← TX module ; pilote GNSS requis.", url: "https://www.u-blox.com/en/positioning-chips-and-modules" },
    Device { name: "RS485 / Modbus · compteur / station météo", bus: "UART", signals: &["TX", "RX", "DE/RE"], help: "Transceiver RS485 3,3 V, terminaison et pilote Modbus requis. A/B ne se branchent jamais directement aux GPIO.", url: "https://www.ti.com/product/THVD1450" },
    Device { name: "RS232 · passerelle série", bus: "UART", signals: &["TX", "RX"], help: "Convertisseur de niveaux MAX3232 requis. Choisir une variante officielle bridge_rs232 si disponible.", url: "https://github.com/meshcore-dev/MeshCore/tree/main/variants" },
    Device { name: "SPS30 · particules", bus: "UART", signals: &["TX", "RX"], help: "Pilote requis ; alimentation du module distincte des niveaux logiques UART.", url: "https://www.sensirion.com/products/catalog/SPS30" },
    Device { name: "DS18B20 · sonde de température", bus: "1-Wire", signals: &["DATA"], help: "Pilote 1-Wire requis ; résistance de tirage et alimentation adaptées.", url: "https://www.analog.com/en/products/ds18b20.html" },
    Device { name: "Pluviomètre / anémomètre / débitmètre", bus: "Compteur GPIO", signals: &["Impulsions"], help: "Comptage, anti-rebond et étalonnage requis ; la lecture numérique seule ne compte pas les impulsions.", url: "https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/peripherals/pcnt.html" },
    Device { name: "Batterie / niveau / sonde analogique", bus: "ADC", signals: &["ADC"], help: "Pilote ADC, pont diviseur et étalonnage requis ; respecter la tension maximale et les restrictions ADC/Wi-Fi.", url: "https://docs.espressif.com/projects/arduino-esp32/en/latest/api/adc.html" },
    Device { name: "Carte microSD / mémoire SPI", bus: "SPI", signals: &["SCK", "MISO", "MOSI", "CS"], help: "Pilote requis ; le bus de la radio est réservé, ne pas le réaffecter sans prise en charge du partage.", url: "https://docs.espressif.com/projects/arduino-esp32/en/latest/api/sdmmc.html" },
    Device { name: "Autre périphérique UART 3,3 V", bus: "UART", signals: &["TX", "RX"], help: "Pilote et protocole spécifiques requis ; masse commune et niveaux 3,3 V.", url: "https://docs.espressif.com/projects/arduino-esp32/en/latest/api/serial.html" },
];

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Wiring {
    pub device: String,
    pub pins: Vec<Option<u8>>,
    pub uart: u8,
    pub baud: u32,
}
pub fn adc_pin(chip: &str, pin: u8) -> bool {
    match chip { "ESP32" => matches!(pin, 0 | 2 | 4 | 12..=15 | 25..=27 | 32..=39), "ESP32-S3" | "ESP32-S2" => (1..=20).contains(&pin), "ESP32-C3" => pin <= 5, _ => false }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Pinout { pub chip: String, pub pins: Vec<Pin>, pub origin: String }
#[derive(Clone, Serialize, Deserialize)]
pub struct Pin { pub number: u8, pub output: bool, pub note: String }
impl Pin { pub fn available(&self) -> bool { self.note.is_empty() } }

pub fn validate_pins(profile: &Profile, pinout: &Pinout) -> Result<()> {
    let check = |number: u8, output: bool| -> Result<()> {
        let pin = pinout.pins.iter().find(|p| p.number == number)
            .ok_or_else(|| tf("GPIO {0} : brochage absent pour cette carte. Vérifiez les sources officielles.", &[&(number).to_string()]))?;
        if !pin.available() || output && !pin.output {
            return Err(tf("GPIO {0} indisponible pour cet usage : {1}. Choisissez une autre ligne.", &[&number.to_string(), if pin.note.is_empty() { tr("entrée seule") } else { &pin.note }]));
        }
        Ok(())
    };
    for gpio in &profile.gpio { check(gpio.pin, gpio.mode != firmware::PinMode::Input)?; }
    if profile.bot.enabled { check(profile.bot.sda, true)?; check(profile.bot.scl, true)?; }
    Ok(())
}

pub fn variant_files(source: &Path, environment: &str) -> Result<Vec<PathBuf>> {
    let marker = format!("[env:{environment}]");
    for entry in fs::read_dir(source.join("variants")).map_err(|e| e.to_string())? {
        let dir = entry.map_err(|e| e.to_string())?.path();
        let ini = fs::read_to_string(dir.join("platformio.ini")).unwrap_or_default();
        if ini.lines().any(|line| line.trim() == marker) {
            let mut files: Vec<_> = fs::read_dir(dir).map_err(|e| e.to_string())?.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("ini" | "h"))).collect();
            for line in ini.lines() {
                if let Some(board) = line.trim().strip_prefix("board = ") {
                    if board.chars().all(|c| c.is_ascii_alphanumeric() || "_-".contains(c)) {
                        let path = source.join("boards").join(format!("{board}.json"));
                        if path.is_file() && !files.contains(&path) { files.push(path); }
                    }
                }
            }
            return Ok(files);
        }
    }
    Err(tr("Brochage introuvable dans les variantes : consulter le schéma constructeur.").into())
}
pub fn local(profile: &Profile) -> Pinout {
    let files = variant_files(Path::new(&profile.source), &profile.environment).unwrap_or_default();
    let text = files.iter().filter_map(|p| fs::read_to_string(p).ok()).collect::<Vec<_>>().join("\n");
    analyze(&text, tr("Sources de la version sélectionnée · analyse conservatrice de toute la variante"))
}
pub fn refresh(profile: &Profile, tx: &Sender<Event>) -> Result<String> {
    let source = Path::new(&profile.source);
    let files = variant_files(source, &profile.environment)?;
    // Download data only; source text from the Internet is never executed.
    let tag = if profile.version == "Sources locales" { "main" } else { &profile.version };
    if tag.is_empty() || !tag.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) { return Err(tr("Version officielle requise pour consulter le brochage en ligne.").into()); }
    let mut texts = Vec::new();
    for path in files {
        let relative = path.strip_prefix(source).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
        let url = format!("https://raw.githubusercontent.com/meshcore-dev/MeshCore/{tag}/{relative}");
        let mut cmd = firmware::powershell()?;
        cmd.env("MESH_PIN_URL", url).args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.Encoding]::UTF8; (Invoke-WebRequest -UseBasicParsing -TimeoutSec 15 -Uri $env:MESH_PIN_URL).Content"]);
        let output = cmd.output().map_err(|e| e.to_string())?;
        if !output.status.success() { return Err(tr("Consultation Internet impossible. Brochage local conservé ; utilisez les liens constructeur.").into()); }
        if output.stdout.len() > 1_000_000 { return Err(tr("Document de brochage trop volumineux.").into()); }
        texts.push(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let pinout = analyze(&texts.join("\n"), &tf("GitHub officiel MeshCore · {0} · consulté en ligne", &[&(tag).to_string()]));
    let _ = tx.send(Event::Pinout(pinout));
    Ok(tr("Brochage officiel consulté sur Internet. Vérifiez les connecteurs sur le schéma constructeur.").into())
}

pub fn analyze(text: &str, origin: &str) -> Pinout {
    let lower = text.to_lowercase();
    let chip = if lower.contains("esp32-s3") || lower.contains("esp32s3") { "ESP32-S3" }
        else if lower.contains("esp32-c3") || lower.contains("esp32c3") { "ESP32-C3" }
        else if lower.contains("esp32-c6") || lower.contains("esp32c6") { "ESP32-C6" }
        else if lower.contains("esp32-s2") || lower.contains("esp32s2") { "ESP32-S2" }
        else if lower.contains("esp32dev") || lower.contains("board = ttgo") || lower.contains("\"mcu\": \"esp32\"") { "ESP32" }
        else if lower.contains("nrf52_base") || lower.contains("nrf52840") { "nRF52840 · indices Arduino" }
        else if lower.contains("rp2040") || lower.contains("rpipico") { "RP2040" }
        else { "Non identifié · consulter le schéma constructeur" };
    let numbers: Vec<u8> = match chip {
        "ESP32-S3" => (0..=21).chain(26..=48).collect(),
        "ESP32-S2" => (0..=21).chain(26..=46).collect(),
        "ESP32-C3" => (0..=21).collect(),
        "ESP32-C6" => (0..=30).collect(),
        "ESP32" => (0..=19).chain(21..=23).chain(25..=27).chain(32..=39).collect(),
        "RP2040" => (0..=29).collect(),
        "nRF52840 · indices Arduino" => {
            let count = text.lines().find_map(|line| line.trim().strip_prefix("#define NUM_DIGITAL_PINS")
                .and_then(|v| v.trim().trim_matches(['(', ')']).parse::<u8>().ok())).unwrap_or(0);
            (0..count.min(64)).collect()
        }
        _ => Vec::new(),
    };
    let mut pins: Vec<Pin> = numbers.into_iter().map(|number| {
        let note = match chip {
            "ESP32-S3" | "ESP32-S2" if (26..=37).contains(&number) => "Flash / PSRAM potentielle",
            "ESP32-S3" | "ESP32-S2" if [19, 20].contains(&number) => "USB",
            "ESP32-S3" if [0, 3, 45, 46].contains(&number) => "Démarrage (strapping)",
            "ESP32-S2" if [0, 45, 46].contains(&number) => "Démarrage (strapping)",
            "ESP32" if (6..=11).contains(&number) || [16, 17].contains(&number) => "Flash / PSRAM potentielle",
            "ESP32" if [0, 2, 5, 12, 15].contains(&number) => "Démarrage (strapping)",
            "ESP32" if [1, 3].contains(&number) => "UART0 · console / flash",
            "ESP32-S3" if [43, 44].contains(&number) => "UART0 · console",
            "ESP32-S2" if [43, 44].contains(&number) => "UART0 · console",
            "ESP32-C3" if (12..=17).contains(&number) => "Flash",
            "ESP32-C3" if [2, 8, 9].contains(&number) => "Démarrage (strapping)",
            "ESP32-C3" if [18, 19].contains(&number) => "USB",
            "ESP32-C3" if [20, 21].contains(&number) => "UART0 · console",
            "ESP32-C6" => "Vérifier le boîtier, flash, USB et strapping sur le schéma",
            "RP2040" if number >= 23 => "Périphérique interne possible · vérifier le schéma",
            _ => "",
        };
        Pin { number, output: !(chip == "ESP32" && number >= 34 || chip == "ESP32-S2" && number == 46), note: tr(note).into() }
    }).collect();
    // shortcut: macros and conditional variants are over-reserved; a verified board schematic is needed to certify exposed free pins.
    let mut definitions = Vec::new();
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if line.starts_with(';') { continue; }
        let definition = line.strip_prefix("#define").or_else(|| line.strip_prefix("-D ")).or_else(|| line.strip_prefix("-D"));
        if let Some(definition) = definition {
            let mut words = definition.split(|c: char| c == '=' || c.is_whitespace()).filter(|s| !s.is_empty());
            let name = words.next().unwrap_or("");
            if let Some(value) = words.next() { definitions.push((name, value.trim_matches(['(', ')']))); }
        }
    }
    let mut resolved: Vec<(&str, u8)> = definitions.iter().filter_map(|(name, value)| value.parse::<u8>().ok().map(|v| (*name, v))).collect();
    for _ in 0..8 {
        let aliases: Vec<_> = definitions.iter().flat_map(|(name, value)| resolved.iter()
            .filter(move |(other, _)| other == value).map(move |(_, number)| (*name, *number))).collect();
        let before = resolved.len();
        for alias in aliases { if !resolved.contains(&alias) { resolved.push(alias); } }
        if before == resolved.len() { break; }
    }
    for (name, number) in resolved {
        let is_pin = name.starts_with("PIN_") || name.starts_with("P_LORA_") || name.ends_with("_PIN") || name.starts_with("LED_");
        if !is_pin || name.ends_with("_ACTIVE") || name.ends_with("_INACTIVE") { continue; }
        if let Some(pin) = pins.iter_mut().find(|p| p.number == number) {
            if !pin.note.is_empty() { pin.note.push_str(" · "); }
            pin.note.push_str(name);
        }
    }
    Pinout { chip: chip.into(), pins, origin: origin.into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn firmware_rejects_reserved_and_input_only_outputs() {
        let pinout = analyze("board = esp32dev\n-D P_LORA_NSS=4", "test");
        let mut profile = Profile::default();
        profile.gpio.push(firmware::Gpio::default());
        assert!(validate_pins(&profile, &pinout).is_err());
        profile.gpio[0].pin = 34;
        assert!(validate_pins(&profile, &pinout).is_ok());
        profile.gpio[0].mode = firmware::PinMode::PullUp;
        assert!(validate_pins(&profile, &pinout).is_err());
        profile.gpio[0].pin = 24;
        assert!(validate_pins(&profile, &pinout).is_err());
        profile.gpio.clear(); profile.bot.enabled = true;
        assert!(validate_pins(&profile, &pinout).is_err());
    }
    #[test]
    fn selected_variant_lists_radio_and_all_processor_lines() {
        let source = firmware::root().join("sources/1793910908852502700");
        if !source.is_dir() { return; }
        let profile = Profile { source: source.to_string_lossy().into(), environment: "Heltec_v3_repeater".into(), ..Default::default() };
        let pinout = local(&profile);
        assert_eq!(pinout.chip, "ESP32-S3");
        assert_eq!(pinout.pins.len(), 45);
        assert!(pinout.pins.iter().find(|p| p.number == 8).unwrap().note.contains("P_LORA_NSS"));
        assert!(pinout.pins.iter().find(|p| p.number == 4).unwrap().available());
        assert!(profile.wiring.device.is_empty());
    }
    #[test]
    fn pinout_excludes_memory_and_preserves_board_reservations() {
        let p = analyze("board = esp32-s3-devkitc-1\n-D P_LORA_NSS=8\n#define PIN_BOARD_SDA 17\n#define PIN_VBAT_READ 1\n#define SCREEN_RESET 5\n#define PIN_SCREEN_RESET SCREEN_RESET\n-D LORA_TX_POWER=22\n; -D PIN_OTHER=4", "test");
        assert_eq!(p.pins.len(), 45);
        assert!(!p.pins.iter().any(|p| p.number == 22));
        assert!(!p.pins.iter().find(|p| p.number == 8).unwrap().available());
        assert!(!p.pins.iter().find(|p| p.number == 26).unwrap().available());
        assert!(p.pins.iter().find(|p| p.number == 4).unwrap().available());
        assert!(!p.pins.iter().find(|p| p.number == 5).unwrap().available());
        assert!(p.pins.iter().find(|p| p.number == 17).unwrap().note.contains("SDA"));
    }
    #[test]
    fn unknown_board_has_no_invented_free_pins() {
        assert!(analyze("board = unknown", "test").pins.is_empty());
        let p = analyze("board = esp32dev", "test");
        assert!(!p.pins.iter().find(|p| p.number == 34).unwrap().output);
        assert!(!p.pins.iter().any(|p| p.number == 24));
    }
}
