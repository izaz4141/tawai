use std::sync::{LazyLock, OnceLock};

use anyhow::Result;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use uroman::Uroman;
use wana_kana::ConvertJapanese;
use whatlang::{Lang, detect};

static UROMAN: LazyLock<Uroman> = LazyLock::new(Uroman::new);
static JAPANESE_TOKENIZER: OnceLock<Tokenizer> = OnceLock::new();

fn get_japanese_tokenizer() -> Result<&'static Tokenizer> {
    if let Some(tok) = JAPANESE_TOKENIZER.get() {
        return Ok(tok);
    }
    let dictionary = load_dictionary("embedded://ipadic")
        .map_err(|e| anyhow::anyhow!("Failed to load IPADIC dictionary: {}", e))?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true);
    let tokenizer = Tokenizer::new(segmenter);
    JAPANESE_TOKENIZER
        .set(tokenizer)
        .map_err(|_| anyhow::anyhow!("Tokenizer already initialized"))?;
    Ok(JAPANESE_TOKENIZER.get().unwrap())
}

fn lang_to_code(lang: Lang) -> &'static str {
    match lang {
        Lang::Jpn => "jpn",
        Lang::Cmn => "cmn",
        Lang::Kor => "kor",
        Lang::Tha => "tha",
        Lang::Rus => "rus",
        Lang::Ara => "ara",
        Lang::Hin => "hin",
        Lang::Eng => "eng",
        Lang::Spa => "spa",
        Lang::Fra => "fra",
        Lang::Deu => "deu",
        Lang::Por => "por",
        Lang::Ita => "ita",
        Lang::Nld => "nld",
        _ => "und",
    }
}

fn romajize_japanese(text: &str) -> Result<String> {
    let tokenizer = get_japanese_tokenizer()?;
    let tokens = tokenizer.tokenize(text)?;

    let mut result = String::new();
    for mut token in tokens {
        let surface = token.surface.to_string();
        if surface.trim().is_empty() {
            result.push_str(&surface);
            continue;
        }
        let reading = token
            .get_detail(7)
            .filter(|r| !r.is_empty() && *r != "*")
            .map(|s| s.to_owned())
            .unwrap_or(surface);
        let romaji = reading.to_romaji();
        if !result.is_empty()
            && !result.ends_with(' ')
            && !romaji.starts_with(',')
            && !romaji.starts_with('.')
        {
            result.push(' ');
        }
        result.push_str(&romaji);
    }
    Ok(result)
}

fn romajize_with_uroman(text: &str, lang: &str) -> String {
    UROMAN
        .romanize_string::<uroman::rom_format::Str>(text, Some(lang))
        .to_string()
}

fn resolve_lang(lang: Option<&str>, text: &str) -> String {
    match lang {
        Some(l) if !l.is_empty() && l != "auto" => l.to_owned(),
        _ => detect(text)
            .map(|info| lang_to_code(info.lang()).to_owned())
            .unwrap_or_else(|| "und".to_owned()),
    }
}

pub fn romajize(text: &str, lang: Option<&str>) -> Result<String> {
    let actual_lang = resolve_lang(lang, text);

    match actual_lang.as_str() {
        "jpn" => romajize_japanese(text),
        _ => Ok(romajize_with_uroman(text, &actual_lang)),
    }
}
