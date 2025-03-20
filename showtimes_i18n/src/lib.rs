use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

use fluent_templates::{
    LanguageIdentifier, Loader, fluent_bundle::FluentValue, langid, static_loader,
};

static LANG_IDS: LazyLock<HashMap<Language, LanguageIdentifier>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(Language::Id, langid!("id-ID"));
    map.insert(Language::En, langid!("en-US"));
    map.insert(Language::Ja, langid!("ja-JP"));
    map.insert(Language::Su, langid!("su-ID"));
    map.insert(Language::Jv, langid!("jv-ID"));
    map
});

/// Supported languages for the application
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// Indonesian
    #[default]
    Id,
    /// English
    En,
    /// Japanese
    Ja,
    /// Sundanese
    Su,
    /// Javanese
    Jv,
}

impl Language {
    /// Get the language code in string format
    pub fn code(&self) -> &'static str {
        match self {
            Language::En => "en-US",
            Language::Id => "id-ID",
            Language::Ja => "ja-JP",
            Language::Jv => "jv-ID",
            Language::Su => "su-ID",
        }
    }
}

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "id-ID",
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

/// Type alias for more complex creation of translation arguments.
pub type TrArg = (&'static str, String);

/// Vector of arguments to be used for translated message.
pub type TrArgVec = Vec<TrArg>;
pub type TrArgVecArg<'a> = &'a [TrArg];

/// Final type for argument map used as direct input for fluent-templates.
type ArgsMap<'m> = HashMap<Cow<'static, str>, FluentValue<'m>>;

/// Translates the input text to the currently set language.
fn translate(text_id: &str, language: Option<Language>, args: Option<&ArgsMap>) -> String {
    if let Some(li) = LANG_IDS.get(&language.unwrap_or_default()) {
        let translated = &*LOCALES.lookup_complete(li, text_id, args);
        return translated.to_string();
    }
    text_id.to_string()
}

/// Creates args map for the string translation.
fn args_to_map<'a, T>(params: &'a [(&'static str, T)]) -> ArgsMap<'a>
where
    T: Into<FluentValue<'a>> + Clone,
{
    let mut map: ArgsMap = HashMap::new();
    for (k, v) in params {
        let value: FluentValue = v.to_owned().into();
        map.insert(std::borrow::Cow::Borrowed(*k), value);
    }
    map
}

/// Return the translation with the given message ID and language.
///
/// This is made for translation text that doesn't need arguments.
///
/// If you need arguments, use the [`tr`] function instead.
pub fn t(msg_id: &str, language: Option<Language>) -> String {
    translate(msg_id, language, None)
}

/// Return the translation with the given message ID and language.
///
/// This is made for translation text that needs arguments.
///
/// If you don't need arguments, use the [`t`] function instead.
pub fn tr(msg_id: &str, language: Option<Language>, args: TrArgVecArg) -> String {
    let arg_map = args_to_map(args);
    translate(msg_id, language, Some(&arg_map))
}
