use eframe::egui::{self, Color32, RichText};
use std::{path::Path, sync::mpsc::{self, Receiver}};
mod firmware;
mod hardware;
use firmware::{BotMessage, Build, Event, Gpio, PinMode, Profile, PublicChannel, Visibility};

const ACCENT: Color32 = Color32::from_rgb(166, 205, 108);
const MUTED: Color32 = Color32::from_rgb(183, 185, 194);
const SURFACE: Color32 = Color32::from_rgb(30, 59, 96);
const BACKGROUND: Color32 = Color32::from_rgb(19, 43, 76);

fn main() -> eframe::Result {
    eframe::run_native("LoRa · Configurateur de firmware MeshCore", eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 900.0]).with_min_inner_size([1050.0, 760.0]),
        ..Default::default()
    }, Box::new(|cc| {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.style_mut_of(egui::Theme::Dark, |s| {
            s.spacing.item_spacing = egui::vec2(12.0, 12.0);
            s.spacing.interact_size.y = 36.0;
            s.spacing.button_padding = egui::vec2(12.0, 8.0);
            s.visuals.panel_fill = BACKGROUND;
            s.visuals.override_text_color = Some(Color32::from_rgb(239, 240, 244));
            s.visuals.selection.bg_fill = Color32::from_rgb(61, 70, 62);
            s.visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
            s.visuals.widgets.inactive.bg_fill = SURFACE;
            s.visuals.widgets.inactive.weak_bg_fill = SURFACE;
            s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(77, 80, 90));
            s.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, ACCENT);
            s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
            s.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(61, 70, 62);
            s.visuals.extreme_bg_color = BACKGROUND;
            s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
            s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
        });
        Ok(Box::new(Studio::default()))
    }))
}

struct Studio {
    profile: Profile,
    versions: Vec<String>,
    environments: Vec<String>,
    board_search: String,
    hardware_search: String,
    pinout: hardware::Pinout,
    tab: usize,
    status: String,
    logs: Vec<String>,
    task: Option<Receiver<Event>>,
    built: Option<Build>,
    flash_confirmed: bool,
    build_confirmed: bool,
    screenshot: Option<std::path::PathBuf>,
    frames: u32,
}
impl Default for Studio {
    fn default() -> Self {
        let profile: Profile = firmware::load(&firmware::root().join("profile.json")).unwrap_or_default();
        let environments = firmware::environments(Path::new(&profile.source)).unwrap_or_default();
        let pinout = hardware::local(&profile);
        Self { profile, environments, board_search: String::new(), hardware_search: String::new(), pinout, versions: firmware::load(&firmware::root().join("versions.json")).unwrap_or_default(),
            tab: 0, status: "Prêt à configurer votre installation.".into(), logs: Vec::new(), task: None,
            built: firmware::load(&firmware::root().join("last_build.json")), flash_confirmed: false, build_confirmed: false,
            screenshot: std::env::args_os().skip_while(|arg| arg != "--screenshot").nth(1).map(Into::into), frames: 0 }
    }
}
impl Studio {
    fn start(&mut self, job: impl FnOnce(&mpsc::Sender<Event>) -> firmware::Result<String> + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        self.task = Some(rx);
        self.build_confirmed = false;
        self.status = "Opération en cours…".into();
        std::thread::spawn(move || { let result = job(&tx); let _ = tx.send(Event::Done(result)); });
    }
    fn poll(&mut self) {
        let events: Vec<_> = self.task.as_ref().map(|rx| rx.try_iter().collect()).unwrap_or_default();
        for event in events {
            match event {
                Event::Log(line) => { self.logs.push(line); if self.logs.len() > 1500 { self.logs.remove(0); } }
                Event::Versions(v) => self.versions = v,
                Event::Sources(path, envs) => {
                    self.profile.source = path.to_string_lossy().into_owned(); self.environments = envs;
                    self.profile.environment.clear(); self.profile.pins_checked = false;
                    self.pinout = Default::default(); self.profile.wiring.pins.clear();
                }
                Event::Built(build) => { self.built = Some(build); self.flash_confirmed = false; }
                Event::Pinout(pinout) => self.pinout = pinout,
                Event::Done(result) => {
                    self.status = result.unwrap_or_else(|e| format!("Échec : {e}")); self.task = None;
                }
            }
        }
    }
    fn sources(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("VERSIONS OFFICIELLES").color(ACCENT));
            ui.label(RichText::new("MESHCORE").color(MUTED));
        });
        ui.separator();
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.strong("Version du firmware");
            if ui.button("Actualiser les versions").clicked() { self.start(firmware::sync_versions); }
        });
        let previous_version = self.profile.version.clone();
        let (number, role) = firmware::release_parts(&previous_version).unwrap_or(("", ""));
        let mut selected_number = number.to_owned();
        let mut numbers = Vec::new();
        for tag in &self.versions {
            if let Some((n, _)) = firmware::release_parts(tag) {
                if !numbers.contains(&n) { numbers.push(n); }
            }
        }
        egui::ComboBox::from_id_salt("version").width(ui.available_width())
            .selected_text(if selected_number.is_empty() { "Sélectionner une version" } else { &selected_number })
            .show_ui(ui, |ui| { for n in numbers { ui.selectable_value(&mut selected_number, n.to_owned(), n); } });
        if selected_number != number {
            self.profile.version = self.versions.iter()
                .find(|tag| firmware::release_parts(tag) == Some((selected_number.as_str(), role)))
                .or_else(|| self.versions.iter().find(|tag| firmware::release_parts(tag).is_some_and(|(n, _)| n == selected_number)))
                .cloned().unwrap_or_default();
        }
        ui.label(RichText::new("Type de firmware").small().color(MUTED));
        let selected_role = firmware::release_parts(&self.profile.version).map(|(_, r)| r).unwrap_or("Choisir d’abord une version").to_owned();
        egui::ComboBox::from_id_salt("release_role").width(ui.available_width())
            .selected_text(selected_role).show_ui(ui, |ui| {
                for tag in &self.versions {
                    if let Some((n, r)) = firmware::release_parts(tag) {
                        if n == selected_number { ui.selectable_value(&mut self.profile.version, tag.clone(), r); }
                    }
                }
            });
        if self.profile.version != previous_version {
            self.build_confirmed = false;
            self.profile.source.clear(); self.environments.clear(); self.profile.environment.clear();
            self.profile.pins_checked = false; self.flash_confirmed = false;
            self.pinout = Default::default(); self.profile.wiring.pins.clear();
        }
        if ui.add_enabled(!self.profile.version.is_empty(), egui::Button::new("Télécharger les sources officielles")).clicked() {
            let v = self.profile.version.clone(); self.start(move |tx| firmware::fetch_source(&v, tx));
        }
        ui.separator();
        if ui.button("Importer un dossier MeshCore local…").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                match firmware::environments(&path) {
                    Ok(envs) => {
                        self.build_confirmed = false;
                        self.profile.source = path.to_string_lossy().into_owned(); self.environments = envs;
                        self.profile.version = "Sources locales".into(); self.profile.environment.clear(); self.profile.pins_checked = false;
                        self.pinout = Default::default(); self.profile.wiring.pins.clear();
                    }
                    Err(e) => self.status = e,
                }
            }
        }
        ui.label(RichText::new(&self.profile.source).small().color(MUTED));
        section(ui, "Cible");
        let environments: Vec<_> = self.environments.iter().filter(|env| firmware::environment_matches_release(env, &self.profile.version)).cloned().collect();
        let previous = self.profile.environment.clone();
        ui.add(egui::TextEdit::singleline(&mut self.board_search).hint_text("Rechercher une carte : T1000, 1, Xiao, X-I…").desired_width(f32::INFINITY));
        let mut boards: Vec<_> = environments.iter().map(|env| firmware::board_name(env).to_owned()).collect();
        boards.sort(); boards.dedup();
        let mut board = firmware::board_name(&previous).to_owned();
        let old_board = board.clone();
        ui.label(RichText::new("Appareil").small().color(MUTED));
        egui::ComboBox::from_id_salt("board").width(ui.available_width())
            .selected_text(if board.is_empty() { "Choisir une carte" } else { &board })
            .show_ui(ui, |ui| {
                let mut count = 0;
                for name in &boards {
                    if firmware::board_matches(name, &self.board_search) { ui.selectable_value(&mut board, name.clone(), name); count += 1; }
                }
                if count == 0 { ui.label(if self.environments.is_empty() { "Chargez les sources pour obtenir les cartes." } else { "Aucune carte trouvée." }); }
            });
        if board != old_board {
            self.profile.environment = environments.iter().filter(|env| firmware::board_name(env) == board)
                .find(|env| firmware::connection(env) == "Bluetooth")
                .or_else(|| environments.iter().find(|env| firmware::board_name(env) == board)).cloned().unwrap_or_default();
        }
        ui.strong("Connexion du firmware");
        let mut mode = firmware::connection(&self.profile.environment).to_owned();
        let old_mode = mode.clone();
        egui::ComboBox::from_id_salt("connection").width(ui.available_width()).selected_text(if board.is_empty() { "Choisir d’abord une carte" } else { &mode }).show_ui(ui, |ui| {
            for label in ["Wi-Fi", "Bluetooth", "USB / série", "Autre rôle (sans interface compagnon)"] {
                let available = !board.is_empty() && environments.iter().any(|env| firmware::board_name(env) == board && firmware::connection(env) == label);
                ui.add_enabled_ui(available, |ui| { ui.selectable_value(&mut mode, label.to_owned(), label); });
            }
        });
        if mode != old_mode {
            self.profile.environment = environments.iter().find(|env| firmware::board_name(env) == board && firmware::connection(env) == mode).cloned().unwrap_or_default();
        }
        ui.label("Rôle / variante officielle");
        egui::ComboBox::from_id_salt("role").width(ui.available_width()).selected_text(if self.profile.environment.is_empty() { "Choisir d’abord une carte" } else { &self.profile.environment }).show_ui(ui, |ui| {
            for env in &environments {
                if firmware::board_name(env) == board && firmware::connection(env) == mode { ui.selectable_value(&mut self.profile.environment, env.clone(), env); }
            }
        });
        ui.label(RichText::new("Connexions grisées : indisponibles dans cette version pour cette carte. Le flash se fait par USB / série.").small().color(MUTED));
        if previous != self.profile.environment {
            self.profile.pins_checked = false; self.flash_confirmed = false;
            self.profile.wiring.pins.clear();
            self.pinout = hardware::local(&self.profile);
            if !self.profile.environment.is_empty() {
                let profile = self.profile.clone(); self.start(move |tx| hardware::refresh(&profile, tx));
            }
        }
        if !self.profile.environment.is_empty() {
            ui.label(RichText::new(format!("Branchements · {} · {} lignes répertoriées", self.pinout.chip, self.pinout.pins.len())).small().color(ACCENT));
            if ui.button("Choisir le matériel et voir toutes les lignes GPIO / UART →").clicked() { self.tab = 2; }
        }
    }
    fn options(&mut self, ui: &mut egui::Ui) {
        section(ui, "Options du périphérique");
        ui.checkbox(&mut self.profile.battery, "Protection batterie et reprise automatique · ESP32");
        ui.add_enabled_ui(self.profile.battery, |ui| {
            ui.horizontal(|ui| {
                ui.label("Veille sous"); ui.add(egui::DragValue::new(&mut self.profile.low_mv).range(2500..=4500).suffix(" mV"));
                ui.label("Reprise au-dessus de"); ui.add(egui::DragValue::new(&mut self.profile.resume_mv).range(2500..=4500).suffix(" mV"));
            });
            ui.horizontal(|ui| { ui.label("Vérifier pendant la veille toutes les"); ui.add(egui::DragValue::new(&mut self.profile.interval).range(10..=3600).suffix(" s")); });
        });
        ui.label(RichText::new("La reprise nécessite une alimentation revenue et un circuit de charge adapté.").small().color(MUTED));
        ui.separator();
        ui.checkbox(&mut self.profile.public_telemetry, "Autoriser des canaux de télémétrie aux invités · répéteur");
        ui.add_enabled_ui(self.profile.public_telemetry, |ui| {
            ui.horizontal(|ui| { for (i, on) in self.profile.channels.iter_mut().enumerate() { ui.checkbox(on, format!("Canal {}", i + 1)); } });
            ui.label("L’accès invité suit les règles de connexion MeshCore. Les coordonnées GPS restent privées. Les GPIO publics utilisent les canaux 100–103.");
        });
        ui.separator();
        ui.checkbox(&mut self.profile.quiet, "Désactiver les journaux de débogage");
        ui.checkbox(&mut self.profile.advertise, "Annoncer le nœud au démarrage");
    }
    fn hardware(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(format!("Carte : {} · {}", self.profile.environment, self.pinout.chip)).color(ACCENT));
        ui.label(RichText::new(&self.pinout.origin).small().color(MUTED));
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.profile.environment.is_empty(), egui::Button::new("Actualiser le brochage sur Internet")).clicked() {
                let profile = self.profile.clone(); self.start(move |tx| hardware::refresh(&profile, tx));
            }
            ui.hyperlink_to("Schémas / variantes MeshCore", "https://github.com/meshcore-dev/MeshCore/tree/main/variants");
        });
        ui.add(egui::TextEdit::singleline(&mut self.hardware_search).hint_text("Rechercher un matériel : GPS, relais, batterie, UART…").desired_width(f32::INFINITY));
        let previous = self.profile.wiring.device.clone();
        egui::ComboBox::from_id_salt("hardware").width(ui.available_width())
            .selected_text(if previous.is_empty() { "Choisir un matériel · aucun capteur obligatoire" } else { &previous })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.profile.wiring.device, String::new(), "Aucun périphérique supplémentaire");
                let mut count = 0;
                for device in hardware::DEVICES {
                    if firmware::board_matches(&format!("{} {} {}", device.name, device.bus, device.help), &self.hardware_search) {
                        ui.selectable_value(&mut self.profile.wiring.device, device.name.into(), format!("{} · {}", device.name, device.bus)); count += 1;
                    }
                }
                if count == 0 { ui.label("Aucun résultat : essayez le nom du bus ou une autre référence."); }
            });
        if self.profile.wiring.device != previous {
            self.profile.wiring.pins.clear(); self.profile.pins_checked = false;
        }
        if let Some(device) = hardware::DEVICES.iter().find(|d| d.name == self.profile.wiring.device) {
            ui.label(device.help);
            ui.hyperlink_to("Documentation du périphérique", device.url);
            ui.label("Plan de câblage enregistré avec le profil. Il est appliqué au firmware uniquement avec le bouton ci-dessous pour les fonctions intégrées.");
            self.profile.wiring.pins.resize(device.signals.len(), None);
            for (i, signal) in device.signals.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(*signal);
                    let current = &mut self.profile.wiring.pins[i];
                    egui::ComboBox::from_id_salt(("wire", i)).width(330.0)
                        .selected_text(current.map(|p| format!("GPIO {p}")).unwrap_or("Choisir une ligne".into()))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(current, None, "Non affectée");
                            for pin in &self.pinout.pins {
                                let input_only = *signal == "RX" || device.bus == "GPIO entrée" || device.bus == "ADC";
                                let adc = device.bus != "ADC" || hardware::adc_pin(&self.pinout.chip, pin.number);
                                let used = self.profile.gpio.iter().any(|g| g.pin == pin.number)
                                    || self.profile.bot.enabled && [self.profile.bot.sda, self.profile.bot.scl].contains(&pin.number);
                                ui.add_enabled_ui(pin.available() && (pin.output || input_only) && adc && !used, |ui| {
                                    ui.selectable_value(current, Some(pin.number), format!("GPIO {} · {}{}", pin.number, pin_status(pin), if used { " · affecté dans le profil" } else { "" }));
                                });
                            }
                            if self.pinout.pins.is_empty() { ui.label("Brochage inconnu : consulter le schéma constructeur."); }
                        });
                });
            }
            if device.bus == "UART" {
                ui.horizontal(|ui| {
                    let count = if matches!(self.pinout.chip.as_str(), "ESP32" | "ESP32-S3") { 3 } else { 2 };
                    let console = self.pinout.chip.starts_with("ESP32");
                    if self.profile.wiring.uart >= count || console && self.profile.wiring.uart == 0 { self.profile.wiring.uart = 1; }
                    egui::ComboBox::from_id_salt("uart").selected_text(format!("UART{}", self.profile.wiring.uart)).show_ui(ui, |ui| {
                        for index in 0..count {
                            ui.add_enabled_ui(!console || index != 0, |ui| { ui.selectable_value(&mut self.profile.wiring.uart, index, format!("UART{index}{}", if console && index == 0 { " · réservé console / flash" } else { " · disponibilité à vérifier" })); });
                        }
                    });
                    if self.profile.wiring.baud == 0 { self.profile.wiring.baud = 9600; }
                    ui.add(egui::DragValue::new(&mut self.profile.wiring.baud).range(300..=2_000_000).suffix(" bauds"));
                });
                ui.label("TX → RX, RX ← TX et GND commun. UART1/2 peuvent déjà être utilisés par le GPS ou une passerelle ; contrôler la variante. Le plan seul n’active aucun protocole série.");
            }
            let unique = self.profile.wiring.pins.iter().enumerate().all(|(i, pin)| pin.is_some() && !self.profile.wiring.pins[..i].contains(pin));
            if !unique { ui.label(RichText::new("Choisissez des lignes distinctes pour chaque signal.").color(MUTED)); }
            let integrated = matches!(device.bus, "GPIO entrée" | "GPIO sortie") || device.name.starts_with("BME280");
            if integrated {
                let supported = self.profile.environment.to_lowercase().ends_with("_repeater") && self.pinout.chip.starts_with("ESP32");
                if ui.add_enabled(unique && supported, egui::Button::new("Appliquer à la configuration du firmware")).clicked() {
                    if device.name.starts_with("BME280") {
                        self.profile.bot.sda = self.profile.wiring.pins[0].unwrap(); self.profile.bot.scl = self.profile.wiring.pins[1].unwrap();
                        self.profile.bot.enabled = true;
                    } else if self.profile.gpio.len() < 4 {
                        let channel = (100..=103).find(|c| self.profile.gpio.iter().all(|g| g.channel != *c)).unwrap_or(100);
                        self.profile.gpio.push(Gpio { name: device.name.into(), pin: self.profile.wiring.pins[0].unwrap(), channel,
                            mode: if device.bus == "GPIO sortie" { PinMode::Low } else { PinMode::Input }, ..Default::default() });
                    } else { self.status = "Quatre GPIO déjà configurés : supprimez une ligne avant d’en ajouter une.".into(); }
                    self.profile.pins_checked = false;
                }
                if !supported { ui.label("Application intégrée réservée aux répéteurs ESP32 ; les autres cartes nécessitent un pilote adapté."); }
            } else { ui.label(RichText::new("Plan uniquement : pilote spécifique non intégré ici ; sélectionner une variante officielle adaptée ou ajouter le pilote aux sources avant compilation.").color(ACCENT)); }
        }
        egui::CollapsingHeader::new(format!("Toutes les lignes GPIO / UART · {} lignes", self.pinout.pins.len())).default_open(true).show(ui, |ui| {
            ui.label("Lignes du processeur : « candidate » signifie sans réservation détectée, pas un connecteur certifié libre. Vérifiez le schéma et la révision de votre carte. Pour nRF52, les numéros sont des indices Arduino, à convertir en P0/P1 via variant.cpp. Sur RP2040, vérifier les fonctions alternatives UART/SPI. Réservations conservatrices ; alimentation et GND sur le schéma.");
            egui::ScrollArea::vertical().id_salt("all_pins").max_height(200.0).show(ui, |ui| {
                for pin in &self.pinout.pins {
                    let used = self.profile.gpio.iter().any(|g| g.pin == pin.number)
                        || self.profile.bot.enabled && [self.profile.bot.sda, self.profile.bot.scl].contains(&pin.number);
                    ui.label(format!("GPIO {:02} · {}{}", pin.number, pin_status(pin), if used { " · affecté dans le profil" } else { "" }));
                }
                if self.pinout.pins.is_empty() { ui.label("Aucune ligne certifiée pour cette carte. Ouvrir le schéma constructeur ; aucune broche libre inventée."); }
            });
            ui.hyperlink_to("GPIO et UART · documentation Espressif", "https://docs.espressif.com/projects/arduino-esp32/en/latest/api/serial.html");
        });
    }
    fn gpio(&mut self, ui: &mut egui::Ui) {
        ui.heading("GPIO, UART & matériel");
        self.hardware(ui);
        ui.separator();
        if ui.checkbox(&mut self.profile.bot.enabled, "Exemple facultatif : bot météo BME280 · ESP32").changed() { self.profile.pins_checked = false; }
        ui.add_enabled_ui(self.profile.bot.enabled, |ui| {
            ui.label("ESP32 à deux contrôleurs I²C requis (ex. ESP32/S3). Bus dédié au bot : broches libres, distinctes des GPIO ci-dessous et des périphériques de la carte.");
            ui.horizontal(|ui| {
                ui.label("SDA / GPIO");
                let a = pin_choice(ui, "sda", &mut self.profile.bot.sda, &self.pinout, true);
                ui.label("SCL / GPIO");
                let b = pin_choice(ui, "scl", &mut self.profile.bot.scl, &self.pinout, true);
                if a || b { self.profile.pins_checked = false; }
            });
            egui::ComboBox::from_id_salt("bme_address").selected_text(format!("Adresse I²C : 0x{:02X}", self.profile.bot.address)).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.profile.bot.address, 0x76, "0x76");
                ui.selectable_value(&mut self.profile.bot.address, 0x77, "0x77");
            });
            ui.label("Mesure chaque minute. Min/max du jour et records conservés en flash NVS dès chaque nouvel extrême, même après coupure. Un effacement complet les supprime.");
            if ui.button("Configurer les rapports du bot →").clicked() { self.tab = 3; }
        });
        ui.separator();
        ui.label("Initialisez une broche au démarrage et partagez l’état d’un contact numérique.");
        ui.label(RichText::new("ESP32 · rôle répéteur · 4 broches maximum").color(ACCENT));
        ui.label("Entrée : état 0/1 lu à la demande en télémétrie. Sortie : niveau fixe au démarrage, sans commande distante. Les capteurs I²C et analogiques nécessitent un pilote adapté.");
        ui.add_space(8.0);
        let mut remove = None;
        let mut changed = false;
        for (i, g) in self.profile.gpio.iter_mut().enumerate() {
            ui.push_id(i, |ui| { ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("{:02}", i + 1));
                    changed |= ui.text_edit_singleline(&mut g.name).changed();
                    if ui.button("Supprimer").clicked() { remove = Some(i); }
                });
                ui.horizontal(|ui| {
                    ui.label("GPIO"); changed |= pin_choice(ui, "gpio", &mut g.pin, &self.pinout, g.mode != PinMode::Input);
                    let before = g.mode.clone();
                    egui::ComboBox::from_id_salt("mode").selected_text(mode_label(&g.mode)).show_ui(ui, |ui| {
                        for mode in [PinMode::Input, PinMode::PullUp, PinMode::Low, PinMode::High] {
                            ui.selectable_value(&mut g.mode, mode.clone(), mode_label(&mode));
                        }
                    });
                    changed |= before != g.mode;
                });
                if matches!(g.mode, PinMode::Input | PinMode::PullUp) {
                    ui.horizontal(|ui| {
                        ui.label("Canal"); changed |= ui.add(egui::DragValue::new(&mut g.channel).range(100..=103)).changed();
                        ui.label("Visible par");
                        egui::ComboBox::from_id_salt("visibility").selected_text(visibility_label(&g.visibility)).show_ui(ui, |ui| {
                            for v in [Visibility::Admin, Visibility::Contacts, Visibility::Public] {
                                ui.selectable_value(&mut g.visibility, v.clone(), visibility_label(&v));
                            }
                        });
                    });
                } else { ui.label("Sortie locale uniquement ; aucune mesure partagée."); }
            }); });
        }
        if let Some(i) = remove { self.profile.gpio.remove(i); changed = true; }
        if ui.add_enabled(self.profile.gpio.len() < 4, egui::Button::new("+ Ajouter une broche")).clicked() {
            let channel = (100..=103).find(|c| self.profile.gpio.iter().all(|g| g.channel != *c)).unwrap_or(100);
            self.profile.gpio.push(Gpio { channel, ..Default::default() }); changed = true;
        }
        if changed { self.profile.pins_checked = false; }
        ui.separator();
        ui.label("Vérifiez le schéma de votre carte : aucune broche radio, flash, écran, batterie, USB ou de démarrage ne doit être réutilisée. Le contrôle électrique du processeur ne détecte pas tous les conflits de câblage.");
        ui.checkbox(&mut self.profile.pins_checked, "J’ai vérifié que ces broches sont libres et que le câblage est adapté.");
    }
    fn builds(&mut self, ui: &mut egui::Ui) {
        ui.heading("Mes compilations");
        if let Some(b) = &self.built {
            ui.label(format!("Dernière compilation : {}", b.id));
            ui.label(format!("{} · {}", b.profile.version, b.profile.environment));
            ui.label(format!("GPIO : {} · profil inclus dans build.json", b.profile.gpio.len()));
            ui.label(b.output.display().to_string());
            for artifact in &b.artifacts { ui.label(format!("• {artifact}")); }
        } else { ui.label("Aucun firmware compilé pour le moment. Configurez votre installation, puis lancez la compilation."); }
    }
    fn bot(&mut self, ui: &mut egui::Ui) {
        ui.heading("Bot & messages programmés");
        if ui.checkbox(&mut self.profile.bot.enabled, "Activer l’exemple facultatif de bot météo · ESP32 / BME280").changed() { self.profile.pins_checked = false; }
        ui.label("Envoi autonome par le répéteur, sans laisser l’ordinateur allumé.");
        if ui.button("Configurer la sonde et ses GPIO →").clicked() { self.tab = 2; }
        ui.horizontal(|ui| {
            ui.label("Décalage UTC (minutes)");
            ui.add(egui::DragValue::new(&mut self.profile.bot.utc_offset).range(-720..=840));
        });
        ui.label(RichText::new("Horloge du répéteur à synchroniser avant utilisation. France : +60 en hiver, +120 en été ; changement manuel.").small().color(MUTED));
        ui.label("Bilan : du début du jour à l’heure choisie. Remise à zéro quotidienne ; records absolus conservés. Aucun rattrapage des envois manqués lorsque le répéteur est éteint.");
        ui.separator();
        let mut remove = None;
        let channels = &self.profile.bot.channels;
        for (i, m) in self.profile.bot.messages.iter_mut().enumerate() {
            ui.push_id(i, |ui| { ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut m.enabled, format!("Message {}", i + 1));
                    if ui.button("Supprimer").clicked() { remove = Some(i); }
                });
                ui.horizontal(|ui| {
                    ui.label("Chaque jour à");
                    ui.add(egui::DragValue::new(&mut m.hour).range(0..=23).suffix(" h"));
                    ui.add(egui::DragValue::new(&mut m.minute).range(0..=59).suffix(" min"));
                });
                egui::ComboBox::from_id_salt("bot_destination")
                    .selected_text(channels.get(m.channel).map(|c| format!("{} · {}", c.name, c.hashtag)).unwrap_or("Choisir un canal".into()))
                    .show_ui(ui, |ui| { for (j, c) in channels.iter().enumerate() { ui.selectable_value(&mut m.channel, j, format!("{} · {}", c.name, c.hashtag)); } });
                ui.add(egui::TextEdit::multiline(&mut m.text).desired_rows(2).desired_width(f32::INFINITY));
                ui.label(RichText::new(format!("Aperçu fictif : {}", m.text.replace("{temperature}", "12.4").replace("{pression}", "1013.2").replace("{min}", "8.1").replace("{max}", "18.6").replace("{record_min}", "-3.2").replace("{record_max}", "32.7"))).small().color(MUTED));
            }); });
        }
        if let Some(i) = remove { self.profile.bot.messages.remove(i); }
        if ui.add_enabled(self.profile.bot.messages.len() < 8, egui::Button::new("+ Ajouter un message")).clicked() {
            self.profile.bot.messages.push(BotMessage::default());
        }
        ui.label("Variables : {temperature}, {pression}, {min}, {max}, {record_min}, {record_max}. Texte court : 120 octets après substitution.");
        if ui.button("Gérer les canaux publics →").clicked() { self.tab = 4; }
    }
    fn public_channels(&mut self, ui: &mut egui::Ui) {
        ui.heading("Canaux publics");
        ui.label("Donnez un nom et un hashtag à chaque destination du bot. Les destinataires doivent rejoindre exactement le même hashtag dans MeshCore.");
        ui.label(RichText::new("Ces canaux sont publics : toute personne connaissant le hashtag peut lire les rapports. Ils sont distincts des numéros de télémétrie.").color(MUTED));
        let mut remove = None;
        for (i, c) in self.profile.bot.channels.iter_mut().enumerate() {
            ui.push_id(i, |ui| { ui.group(|ui| {
                ui.label("Nom du canal"); ui.text_edit_singleline(&mut c.name);
                ui.label("Hashtag"); ui.add(egui::TextEdit::singleline(&mut c.hashtag).hint_text("#meteo"));
                let used = self.profile.bot.messages.iter().any(|m| m.channel == i);
                if ui.add_enabled(!used, egui::Button::new("Supprimer le canal")).clicked() { remove = Some(i); }
                if used { ui.label(RichText::new("Utilisé par un message : changez sa destination avant suppression.").small().color(MUTED)); }
            }); });
        }
        if let Some(i) = remove {
            self.profile.bot.channels.remove(i);
            for m in &mut self.profile.bot.messages { if m.channel > i { m.channel -= 1; } }
        }
        if ui.add_enabled(self.profile.bot.channels.len() < 8, egui::Button::new("+ Ajouter un canal public")).clicked() {
            self.profile.bot.channels.push(PublicChannel { name: "Nouveau canal".into(), hashtag: String::new() });
        }
        ui.label("Hashtag unique : # suivi de minuscules, chiffres, _ ou -. Le nom sert de repère dans le profil ; le hashtag détermine la destination réelle.");
    }
    fn flash_controls(&mut self, ui: &mut egui::Ui) {
        if let Some(b) = &self.built {
            ui.label(format!("À flasher : {} · {}", b.profile.version, b.profile.environment));
            ui.label("Flash expérimental par USB. Une erreur peut effacer les données ou rendre la carte inutilisable.");
            ui.label("Port série USB (ex. COM3)");
            if ui.text_edit_singleline(&mut self.profile.port).changed() { self.flash_confirmed = false; }
            if ui.checkbox(&mut self.profile.erase, "Effacer l’appareil avant le transfert (perte des données)").changed() {
                self.flash_confirmed = false;
            }
            ui.checkbox(&mut self.flash_confirmed, "La carte correspond à cette compilation et mes données sont sauvegardées.");
            if ui.add_enabled(self.flash_confirmed && !self.profile.port.trim().is_empty(), egui::Button::new(RichText::new("2. Flasher le firmware").strong()).min_size(egui::vec2(240.0, 42.0))).clicked() {
                let b = b.clone(); let port = self.profile.port.clone(); let erase = self.profile.erase;
                self.flash_confirmed = false;
                self.start(move |tx| firmware::flash(&b, &port, erase, tx));
            }
        } else {
            ui.add_enabled(false, egui::Button::new("2. Flasher le firmware").min_size(egui::vec2(240.0, 42.0)));
            ui.label(RichText::new("Compilez le firmware pour activer le flash.").small().color(MUTED));
        }
    }
    fn actions(&mut self, ui: &mut egui::Ui) {
                        ui.heading("Compiler et flasher");
                        ui.label("PlatformIO exécute sur Windows les scripts de compilation des sources et de leurs dépendances avec vos droits.");
                        ui.label("Exécutable PlatformIO");
                        if ui.text_edit_singleline(&mut self.profile.pio).changed() { self.build_confirmed = false; }
                        ui.label(RichText::new("RÉCAPITULATIF AVANT COMPILATION").small().color(MUTED));
                        ui.separator();
                        ui.label(format!("Version : {}", if self.profile.version.is_empty() { "à sélectionner" } else { &self.profile.version }));
                        ui.label(format!("Carte : {}", if self.profile.environment.is_empty() { "à sélectionner" } else { &self.profile.environment }));
                        ui.label(format!("{} GPIO configuré(s)", self.profile.gpio.len()));
                        if !self.profile.wiring.device.is_empty() {
                            ui.label(format!("Repère de câblage : {} (plan enregistré ; seuls les GPIO et le bot activés ci-dessous sont compilés)", self.profile.wiring.device));
                        }
                        if self.profile.bot.enabled {
                            ui.label(format!("Bot BME280 · SDA {} / SCL {} · min/max en flash", self.profile.bot.sda, self.profile.bot.scl));
                            ui.label(format!("{} rapport(s) activé(s) · {} canal(aux)", self.profile.bot.messages.iter().filter(|m| m.enabled).count(), self.profile.bot.channels.len()));
                        }
                        ui.label(if self.profile.public_telemetry { "Télémétrie publique activée" } else { "Télémétrie publique désactivée" });
                        ui.label(if self.profile.battery { "Protection batterie activée" } else { "Protection batterie standard" });
                        ui.separator();
                        let validation = self.profile.validate().and_then(|()| hardware::validate_pins(&self.profile, &self.pinout));
                        if let Err(e) = &validation { ui.label(RichText::new(e).color(Color32::from_rgb(239, 190, 109))); }
                        else { ui.label(RichText::new("Profil valide · compatibilité source vérifiée à la compilation").color(ACCENT)); }
                        ui.label(format!("Dossier source : {}", self.profile.source));
                        ui.horizontal(|ui| {
                            if ui.button("Choisir un correctif Git…").clicked() {
                                if let Some(path) = rfd::FileDialog::new().add_filter("Correctif Git", &["patch", "diff"]).pick_file() {
                                    self.profile.patch = path.to_string_lossy().into_owned();
                                    self.build_confirmed = false;
                                }
                            }
                            if !self.profile.patch.is_empty() && ui.button("Retirer le correctif").clicked() {
                                self.profile.patch.clear();
                                self.build_confirmed = false;
                            }
                        });
                        if !self.profile.patch.is_empty() { ui.label(format!("Correctif : {}", self.profile.patch)); }
                        ui.checkbox(&mut self.build_confirmed, "J’autorise l’exécution des scripts de ces sources, dépendances et correctifs de confiance.");
                        if ui.add_enabled(self.build_confirmed && validation.is_ok() && !self.profile.source.is_empty(), egui::Button::new(RichText::new("1. Compiler le firmware").strong()).min_size(egui::vec2(240.0, 42.0))).clicked() {
                            let p = self.profile.clone(); self.start(move |tx| firmware::build(p, tx));
                        }
                        self.flash_controls(ui);
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Enregistrer le profil").clicked() {
                                self.status = firmware::save(&firmware::root().join("profile.json"), &self.profile).map(|_| "Profil enregistré localement.".into()).unwrap_or_else(|e| e);
                            }
                            if ui.button("Exporter…").clicked() {
                                if let Some(path) = rfd::FileDialog::new().set_file_name("installation.json").save_file() {
                                    let profile = self.profile.clone().without_local_settings();
                                    self.status = firmware::save(&path, &profile).map(|_| "Profil exporté sans chemins ni réglages locaux.".into()).unwrap_or_else(|e| e);
                                }
                            }
                            if ui.button("Importer…").clicked() {
                                if let Some(path) = rfd::FileDialog::new().add_filter("Profil JSON", &["json"]).pick_file() {
                                    if let Some(p) = firmware::load::<Profile>(&path) {
                                        let mut profile = p.without_local_settings();
                                        profile.pio = self.profile.pio.clone();
                                        self.profile = profile;
                                        self.environments.clear();
                                        self.pinout = Default::default();
                                        self.build_confirmed = false;
                                        self.flash_confirmed = false;
                                        self.status = "Profil importé. Sélectionnez les sources, la carte et vérifiez le câblage.".into();
                                    } else { self.status = "Profil illisible ou invalide.".into(); }
                                }
                            }
                        });

    }
    fn community(ui: &mut egui::Ui) {
        ui.heading("Demandes de la communauté");
        ui.label("Sources consultées le 5 novembre 2026. Ce relevé est ciblé, pas exhaustif.");
        ui.add_space(12.0);
        ui.hyperlink_to("Capteurs et mesures sur le réseau · demande Sensor Support", "https://github.com/meshcore-dev/MeshCore/issues/41");
        ui.label("Intégré ici : contacts numériques sur GPIO, lecture à la demande. Les pilotes I²C restent ceux du firmware officiel.");
        ui.hyperlink_to("Contrôle des GPIO · discussion r/meshcore", "https://www.reddit.com/r/meshcore/comments/1sd3dvs/how_hard_is_it_to_control_gpio_via_meshcore/");
        ui.label("Intégré ici : entrées et niveaux de sortie au démarrage. Commande distante non intégrée.");
        ui.hyperlink_to("Capteurs pris en charge · discussion r/meshcore", "https://www.reddit.com/r/meshcore/comments/1o929ew/sensors_in_meshcore/");
        ui.hyperlink_to("Permissions et télémétrie · sources officielles MeshCore", "https://github.com/meshcore-dev/MeshCore/blob/main/examples/simple_repeater/MyMesh.cpp");
        ui.label("Accès par GPIO : administrateur, contacts authentifiés hors rôle invité, ou public. Les droits des contacts restent administrés dans MeshCore.");
        ui.add_space(20.0);
        ui.label("Une compilation réussie ne certifie pas la stabilité sur le matériel. Testez le démarrage, la radio, la consommation et la récupération batterie avant une installation distante.");
    }
}
fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(18.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new(format!("⚙  {title}")).size(21.0));
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(12.0);
}
fn pin_status(pin: &hardware::Pin) -> String {
    if !pin.note.is_empty() { format!("réservée / à vérifier · {}", pin.note) }
    else if pin.output { "candidate GPIO / I²C / SPI / UART TX-RX · schéma à vérifier".into() }
    else { "entrée seule / UART RX · sans pull-up interne · schéma à vérifier".into() }
}
fn lora_logo(ui: &mut egui::Ui) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(98.0, 48.0), egui::Sense::hover());
    response.on_hover_text("LoRa® · technologie radio Semtech");
    let p = ui.painter();
    p.text(rect.center() + egui::vec2(0.0, 5.0), egui::Align2::CENTER_CENTER, "LoRa", egui::FontId::proportional(32.0), Color32::WHITE);
    let center = rect.center() + egui::vec2(-8.0, 7.0);
    for radius in [21.0, 27.0] {
        let points = (0..=20).map(|i| {
            let angle = std::f32::consts::PI * (1.20 + 0.60 * i as f32 / 20.0);
            center + egui::vec2(angle.cos(), angle.sin()) * radius
        }).collect::<Vec<_>>();
        p.add(egui::Shape::line(points, egui::Stroke::new(1.6, Color32::WHITE)));
    }
    p.text(rect.right_top() + egui::vec2(-3.0, 10.0), egui::Align2::RIGHT_TOP, "®", egui::FontId::proportional(10.0), Color32::WHITE);
}
fn pin_choice(ui: &mut egui::Ui, id: &str, value: &mut u8, pinout: &hardware::Pinout, output: bool) -> bool {
    let before = *value;
    egui::ComboBox::from_id_salt(id).selected_text(format!("GPIO {}", *value)).width(150.0).show_ui(ui, |ui| {
        for pin in &pinout.pins {
            ui.add_enabled_ui(pin.available() && (!output || pin.output), |ui| { ui.selectable_value(value, pin.number, format!("GPIO {} · {}", pin.number, pin_status(pin))); });
        }
        if pinout.pins.is_empty() { ui.label("Charger une carte avec un brochage documenté."); }
    });
    before != *value
}
fn mode_label(mode: &PinMode) -> &'static str { match mode { PinMode::Input => "Entrée numérique", PinMode::PullUp => "Entrée avec pull-up", PinMode::Low => "Sortie à 0 au démarrage", PinMode::High => "Sortie à 1 au démarrage" } }
fn visibility_label(v: &Visibility) -> &'static str { match v { Visibility::Admin => "Administrateur", Visibility::Contacts => "Contacts autorisés", Visibility::Public => "Tout le monde" } }

impl eframe::App for Studio {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if let Some(path) = self.screenshot.clone() {
            self.frames += 1;
            ui.ctx().request_repaint();
            if self.frames == 120 { ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default())); }
            let capture = ui.input(|i| i.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event { Some(image.clone()) } else { None }
            }));
            if let Some(capture) = capture {
                let pixels: Vec<u8> = capture.pixels.iter().flat_map(|pixel| pixel.to_array()).collect();
                match image::save_buffer(&path, &pixels, capture.size[0] as u32, capture.size[1] as u32, image::ColorType::Rgba8) {
                    Ok(()) => self.screenshot = None,
                    Err(error) => eprintln!("Capture impossible : {error}"),
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.poll();
        if self.task.is_some() { ui.ctx().request_repaint_after(std::time::Duration::from_millis(150)); }
        egui::Panel::top("brand").resizable(false).exact_size(58.0)
            .frame(egui::Frame::NONE).show(ui, |ui| {
                let rect = ui.max_rect();
                let mut mesh = egui::Mesh::default();
                mesh.colored_vertex(rect.left_top(), Color32::from_rgb(153, 194, 100));
                mesh.colored_vertex(rect.right_top(), Color32::from_rgb(65, 95, 185));
                mesh.colored_vertex(rect.right_bottom(), Color32::from_rgb(65, 95, 185));
                mesh.colored_vertex(rect.left_bottom(), Color32::from_rgb(153, 194, 100));
                mesh.add_triangle(0, 1, 2); mesh.add_triangle(0, 2, 3);
                ui.painter().add(egui::Shape::mesh(mesh));
                ui.horizontal_centered(|ui| {
                    ui.add_space(20.0);
                    lora_logo(ui);
                    ui.label(RichText::new("Configurateur").size(29.0).color(Color32::WHITE));
                    ui.label(RichText::new("MeshCore").size(14.0).color(Color32::WHITE));
                });
            });
        egui::Panel::left("navigation").resizable(false).exact_size(190.0)
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(8.0)).show(ui, |ui| {
                ui.add_space(10.0);
                for (i, label) in [(0, "Configurateur"), (1, "Options du firmware"), (2, "GPIO & UART"),
                    (3, "Bot & messages"), (4, "Canaux publics"), (5, "Compiler et flasher"),
                    (7, "Logs"), (6, "Support")]
                {
                    if ui.add_sized([174.0, 44.0], egui::Button::selectable(self.tab == i, label)).clicked() { self.tab = i; }
                }
            });
        egui::Panel::bottom("status").resizable(false).show(ui, |ui| {
            ui.horizontal(|ui| { if self.task.is_some() { ui.spinner(); } ui.label(&self.status); });
        });
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(BACKGROUND)).show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            let width = (ui.available_width() * 0.84).min(1000.0);
            let margin = (ui.available_width() - width) / 2.0;
            ui.horizontal_top(|ui| {
                ui.add_space(margin);
                egui::Frame::new().fill(SURFACE).inner_margin(16.0).show(ui, |ui| {
                    ui.set_width(width - 32.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::ScrollArea::vertical().id_salt(("page", self.tab)).show(ui, |ui| {
                        ui.add_enabled_ui(self.task.is_none(), |ui| {
                            match self.tab {
                                0 => { self.sources(ui); self.options(ui); },
                                1 => self.options(ui),
                                2 => self.gpio(ui),
                                3 => self.bot(ui),
                                4 => self.public_channels(ui),
                                5 => { self.actions(ui); ui.add_space(24.0); self.builds(ui); },
                                7 => {},
                                _ => Self::community(ui),
                            }
                        });
                        if self.tab == 0 {
                            ui.add_space(20.0);
                            if ui.button("Continuer : compiler et flasher →").clicked() { self.tab = 5; }
                        }
                        if self.tab == 7 || self.tab == 5 {
                            section(ui, "Journal de compilation et de transfert");
                            if self.logs.is_empty() { ui.label(RichText::new("Les opérations et leurs erreurs apparaîtront ici.").color(MUTED)); }
                            for line in &self.logs { ui.monospace(line); }
                        }
                    });
                    });
                });
            });
        });
    }
}
