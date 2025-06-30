use once_cell::sync::Lazy;
use poise::CreateReply;
use std::collections::HashMap;

use crate::{Context, Error, utils::bot};

static MORSE_CODE_MAP: Lazy<HashMap<char, &'static str>> = Lazy::new(|| {
    HashMap::from([
        ('A', ".-"),
        ('B', "-..."),
        ('C', "-.-."),
        ('D', "-.."),
        ('E', "."),
        ('F', "..-."),
        ('G', "--."),
        ('H', "...."),
        ('I', ".."),
        ('J', ".---"),
        ('K', "-.-"),
        ('L', ".-.."),
        ('M', "--"),
        ('N', "-."),
        ('O', "---"),
        ('P', ".--."),
        ('Q', "--.-"),
        ('R', ".-."),
        ('S', "..."),
        ('T', "-"),
        ('U', "..-"),
        ('V', "...-"),
        ('W', ".--"),
        ('X', "-..-"),
        ('Y', "-.--"),
        ('Z', "--.."),
        ('1', ".----"),
        ('2', "..---"),
        ('3', "...--"),
        ('4', "....-"),
        ('5', "....."),
        ('6', "-...."),
        ('7', "--..."),
        ('8', "---.."),
        ('9', "----."),
        ('0', "-----"),
    ])
});

static REVERSE_MORSE_MAP: Lazy<HashMap<String, char>> = Lazy::new(|| {
    let mut rev = HashMap::new();
    for (&ch, &code) in MORSE_CODE_MAP.iter() {
        rev.insert(code.to_string(), ch);
    }
    rev
});

#[poise::command(slash_command)]
pub async fn morse(
    ctx: Context<'_>,
    #[description = "Text to convert to/from Morse"] text: String,
    #[description = "True = from Morse, False = to Morse"] from_morse: Option<bool>,
    #[description = "High signal char"] high: Option<char>,
    #[description = "Low signal char"] low: Option<char>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    let high = high.unwrap_or('.');
    let low = low.unwrap_or('-');
    let from_morse = from_morse.unwrap_or(false);

    let output = if from_morse {
        morse_to_text(&text, high, low)
    } else {
        text_to_morse(&text, high, low)
    };

    ctx.send(CreateReply::default().content(output).ephemeral(ephemeral))
        .await?;

    Ok(())
}

fn text_to_morse(text: &str, high: char, low: char) -> String {
    text.to_uppercase()
        .chars()
        .map(|c| {
            if c == ' ' {
                "/".to_string()
            } else {
                MORSE_CODE_MAP
                    .get(&c)
                    .map(|code| {
                        code.chars()
                            .map(|sym| if sym == '.' { high } else { low })
                            .collect::<String>()
                    })
                    .unwrap_or_default()
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn morse_to_text(morse: &str, high: char, low: char) -> String {
    // Build reverse map with custom chars
    let rev_map: HashMap<String, char> = REVERSE_MORSE_MAP
        .iter()
        .map(|(code, &ch)| {
            let custom_code = code
                .chars()
                .map(|c| if c == '.' { high } else { low })
                .collect();
            (custom_code, ch)
        })
        .collect();

    morse
        .split(" / ")
        .map(|word| {
            word.split_whitespace()
                .filter_map(|code| rev_map.get(code))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
