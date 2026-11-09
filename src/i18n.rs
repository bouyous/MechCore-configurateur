use eframe::egui;
use std::{
    collections::BTreeMap,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

// ISO language code, name in the language itself, and name for the Ollama prompt.
pub const LANGUAGES: &[(&str, &str, &str)] = &[
    ("fr", "Français", "French"),
    ("en", "English", "English"),
    ("az", "Azərbaycanca", "Azerbaijani"),
    ("bs", "Bosanski", "Bosnian"),
    ("ca", "Català", "Catalan"),
    ("cs", "Čeština", "Czech"),
    ("cnr", "Crnogorski", "Montenegrin"),
    ("da", "Dansk", "Danish"),
    ("de", "Deutsch", "German"),
    ("et", "Eesti", "Estonian"),
    ("es", "Español", "Spanish"),
    ("ga", "Gaeilge", "Irish"),
    ("hr", "Hrvatski", "Croatian"),
    ("is", "Íslenska", "Icelandic"),
    ("it", "Italiano", "Italian"),
    ("la", "Latina", "Latin"),
    ("lv", "Latviešu", "Latvian"),
    ("lb", "Lëtzebuergesch", "Luxembourgish"),
    ("lt", "Lietuvių", "Lithuanian"),
    ("hu", "Magyar", "Hungarian"),
    ("mt", "Malti", "Maltese"),
    ("nl", "Nederlands", "Dutch"),
    ("no", "Norsk", "Norwegian"),
    ("pl", "Polski", "Polish"),
    ("pt", "Português", "European Portuguese"),
    ("ro", "Română", "Romanian"),
    ("rm", "Rumantsch Grischun", "Romansh"),
    ("sq", "Shqip", "Albanian"),
    ("sk", "Slovenčina", "Slovak"),
    ("sl", "Slovenščina", "Slovenian"),
    ("fi", "Suomi", "Finnish"),
    ("sv", "Svenska", "Swedish"),
    ("tr", "Türkçe", "Turkish"),
    ("el", "Ελληνικά", "Greek"),
    ("be", "Беларуская", "Belarusian"),
    ("bg", "Български", "Bulgarian"),
    ("kk", "Қазақша", "Kazakh"),
    ("mk", "Македонски", "Macedonian"),
    ("ru", "Русский", "Russian"),
    ("sr", "Српски", "Serbian"),
    ("uk", "Українська", "Ukrainian"),
    ("hy", "Հայերեն", "Armenian"),
    ("ka", "ქართული", "Georgian"),
];

type Messages = BTreeMap<String, String>;
static CATALOGS: OnceLock<BTreeMap<String, Messages>> = OnceLock::new();
static LANGUAGE: AtomicUsize = AtomicUsize::new(0);

fn catalogs() -> &'static BTreeMap<String, Messages> {
    CATALOGS.get_or_init(|| {
        serde_json::from_str(include_str!("../locales/translations.json"))
            .expect("valid embedded translations")
    })
}

pub fn current() -> usize {
    LANGUAGE.load(Ordering::Relaxed)
}

pub fn set(locale: usize) {
    LANGUAGE.store(locale, Ordering::Relaxed);
}

pub fn saved() -> Option<usize> {
    let code: String = crate::firmware::load(&crate::firmware::root().join("language.json"))?;
    LANGUAGES.iter().position(|(id, _, _)| *id == code)
}

pub fn save(locale: usize) -> crate::firmware::Result<()> {
    crate::firmware::save(
        &crate::firmware::root().join("language.json"),
        &LANGUAGES[locale].0,
    )
}

pub fn for_language(locale: usize, text: &str) -> &str {
    if locale == 0 {
        return text;
    }
    catalogs()
        .get(LANGUAGES[locale].0)
        .and_then(|items| items.get(text))
        .map_or(text, String::as_str)
}

pub fn tr(text: &str) -> &str {
    for_language(current(), text)
}

pub fn tf(text: &str, values: &[&str]) -> String {
    interpolate(tr(text), values)
}

fn interpolate(text: &str, values: &[&str]) -> String {
    let mut result = String::with_capacity(text.len());
    for part in text.split_inclusive('}') {
        if let Some((prefix, field)) = part.rsplit_once('{')
            && let Some(index) = field
                .strip_suffix('}')
                .and_then(|index| index.parse::<usize>().ok())
            && let Some(value) = values.get(index)
        {
            result.push_str(prefix);
            result.push_str(value);
        } else {
            result.push_str(part);
        }
    }
    result
}

pub fn install_fonts(ctx: &egui::Context) {
    let Some(system) = std::env::var_os("SystemRoot") else {
        return;
    };
    let dir = std::path::PathBuf::from(system).join("Fonts");
    let mut fonts = egui::FontDefinitions::default();
    for name in ["segoeui.ttf", "sylfaen.ttf"] {
        if let Ok(bytes) = std::fs::read(dir.join(name)) {
            fonts
                .font_data
                .insert(name.into(), egui::FontData::from_owned(bytes).into());
            for family in fonts.families.values_mut() {
                family.push(name.into());
            }
        }
    }
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_offered_language_has_messages_and_placeholders() {
        let catalogs = catalogs();
        let english = &catalogs["en"];
        for (code, _, _) in LANGUAGES.iter().skip(1) {
            let messages = &catalogs[*code];
            assert_eq!(messages.len(), english.len(), "{code}");
            assert_ne!(
                messages["Choisissez votre langue"], "Choisissez votre langue",
                "{code}"
            );
            for (source, translation) in messages {
                for index in 0..4 {
                    let field = format!("{{{index}}}");
                    assert_eq!(
                        source.matches(&field).count(),
                        translation.matches(&field).count(),
                        "{code}: {source}"
                    );
                }
            }
        }
    }

    #[test]
    fn interpolation_reorders_fields_and_keeps_inserted_values_literal() {
        assert_eq!(interpolate("{1}, {0}!", &["{1}", "Xiao"]), "Xiao, {1}!");
    }
}
