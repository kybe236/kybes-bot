use std::collections::HashMap;

use poise::CreateReply;

use crate::{Context, Error, utils::bot};

#[poise::command(slash_command)]
pub async fn morse(
    ctx: Context<'_>,
    #[description = "Text to convert to/from Morse"] text: String,
    #[description = "True = from Morse, False = to Morse"] from_morse: Option<bool>,
    #[description = "high"] high: Option<char>,
    #[description = "low"] low: Option<char>,
    #[description = "Send the response directly to you?"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    let ephemeral = bot::defer_based_on_ephemeral(ctx, ephemeral).await?;

    let high_char = high.unwrap_or('.');
    let low_char = low.unwrap_or('-');
    let from_morse = from_morse.unwrap_or(false);

    let output = if from_morse {
        morse_to_text(&text, high_char, low_char)
    } else {
        text_to_morse(&text, high_char, low_char)
    };

    ctx.send(CreateReply::default().content(output).ephemeral(ephemeral))
        .await?;

    Ok(())
}

fn text_to_morse(text: &str, high: char, low: char) -> String {
    let code_map = morse_code_map();

    text.to_uppercase()
        .chars()
        .map(|c| {
            if c == ' ' {
                "/".to_string()
            } else {
                code_map
                    .get(&c)
                    .map(|code| {
                        code.chars()
                            .map(|ch| if ch == '.' { high } else { low })
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
    let code_map = morse_code_map();
    let mut rev_map = HashMap::new();
    for (k, v) in &code_map {
        let custom_code: String = v
            .chars()
            .map(|c| if c == '.' { high } else { low })
            .collect();
        rev_map.insert(custom_code, *k);
    }

    morse
        .split(" / ")
        .map(|word| {
            word.split(' ')
                .filter_map(|letter_code| rev_map.get(letter_code))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn morse_code_map() -> HashMap<char, &'static str> {
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
}
