use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};
use std::{io::{BufRead, BufReader}, process::{Command, Stdio}, sync::mpsc::Sender, time::{SystemTime, UNIX_EPOCH}};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum PinMode { Input, PullUp, Low, High }
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Visibility { Admin, Contacts, Public }
#[derive(Clone, Serialize, Deserialize)]
pub struct Gpio {
    pub name: String,
    pub pin: u8,
    pub mode: PinMode,
    pub channel: u8,
    pub visibility: Visibility,
}
impl Default for Gpio {
    fn default() -> Self { Self { name: "Contact".into(), pin: 4, mode: PinMode::Input, channel: 100, visibility: Visibility::Admin } }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PublicChannel { pub name: String, pub hashtag: String }
impl Default for PublicChannel {
    fn default() -> Self { Self { name: "Météo locale".into(), hashtag: "#meteo".into() } }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct BotMessage {
    pub enabled: bool, pub hour: u8, pub minute: u8, pub channel: usize, pub text: String,
}
impl Default for BotMessage {
    fn default() -> Self { Self { enabled: true, hour: 7, minute: 0, channel: 0,
        text: "Ce matin : {temperature} °C, {pression} hPa".into() } }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Bot {
    pub enabled: bool, pub sda: u8, pub scl: u8, pub address: u8, pub utc_offset: i16,
    pub channels: Vec<PublicChannel>, pub messages: Vec<BotMessage>,
}
impl Default for Bot {
    fn default() -> Self { Self { enabled: false, sda: 4, scl: 5, address: 0x76, utc_offset: 60,
        channels: vec![PublicChannel::default()], messages: vec![BotMessage::default(), BotMessage {
            hour: 21, text: "Bilan du jour : min {min} °C / max {max} °C".into(), ..Default::default()
        }] } }
}
pub fn render_bot_message(text: &str) -> String {
    ["temperature", "pression", "min", "max", "record_min", "record_max"].iter()
        .fold(text.to_owned(), |s, key| s.replace(&format!("{{{key}}}"), "-1234.5"))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub version: String,
    pub environment: String,
    pub source: String,
    pub battery: bool,
    pub low_mv: u16,
    pub resume_mv: u16,
    pub interval: u32,
    pub public_telemetry: bool,
    pub channels: [bool; 4],
    pub quiet: bool,
    pub advertise: bool,
    pub patch: String,
    pub pio: String,
    pub port: String,
    pub erase: bool,
    pub gpio: Vec<Gpio>,
    pub pins_checked: bool,
    pub bot: Bot,
    pub wiring: crate::hardware::Wiring,
}
impl Default for Profile {
    fn default() -> Self { Self {
        version: String::new(), environment: String::new(), source: String::new(),
        battery: false, low_mv: 3200, resume_mv: 3600, interval: 60,
        public_telemetry: false, channels: [true; 4], quiet: true, advertise: true,
        patch: String::new(), pio: "pio".into(), port: String::new(), erase: false,
        gpio: Vec::new(), pins_checked: false, bot: Bot::default(), wiring: Default::default(),
    }}
}
impl Profile {
    pub fn validate(&self) -> Result<()> {
        if self.environment.is_empty() { return Err("Choisissez une carte après avoir chargé les sources.".into()); }
        if !self.environment.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
            || !self.environment.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)) {
            return Err("Identifiant de carte invalide.".into());
        }
        if self.battery && (!(2500..=4500).contains(&self.low_mv) || self.resume_mv <= self.low_mv + 100 || self.resume_mv > 4500 || !(10..=3600).contains(&self.interval)) {
            return Err("Seuils invalides : 2,5 à 4,5 V, reprise > seuil bas + 0,1 V ; intervalle de 10 à 3600 s.".into());
        }
        if self.public_telemetry && !self.channels.contains(&true)
            && !self.gpio.iter().any(|g| matches!(g.mode, PinMode::Input | PinMode::PullUp) && g.visibility == Visibility::Public) {
            return Err("Sélectionnez au moins un canal ou un GPIO public.".into());
        }
        if !self.gpio.is_empty() {
            if !self.environment.to_lowercase().ends_with("_repeater") || !self.pins_checked {
                return Err("GPIO : cible ESP32 répéteur requise ; vérifiez les broches libres sur le schéma et confirmez le câblage.".into());
            }
            if !self.patch.is_empty() { return Err("GPIO et correctif libre ne peuvent pas être combinés : conflits non vérifiables.".into()); }
            if self.gpio.len() > 4 { return Err("Quatre GPIO au maximum par installation.".into()); }
            for (i, g) in self.gpio.iter().enumerate() {
                if g.name.trim().is_empty() || g.pin > 48 || g.channel < 100 || g.channel > 103 {
                    return Err("GPIO : nom requis, broche 0–48 et canal réservé 100–103.".into());
                }
                if self.gpio[..i].iter().any(|other| other.pin == g.pin || other.channel == g.channel) {
                    return Err("Chaque GPIO et chaque canal doivent être uniques.".into());
                }
                if matches!(g.mode, PinMode::Input | PinMode::PullUp) && g.visibility == Visibility::Public && !self.public_telemetry {
                    return Err("Activez la télémétrie publique pour partager un GPIO publiquement.".into());
                }
            }
        }
        if self.bot.enabled {
            let b = &self.bot;
            if !self.environment.to_lowercase().ends_with("_repeater") || !self.pins_checked || !self.patch.is_empty() {
                return Err("Bot météo : répéteur ESP32, câblage confirmé et aucun correctif libre requis.".into());
            }
            if b.sda > 48 || b.scl > 48 || b.sda == b.scl || ![0x76, 0x77].contains(&b.address)
                || self.gpio.iter().any(|g| g.pin == b.sda || g.pin == b.scl) {
                return Err("BME280 : GPIO SDA/SCL distincts et libres (0–48), adresse 0x76 ou 0x77.".into());
            }
            if !(-720..=840).contains(&b.utc_offset) || b.channels.is_empty() || b.channels.len() > 8 || b.messages.is_empty() || b.messages.len() > 8 {
                return Err("Bot : 1 à 8 canaux et messages, décalage UTC de −720 à +840 minutes.".into());
            }
            for (i, c) in b.channels.iter().enumerate() {
                if c.name.trim().is_empty() || c.name.len() > 48 || !c.hashtag.starts_with('#') || c.hashtag.len() < 2 || c.hashtag.len() > 32
                    || !c.hashtag[1..].chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
                    || b.channels[..i].iter().any(|other| other.hashtag == c.hashtag) {
                    return Err("Canal : nom requis (48 octets max), hashtag unique comme #meteo (minuscules, chiffres, _ ou -, 32 caractères max).".into());
                }
            }
            for m in &b.messages {
                let rendered = render_bot_message(&m.text);
                if m.hour > 23 || m.minute > 59 || m.channel >= b.channels.len() || m.text.trim().is_empty()
                    || m.text.chars().any(char::is_control) || rendered.len() > 120 || rendered.contains(['{', '}']) {
                    return Err("Message : horaire et canal valides, texte court (120 octets après substitution), variables connues uniquement.".into());
                }
            }
        }
        Ok(())
    }
}
pub fn root() -> PathBuf {
    if let Some(local_data) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).filter(|path| path.is_absolute()) {
        return local_data.join("MeshStudio");
    }
    std::env::current_exe().ok().and_then(|path| path.parent().map(Path::to_owned))
        .unwrap_or_else(std::env::temp_dir).join("MeshStudio")
}
pub fn load<T: for<'a> Deserialize<'a>>(path: &Path) -> Option<T> { serde_json::from_slice(&fs::read(path).ok()?).ok() }
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(p) = path.parent() { fs::create_dir_all(p).map_err(|e| e.to_string())?; }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, bytes).map_err(|e| e.to_string())
}
pub fn environments(source: &Path) -> Result<Vec<String>> {
    let mut files = vec![source.join("platformio.ini")];
    let variants = fs::read_dir(source.join("variants")).map_err(|e| format!("Sources MeshCore incomplètes : {e}"))?;
    for entry in variants { files.push(entry.map_err(|e| e.to_string())?.path().join("platformio.ini")); }
    let mut names = Vec::new();
    for path in files {
        if !path.is_file() { continue; }
        for line in fs::read_to_string(path).map_err(|e| e.to_string())?.lines() {
            if let Some(name) = line.trim().strip_prefix("[env:").and_then(|s| s.strip_suffix(']')) {
                if !name.starts_with("native") { names.push(name.to_owned()); }
            }
        }
    }
    names.sort(); names.dedup();
    if names.is_empty() { return Err("Aucune cible matérielle trouvée dans ces sources.".into()); }
    Ok(names)
}

pub fn release_parts(tag: &str) -> Option<(&str, &str)> {
    let (kind, version) = tag.rsplit_once("-v").or_else(|| tag.strip_prefix('v').map(|v| ("", v)))?;
    if !version.starts_with(|c: char| c.is_ascii_digit()) { return None; }
    let role = match kind {
        "room" | "room-server" => "Serveur de room",
        "rep" | "repeater" => "Répéteur",
        "companion" => "Compagnon",
        "" => "Standard",
        other => other,
    };
    Some((version, role))
}
pub fn environment_matches_release(environment: &str, tag: &str) -> bool {
    let env = environment.to_lowercase();
    match release_parts(tag).map(|(_, role)| role) {
        Some("Serveur de room") => env.contains("_room"),
        Some("Répéteur") => env.contains("_repeater") || env.contains("_repeatr"),
        Some("Compagnon") => env.contains("_companion") || env.contains("_comp_radio"),
        Some("Standard") | None => true,
        Some(role) => env.contains(&format!("_{}", role.replace('-', "_"))),
    }
}
pub fn board_name(environment: &str) -> &str {
    let lower = environment.to_lowercase();
    let end = ["_companion", "_comp_radio", "_repeater", "_repeatr", "_room_", "_terminal_", "_sensor", "_kiss_"]
        .iter().filter_map(|role| lower.find(role)).min().unwrap_or(environment.len());
    &environment[..end]
}
pub fn connection(environment: &str) -> &'static str {
    let lower = environment.trim_end_matches('_').to_lowercase();
    if lower.ends_with("_wifi") { "Wi-Fi" }
    else if lower.ends_with("_ble") { "Bluetooth" }
    else if lower.ends_with("_usb") || lower.ends_with("_serial") { "USB / série" }
    else { "Autre rôle (sans interface compagnon)" }
}
pub fn board_matches(board: &str, query: &str) -> bool {
    let normalize = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect::<String>();
    normalize(board).contains(&normalize(query))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Build {
    pub id: String,
    pub profile: Profile,
    pub work: PathBuf,
    pub output: PathBuf,
    pub artifacts: Vec<String>,
}
pub enum Event {
    Log(String),
    Versions(Vec<String>),
    Sources(PathBuf, Vec<String>),
    Pinout(crate::hardware::Pinout),
    Built(Build),
    Done(std::result::Result<String, String>),
}
impl Profile {
    pub fn without_local_settings(mut self) -> Self {
        self.source.clear();
        self.patch.clear();
        self.pio = Self::default().pio;
        self.port.clear();
        self.erase = false;
        self.pins_checked = false;
        self
    }
}

fn tool_path(executable: &str) -> Result<PathBuf> {
    let path = Path::new(executable);
    if path.is_absolute() {
        #[cfg(windows)]
        if !path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("exe")) {
            return Err("Sélectionnez un exécutable .exe installé localement.".into());
        }
        return path.is_file().then(|| path.to_owned())
            .ok_or_else(|| format!("Exécutable introuvable : {executable}"));
    }
    if !matches!(executable, "git" | "pio") {
        return Err("Indiquez le chemin absolu de PlatformIO ou utilisez pio depuis le PATH.".into());
    }
    let name = format!("{executable}{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(&name))
        .find(|file| file.is_file())
        .ok_or_else(|| format!("{name} est introuvable dans le PATH."))
}

pub fn powershell() -> Result<Command> {
    let root = std::env::var_os("SystemRoot").ok_or("Dossier système Windows introuvable.")?;
    let path = PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    command(path.to_str().ok_or("Chemin de PowerShell invalide.")?)
}

fn command(executable: &str) -> Result<Command> {
    let executable = tool_path(executable)?;
    let mut c = Command::new(executable);
    #[cfg(windows)] c.creation_flags(0x08000000);
    c.env("GIT_TERMINAL_PROMPT", "0");
    Ok(c)
}
pub fn run(mut cmd: Command, tx: &Sender<Event>) -> Result<String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    let mut child = cmd.spawn().map_err(|e| format!("Impossible de démarrer l'outil : {e}. Vérifiez Git / PlatformIO dans les réglages."))?;
    let err = child.stderr.take().unwrap();
    let sender = tx.clone();
    let errors = std::thread::spawn(move || {
        let mut tail = String::new();
        for line in BufReader::new(err).lines().map_while(std::result::Result::ok) {
            let _ = sender.send(Event::Log(line.clone()));
            if tail.len() > 12000 { tail.clear(); }
            tail.push_str(&line); tail.push('\n');
        }
        tail
    });
    let mut output = String::new();
    for line in BufReader::new(child.stdout.take().unwrap()).lines().map_while(std::result::Result::ok) {
        let _ = tx.send(Event::Log(line.clone()));
        if output.len() < 2_000_000 { output.push_str(&line); output.push('\n'); }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let tail = errors.join().unwrap_or_default();
    if !status.success() { return Err(format!("L'outil a échoué ({status}).\n{tail}")); }
    Ok(output)
}
fn stamp() -> String { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos().to_string() }

pub fn sync_versions(tx: &Sender<Event>) -> Result<String> {
    // The fixed PowerShell script keeps network access out of the UI thread.
    let script = "$ErrorActionPreference='Stop'; [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; $tags=@(); for($page=1;$page -le 100;$page++){ $batch=@(Invoke-RestMethod -Uri ('https://api.github.com/repos/meshcore-dev/MeshCore/tags?per_page=100&page='+$page) -Headers @{'User-Agent'='Atelier-Mesh'} -TimeoutSec 20); $tags+=@($batch | ForEach-Object {$_.name}); if($batch.Count -lt 100){ConvertTo-Json -InputObject @($tags) -Compress; exit 0} }; throw 'Catalogue trop grand : synchronisation incomplète'";
    let mut cmd = powershell()?;
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    let output = run(cmd, tx)?;
    let versions: Vec<String> = serde_json::from_str(output.trim_start_matches('\u{feff}').trim()).map_err(|e| format!("Réponse GitHub invalide : {e}"))?;
    if versions.is_empty() { return Err("GitHub n'a renvoyé aucune version.".into()); }
    save(&root().join("versions.json"), &versions)?;
    let n = versions.len();
    let _ = tx.send(Event::Versions(versions));
    Ok(format!("{n} versions synchronisées depuis le dépôt officiel."))
}
pub fn fetch_source(version: &str, tx: &Sender<Event>) -> Result<String> {
    if release_parts(version).is_none() || !version.starts_with(|c: char| c.is_ascii_alphanumeric())
        || !version.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) {
        return Err("Sélectionnez une version officielle.".into());
    }
    let dest = root().join("sources").join(stamp());
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut cmd = command("git")?;
    cmd.args(["-c", "http.sslVerify=true", "-c", "http.followRedirects=false", "-c", "http.lowSpeedLimit=100", "-c", "http.lowSpeedTime=20", "clone", "--depth", "1", "--branch", version, "https://github.com/meshcore-dev/MeshCore.git"]).arg(&dest);
    cmd.env_remove("GIT_SSL_NO_VERIFY").env("GIT_ALLOW_PROTOCOL", "https");
    run(cmd, tx)?;
    let envs = environments(&dest)?;
    let _ = tx.send(Event::Sources(dest, envs));
    Ok("Sources chargées. Choisissez la cible de votre appareil.".into())
}
fn copy_sources(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        if [".git", ".pio", "out", "platformio.local.ini"].iter().any(|s| name == *s) { continue; }
        let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        let ty = metadata.file_type();
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err("Sources avec point de réanalyse Windows (lien ou jonction) : copie refusée.".into());
            }
        }
        if ty.is_symlink() { return Err("Sources avec lien symbolique : import refusé pour protéger les fichiers externes.".into()); }
        let dest = to.join(name);
        if ty.is_dir() { copy_sources(&entry.path(), &dest)?; }
        else if ty.is_file() { fs::copy(entry.path(), dest).map_err(|e| e.to_string())?; }
    }
    Ok(())
}
fn replace_once(text: &str, anchor: &str, replacement: &str) -> Result<String> {
    if text.matches(anchor).count() != 1 { return Err(format!("Modification incompatible avec cette version : point d'insertion absent ou ambigu ({anchor}). Aucun firmware créé.")); }
    Ok(text.replacen(anchor, replacement, 1))
}
fn write_text(path: &Path, text: &str) -> Result<()> { fs::write(path, text).map_err(|e| e.to_string()) }

fn patch_gpio(work: &Path, p: &Profile) -> Result<()> {
    let main = work.join("examples/simple_repeater/main.cpp");
    let mesh = work.join("examples/simple_repeater/MyMesh.cpp");
    let main_text = fs::read_to_string(&main).map_err(|e| e.to_string())?.replace("\r\n", "\n");
    let mesh_text = fs::read_to_string(&mesh).map_err(|e| e.to_string())?.replace("\r\n", "\n");
    // shortcut: board wiring needs a schematic check; add board-tested pin maps before automatic approval.
    let mut boot = String::from("board.begin();\n#if !defined(ESP32)\n#error Atelier_GPIO_requires_ESP32\n#else\n");
    let mut readings = String::new();
    for g in &p.gpio {
        let pin = g.pin;
        let output = matches!(g.mode, PinMode::Low | PinMode::High);
        boot.push_str(&format!("  static_assert(GPIO_IS_VALID_{}({pin}), \"GPIO incompatible\");\n", if output { "OUTPUT_GPIO" } else { "GPIO" }));
        if output { boot.push_str(&format!("  digitalWrite({pin}, {});\n", if g.mode == PinMode::High { "HIGH" } else { "LOW" })); }
        boot.push_str(&format!("  pinMode({pin}, {});\n", match g.mode { PinMode::Input => "INPUT", PinMode::PullUp => "INPUT_PULLUP", _ => "OUTPUT" }));
        if !output {
            let condition = match g.visibility {
                Visibility::Admin => "sender->isAdmin()",
                Visibility::Contacts => "(sender->permissions & PERM_ACL_ROLE_MASK) != PERM_ACL_GUEST",
                Visibility::Public => "true",
            };
            readings.push_str(&format!("    if ({condition}) telemetry.addDigitalInput({}, digitalRead({pin}) == HIGH ? 1 : 0);\n", g.channel));
        }
    }
    boot.push_str("#endif");
    let main_text = replace_once(&main_text, "board.begin();", &boot)?;
    let main_text = format!("#if defined(ESP32)\n#include <driver/gpio.h>\n#endif\n{main_text}");
    // Insert before the optional public-channel filter so private records never enter its response.
    let mesh_text = replace_once(&mesh_text, "uint8_t tlen = telemetry.getSize();", &format!("{readings}uint8_t tlen = telemetry.getSize();"))?;
    write_text(&main, &main_text)?;
    write_text(&mesh, &mesh_text)
}

fn patch_bot(work: &Path, p: &Profile) -> Result<()> {
    let role = work.join("examples/simple_repeater");
    let main = fs::read_to_string(role.join("main.cpp")).map_err(|e| e.to_string())?.replace("\r\n", "\n");
    let header = fs::read_to_string(role.join("MyMesh.h")).map_err(|e| e.to_string())?.replace("\r\n", "\n");
    let send = r#"void sendNodeDiscoverReq();
  bool sendAtelierBot(uint32_t now, const char* hashtag, const char* text) {
    mesh::GroupChannel channel = {};
    mesh::Utils::sha256(channel.secret, 16, (const uint8_t*)hashtag, strlen(hashtag));
    mesh::Utils::sha256(channel.hash, sizeof(channel.hash), channel.secret, 16);
    uint8_t payload[5 + 7 + 120];
    memcpy(payload, &now, 4); payload[4] = TXT_TYPE_PLAIN;
    memcpy(payload + 5, "Meteo: ", 7);
    size_t len = strlen(text); if (len > 120) return false;
    memcpy(payload + 12, text, len);
    auto packet = createGroupDatagram(PAYLOAD_TYPE_GRP_TXT, channel, payload, 12 + len);
    if (!packet) return false;
    sendFloodScoped(default_scope, packet, 0, _prefs.path_hash_mode + 1); return true;
  }"#;
    let header = replace_once(&header, "void sendNodeDiscoverReq();", send)?;
    let main = replace_once(&main, "#include \"MyMesh.h\"", "#include \"MyMesh.h\"\n#include \"AtelierBot.h\"")?;
    let main = replace_once(&main, "the_mesh.begin(fs);", "the_mesh.begin(fs);\n  AtelierBot::begin();")?;
    let main = replace_once(&main, "the_mesh.loop();", "the_mesh.loop();\n  AtelierBot::loop(the_mesh, rtc_clock.getCurrentTime());")?;
    let b = &p.bot;
    let mut config = format!("#pragma once\nnamespace AtelierBot {{\nconstexpr int SDA_PIN={}, SCL_PIN={}, ADDRESS={}, UTC_OFFSET={};\n", b.sda, b.scl, b.address, b.utc_offset);
    config.push_str("static const char* hashtags[] = {\n");
    for c in &b.channels { config.push_str(&format!("{},\n", serde_json::to_string(&c.hashtag).unwrap())); }
    config.push_str("};\nstruct Message { bool enabled; unsigned minute; unsigned channel; const char* text; };\nstatic const Message messages[] = {\n");
    for m in &b.messages {
        config.push_str(&format!("{{{}, {}, {}, {}}},\n", m.enabled, u16::from(m.hour) * 60 + u16::from(m.minute), m.channel, serde_json::to_string(&m.text).unwrap()));
    }
    config.push_str("};\n}\n");
    // Reuse the BME280 dependency declared by the official sensor configuration.
    let ini_path = work.join("platformio.ini");
    let ini = fs::read_to_string(&ini_path).map_err(|e| e.to_string())?.replace("\r\n", "\n");
    let ini = replace_once(&ini, "lib_deps =\n  SPI", "lib_deps =\n  adafruit/Adafruit BME280 Library @ ^2.3.0\n  SPI")?;
    write_text(&ini_path, &ini)?;
    write_text(&role.join("AtelierBotConfig.h"), &config)?;
    write_text(&role.join("AtelierBot.h"), include_str!("atelier_bot.h"))?;
    write_text(&role.join("main.cpp"), &main)?;
    write_text(&role.join("MyMesh.h"), &header)
}

fn patch_sources(work: &Path, p: &Profile) -> Result<()> {
    if p.bot.enabled { patch_bot(work, p)?; }
    if !p.gpio.is_empty() { patch_gpio(work, p)?; }
    if p.battery {
        // shortcut: ESP32 only; add board-tested recovery adapters before supporting other chips.
        let candidates = ["simple_repeater", "companion_radio", "simple_room_server", "simple_sensor"];
        let mut found = false;
        for role in candidates {
            let path = work.join(format!("examples/{role}/main.cpp"));
            if !path.is_file() { continue; }
            let text = fs::read_to_string(&path).map_err(|e| e.to_string())?.replace("\r\n", "\n");
            if !text.contains("board.begin();") { continue; }
            let declarations = "\n#if !defined(ESP32)\n#error Atelier_Mesh_battery_recovery_requires_ESP32\n#else\n#include <esp_sleep.h>\n#include <esp_attr.h>\nRTC_DATA_ATTR static bool atelier_low_battery = false;\n#endif\n";
            let guard = format!("board.begin();\n#if defined(ESP32)\n  uint32_t atelier_mv = 0;\n  for (int i = 0; i < 5; ++i) {{ atelier_mv += board.getBattMilliVolts(); delay(50); }}\n  atelier_mv /= 5;\n  if (atelier_mv > 0 && atelier_mv <= {}) atelier_low_battery = true;\n  if (atelier_low_battery && atelier_mv < {}) {{\n    esp_sleep_enable_timer_wakeup({}ULL * 1000000ULL);\n    esp_deep_sleep_start();\n  }}\n  atelier_low_battery = false;\n#endif", p.low_mv, p.resume_mv, p.interval);
            let text = replace_once(&text, "board.begin();", &guard)?;
            // The RTC latch survives timed deep sleep and prevents rapid restart cycles.
            let loop_hook = format!("void loop() {{\n#if defined(ESP32)\n  static uint32_t atelier_check = millis();\n  if ((uint32_t)(millis() - atelier_check) >= 60000) {{\n    atelier_check = millis();\n    uint16_t mv = board.getBattMilliVolts();\n    if (mv > 0 && mv <= {}) {{\n      atelier_low_battery = true;\n      esp_sleep_enable_timer_wakeup({}ULL * 1000000ULL);\n      esp_deep_sleep_start();\n    }}\n  }}\n#endif", p.low_mv, p.interval);
            let text = replace_once(&text, "void loop() {", &loop_hook)?;
            // Includes must precede the declarations that use the ESP32 attributes.
            let text = replace_once(&text, "void setup() {", &format!("{declarations}\nvoid setup() {{"))?;
            write_text(&path, &text)?;
            found = true;
        }
        if !found { return Err("Cette version ne possède pas de point d'insertion batterie compatible.".into()); }
    }
    if p.public_telemetry {
        if !p.environment.to_lowercase().contains("repeater") { return Err("La télémétrie publique intégrée est actuellement compatible avec le rôle répéteur uniquement. Un correctif personnalisé peut adapter un autre rôle.".into()); }
        let path = work.join("examples/simple_repeater/MyMesh.cpp");
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?.replace("\r\n", "\n");
        let text = replace_once(&text, "perm_mask = 0x00;", "perm_mask = 0xFF;")?;
        let channels: String = p.channels.iter().enumerate().filter(|(_, on)| **on).map(|(i,_)| format!("channel == {}", i+1))
            .chain(p.gpio.iter().filter(|g| matches!(g.mode, PinMode::Input | PinMode::PullUp) && g.visibility == Visibility::Public).map(|g| format!("channel == {}", g.channel))).collect::<Vec<_>>().join(" || ");
        // Only standard CayenneLPP records with a known length are exposed. Unknown data fails closed.
        let filter = format!(r#"uint8_t tlen = telemetry.getSize();
    if ((sender->permissions & PERM_ACL_ROLE_MASK) == PERM_ACL_GUEST) {{
      const uint8_t* bytes = telemetry.getBuffer();
      size_t out = 0;
      for (uint16_t pos = 0; pos + 2 <= tlen;) {{
        uint8_t channel = bytes[pos], type = bytes[pos + 1];
        uint8_t size = 0;
        switch (type) {{
          case 0: case 1: case 102: case 104: size = 1; break;
          case 2: case 3: case 101: case 103: case 115: case 116: size = 2; break;
          case 113: case 134: size = 6; break;
          case 136: size = 9; break;
          default: return 0;
        }}
        if (pos + 2 + size > tlen) return 0;
        // GPS coordinates are never sent to guests, even on a selected channel.
        if (type != 136 && ({channels})) {{
          if (4u + out + size + 2u > sizeof(reply_data)) return 0;
          memcpy(&reply_data[4 + out], bytes + pos, size + 2); out += size + 2;
        }}
        pos += size + 2;
      }}
      return 4 + out;
    }}"#);
        let text = replace_once(&text, "uint8_t tlen = telemetry.getSize();", &filter)?;
        write_text(&path, &text)?;
    }
    Ok(())
}

pub fn build(p: Profile, tx: &Sender<Event>) -> Result<String> {
    p.validate()?;
    crate::hardware::validate_pins(&p, &crate::hardware::local(&p))?;
    let source = PathBuf::from(&p.source);
    let envs = environments(&source)?;
    if !envs.contains(&p.environment) { return Err("Cette cible n'existe pas dans les sources chargées.".into()); }
    let id = stamp();
    // Sources supplied by the user stay untouched; every build gets an isolated copy.
    let canonical = source.canonicalize().map_err(|e| e.to_string())?;
    let builds = root().join("builds");
    fs::create_dir_all(&builds).map_err(|e| e.to_string())?;
    if builds.canonicalize().map_err(|e| e.to_string())?.starts_with(&canonical) {
        return Err("Choisissez un dossier de sources distinct du dossier de travail de l'application.".into());
    }
    let build_dir = builds.join(&id);
    fs::create_dir(&build_dir).map_err(|e| e.to_string())?;
    let work = build_dir.join("sources");
    let _ = tx.send(Event::Log("Copie isolée des sources et vérification des modifications…".into()));
    copy_sources(&canonical, &work)?;
    patch_sources(&work, &p)?;
    if !p.patch.is_empty() {
        let path = PathBuf::from(&p.patch).canonicalize().map_err(|e| e.to_string())?;
        apply_patch(&work, &path, tx)?;
    }
    run(platformio(&p, &work)?, tx)?;
    let output = build_dir.join("firmware");
    fs::create_dir_all(&output).map_err(|e| e.to_string())?;
    let mut artifacts = Vec::new();
    let built_dir = work.join(".pio/build").join(&p.environment);
    for entry in fs::read_dir(&built_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()).is_some_and(|e| ["bin","uf2","hex"].contains(&e)) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 0 {
                fs::copy(path, output.join(&name)).map_err(|e| e.to_string())?;
                artifacts.push(name);
            }
        }
    }
    if !artifacts.iter().any(|a| a.starts_with("firmware")) { return Err("La compilation n'a produit aucun firmware exploitable. Consultez le journal.".into()); }
    let receipt = Build { id, profile: p, work, output: output.clone(), artifacts };
    save(&output.join("build.json"), &receipt)?;
    save(&root().join("last_build.json"), &receipt)?;
    let _ = tx.send(Event::Built(receipt));
    Ok("Firmware compilé. Les fichiers et le profil sont disponibles dans Mes compilations.".into())
}

fn apply_patch(work: &Path, patch: &Path, tx: &Sender<Event>) -> Result<()> {
    let parent = work.parent().ok_or("Dossier de compilation invalide.")?;
    let mut cmd = command("git")?;
    // Git must apply relative to the copied sources, even if the configurator is in a Git clone.
    cmd.current_dir(work).env("GIT_CEILING_DIRECTORIES", parent)
        .env_remove("GIT_DIR").env_remove("GIT_WORK_TREE")
        .arg("apply").arg(patch);
    run(cmd, tx)?;
    Ok(())
}

fn platformio(p: &Profile, work: &Path) -> Result<Command> {
    let mut cmd = command(&p.pio)?;
    cmd.current_dir(work).args(["run", "-e", &p.environment]);
    let mut unflags = String::from("-DENABLE_ADVERT_ON_BOOT");
    let mut flags = format!("-DENABLE_ADVERT_ON_BOOT={}", u8::from(p.advertise));
    if p.quiet {
        unflags.push_str(" -DMESH_DEBUG -DMESH_PACKET_LOGGING -DBLE_DEBUG_LOGGING");
        flags.push_str(" -DMESH_DEBUG=0 -DMESH_PACKET_LOGGING=0 -DBLE_DEBUG_LOGGING=0");
    }
    cmd.env("PLATFORMIO_BUILD_UNFLAGS", unflags).env("PLATFORMIO_BUILD_SRC_FLAGS", flags);
    Ok(cmd)
}

pub fn flash(b: &Build, port: &str, erase: bool, tx: &Sender<Event>) -> Result<String> {
    let port = port.trim();
    let port_number = port.get(..3).filter(|prefix| prefix.eq_ignore_ascii_case("COM"))
        .and_then(|_| port.get(3..));
    if !port_number.is_some_and(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        && number.parse::<u16>().is_ok_and(|value| value > 0)) {
        return Err("Indiquez un port série Windows, par exemple COM3.".into());
    }
    if b.id.is_empty() || !b.id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("Identifiant de compilation invalide. Recompilez le firmware.".into());
    }
    let p = &b.profile;
    p.validate()?;
    let build_dir = root().join("builds").join(&b.id);
    let work = build_dir.join("sources");
    if b.work != work || b.output != build_dir.join("firmware") || !work.join("platformio.ini").is_file() {
        return Err("Les sources de cette compilation ne sont plus disponibles. Recompilez le firmware.".into());
    }
    let compiled = work.join(".pio/build").join(&p.environment);
    if !b.artifacts.iter().any(|name| name.starts_with("firmware")) {
        return Err("Le reçu ne contient aucun firmware. Recompilez le firmware.".into());
    }
    for name in &b.artifacts {
        let path = Path::new(name);
        if path.file_name() != Some(path.as_os_str()) || name.contains(':') {
            return Err("Nom de fichier de firmware invalide. Recompilez le firmware.".into());
        }
        let original = fs::read(b.output.join(path)).map_err(|e| e.to_string())?;
        let current = fs::read(compiled.join(path)).map_err(|e| e.to_string())?;
        if original.is_empty() || current != original {
            return Err("Un fichier de compilation a changé. Recompilez avant le flash.".into());
        }
    }
    let mut cmd = platformio(p, &work)?;
    cmd.args(["--upload-port", port, "-t", "upload", "--disable-auto-clean"]);
    if erase {
        let mut erase_cmd = platformio(p, &work)?;
        erase_cmd.args(["--upload-port", port, "-t", "erase", "--disable-auto-clean"]);
        run(erase_cmd, tx)?;
    }
    run(cmd, tx)?;
    Ok("Transfert terminé par PlatformIO. Vérifiez le démarrage de l'appareil.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a trusted local MeshCore checkout and PlatformIO"]
    fn builds_meshcore_without_flashing() {
        let profile = Profile {
            source: std::env::var("MESH_TEST_SOURCE").unwrap(),
            environment: std::env::var("MESH_TEST_TARGET").unwrap(),
            pio: std::env::var("MESH_TEST_PIO").unwrap_or_else(|_| "pio".into()),
            ..Default::default()
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let log = std::thread::spawn(move || {
            for event in rx {
                if let Event::Log(line) = event {
                    println!("{line}");
                }
            }
        });
        let result = build(profile, &tx);
        drop(tx);
        log.join().unwrap();
        result.unwrap();
    }
    #[test] fn shared_profiles_cannot_select_local_tools_or_paths() {
        let p: Profile = serde_json::from_str(r#"{"pio":"C:\\other.exe","source":"C:\\elsewhere","patch":"custom.patch","port":"COM5","erase":true,"pins_checked":true,"environment":"Xiao_S3_repeater"}"#).unwrap();
        let p = p.without_local_settings();
        assert_eq!(p.pio, "pio");
        assert_eq!(p.environment, "Xiao_S3_repeater");
        assert!(p.source.is_empty() && p.patch.is_empty() && p.port.is_empty());
        assert!(!p.erase && !p.pins_checked);
        assert!(tool_path("..\\custom.cmd").is_err());
        assert!(Profile { environment: "..".into(), ..Default::default() }.validate().is_err());
    }
    #[test] fn release_numbers_and_roles() {
        assert_eq!(release_parts("room-server-v1.18.0"), Some(("1.18.0", "Serveur de room")));
        assert_eq!(release_parts("companion-v1.18.0"), Some(("1.18.0", "Compagnon")));
        assert_eq!(release_parts("rep-v6"), Some(("6", "Répéteur")));
        assert_eq!(release_parts("repeater-v1.0.0d"), Some(("1.0.0d", "Répéteur")));
        assert_eq!(release_parts("Sources locales"), None);
        assert_eq!(release_parts("invalid-vtest"), None);
        assert!(environment_matches_release("Heltec_v3_room_server", "room-server-v1.18.0"));
        assert!(!environment_matches_release("Heltec_v3_repeater", "companion-v1.18.0"));
        assert!(environment_matches_release("Xiao_C3_comp_radio_ble", "companion-v1.18.0"));
        assert!(environment_matches_release("Heltec_v3_repeater", "Sources locales"));
    }
    #[test] fn board_search_and_connection_variants() {
        assert!(board_matches("t1000e", "1"));
        assert!(board_matches("Xiao_nrf52", "X-I"));
        assert!(!board_matches("Heltec_v3", "xiao"));
        for (env, mode) in [("Xiao_C3_companion_radio_wifi", "Wi-Fi"), ("Xiao_C3_companion_radio_ble", "Bluetooth"), ("Xiao_C3_companion_radio_usb", "USB / série")] {
            assert_eq!(board_name(env), "Xiao_C3");
            assert_eq!(connection(env), mode);
        }
        assert_eq!(connection("Xiao_C6_companion_radio_ble_"), "Bluetooth");
        assert_eq!(board_name("t1000e_repeater"), "t1000e");
        assert_eq!(board_name("unknown"), "unknown");
    }
    fn gpio_profile() -> Profile {
        Profile { environment: "Heltec_v3_repeater".into(), pins_checked: true, gpio: vec![Gpio::default()], ..Default::default() }
    }
    #[test] fn old_profiles_keep_gpio_disabled() {
        let p: Profile = serde_json::from_str(r#"{"environment":"test_repeater"}"#).unwrap();
        assert!(p.gpio.is_empty());
        assert!(!p.pins_checked);
        assert!(!p.bot.enabled);
    }
    #[test] fn bot_validation_checks_wiring_destinations_and_templates() {
        let mut p = Profile { environment: "Heltec_v3_repeater".into(), pins_checked: true, ..Default::default() };
        p.bot.enabled = true;
        assert!(p.validate().is_ok());
        p.bot.scl = p.bot.sda; assert!(p.validate().is_err()); p.bot.scl = 5;
        p.gpio.push(Gpio::default()); assert!(p.validate().is_err()); p.gpio.clear();
        p.bot.messages[0].channel = 9; assert!(p.validate().is_err()); p.bot.messages[0].channel = 0;
        p.bot.messages[0].hour = 24; assert!(p.validate().is_err()); p.bot.messages[0].hour = 7;
        p.bot.messages[0].text = "{unknown}".into(); assert!(p.validate().is_err());
        p.bot.messages[0].text = "{min}".repeat(20); assert!(p.validate().is_err());
        p.bot.messages[0] = BotMessage::default();
        p.bot.channels[0].hashtag = "#bad\"; code".into(); assert!(p.validate().is_err());
        p.bot.channels[0] = PublicChannel::default();
        let copy: Profile = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(copy.validate().is_ok()); assert_eq!(copy.bot.messages[1].hour, 21);
        assert_eq!(copy.bot.channels[0].hashtag, "#meteo");
        p.pins_checked = false; assert!(p.validate().is_err());
    }
    #[test] fn bot_patch_wires_runtime_and_rejects_incompatible_sources() {
        let work = std::env::temp_dir().join(format!("atelier-bot-test-{}", stamp()));
        let role = work.join("examples/simple_repeater"); fs::create_dir_all(&role).unwrap();
        fs::write(work.join("platformio.ini"), "lib_deps =\n  SPI\n").unwrap();
        fs::write(role.join("main.cpp"), "#include \"MyMesh.h\"\nvoid setup() { the_mesh.begin(fs); }\nvoid loop() { the_mesh.loop(); }").unwrap();
        fs::write(role.join("MyMesh.h"), "public: void sendNodeDiscoverReq();").unwrap();
        let mut p = Profile::default(); p.bot.enabled = true;
        patch_bot(&work, &p).unwrap();
        let main = fs::read_to_string(role.join("main.cpp")).unwrap();
        assert!(main.contains("AtelierBot::begin();"));
        assert!(main.contains("AtelierBot::loop(the_mesh, rtc_clock.getCurrentTime());"));
        let config = fs::read_to_string(role.join("AtelierBotConfig.h")).unwrap();
        assert!(config.contains("{true, 420, 0,")); assert!(config.contains("{true, 1260, 0,"));
        assert!(config.contains("\"#meteo\""));
        assert!(fs::read_to_string(role.join("MyMesh.h")).unwrap().contains("createGroupDatagram(PAYLOAD_TYPE_GRP_TXT"));
        assert!(patch_bot(&work, &p).is_err());
        fs::remove_dir_all(work).unwrap();
    }
    #[test] fn gpio_validation_fails_closed() {
        let mut p = gpio_profile();
        assert!(p.validate().is_ok());
        p.pins_checked = false; assert!(p.validate().is_err());
        p.pins_checked = true; p.gpio.push(Gpio::default()); assert!(p.validate().is_err());
        p.gpio.pop(); p.gpio[0].pin = 49; assert!(p.validate().is_err());
        p.gpio[0].pin = 4; p.gpio[0].channel = 1; assert!(p.validate().is_err());
        p.gpio[0].channel = 100; p.gpio[0].visibility = Visibility::Public; assert!(p.validate().is_err());
        p.public_telemetry = true; p.channels = [false; 4]; assert!(p.validate().is_ok());
        p.environment = "test_companion".into(); assert!(p.validate().is_err());
        p.environment = "test_repeater".into(); p.patch = "custom.patch".into(); assert!(p.validate().is_err());
    }
    #[test] fn gpio_patch_initializes_pins_and_preserves_access_boundaries() {
        let work = std::env::temp_dir().join(format!("atelier-gpio-test-{}", stamp()));
        let role = work.join("examples/simple_repeater");
        fs::create_dir_all(&role).unwrap();
        fs::write(role.join("main.cpp"), "void setup() { board.begin(); }").unwrap();
        fs::write(role.join("MyMesh.cpp"), "} else if (strcmp((char *)data, _prefs.guest_password) == 0) {}\nperm_mask = 0x00;\nuint8_t tlen = telemetry.getSize();").unwrap();
        let mut p = gpio_profile();
        p.public_telemetry = true;
        p.gpio.push(Gpio { pin: 5, channel: 101, visibility: Visibility::Public, ..Default::default() });
        p.gpio.push(Gpio { pin: 6, channel: 102, visibility: Visibility::Contacts, ..Default::default() });
        p.gpio.push(Gpio { pin: 7, channel: 103, mode: PinMode::High, ..Default::default() });
        patch_sources(&work, &p).unwrap();
        let main = fs::read_to_string(role.join("main.cpp")).unwrap();
        assert!(main.contains("#error Atelier_GPIO_requires_ESP32"));
        assert!(main.contains("GPIO_IS_VALID_OUTPUT_GPIO(7)"));
        assert!(main.find("digitalWrite(7, HIGH)").unwrap() < main.find("pinMode(7, OUTPUT)").unwrap());
        let mesh = fs::read_to_string(role.join("MyMesh.cpp")).unwrap();
        assert!(mesh.contains("if (sender->isAdmin()) telemetry.addDigitalInput(100"));
        assert!(mesh.contains("if (true) telemetry.addDigitalInput(101"));
        assert!(mesh.contains("!= PERM_ACL_GUEST) telemetry.addDigitalInput(102"));
        assert!(mesh.contains("channel == 101"));
        assert!(!mesh.contains("channel == 100"));
        assert!(!mesh.contains("channel == 102"));
        assert!(!mesh.contains("addDigitalInput(103"));
        assert!(!mesh.contains("data[0] == 0 ||"));
        assert!(mesh.contains("type != 136"));
        fs::remove_dir_all(&work).unwrap();
    }
    #[test] fn incompatible_source_is_rejected_before_writing() {
        assert!(replace_once("nothing", "board.begin();", "patch").is_err());
        assert!(replace_once("board.begin(); board.begin();", "board.begin();", "patch").is_err());
    }
    #[test] fn rejects_bad_voltage_and_empty_channels() {
        let mut p = Profile { environment: "Heltec_v3_repeater".into(), battery: true, ..Default::default() };
        assert!(p.validate().is_ok());
        p.resume_mv = p.low_mv; assert!(p.validate().is_err());
        p.battery = false; p.public_telemetry = true; p.channels = [false; 4]; assert!(p.validate().is_err());
        p.channels[0] = true; assert!(p.validate().is_ok());
    }
}
