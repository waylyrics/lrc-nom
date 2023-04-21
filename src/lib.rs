use nom::IResult;
use rust_decimal::prelude::ToPrimitive;
use thiserror::Error;

pub enum LrcMetadata<'a> {
    /// Artist of the song
    Artist(&'a str),
    /// Album this song belongs to
    Album(&'a str),
    /// Title of the song
    Title(&'a str),
    /// Lyricist wrote this songtext
    Lyricist(&'a str),
    /// Author of this LRC
    Author(&'a str),
    /// Length of the song
    Length(&'a str),
    /// Offset in milliseconds
    Offset(i64),
    /// The player or editor that created the LRC file
    Application(&'a str),
    /// version of the app above
    AppVersion(&'a str),
}

pub enum LrcItem<'a> {
    Metadata(LrcMetadata<'a>),
    /// Lyric text and timestamp in milliseconds without offset
    Lyric(&'a str, i64),
}

#[derive(Debug, Error)]
pub enum LrcParseError {
    #[error("No tag was found in non-empty line {0}")]
    NoTagInNonEmptyLine(usize),
    #[error("Invalid timestamp format in line {0}")]
    InvalidTimestamp(usize),
    #[error("Invalid offset format in line {0}")]
    InvalidOffset(usize),
}

pub fn parse<'a>(lyric: &'a str, lf: &str) -> Result<Vec<LrcItem<'a>>, LrcParseError> {
    use nom::{
        bytes::complete::{tag, take_until},
        multi::many1,
        sequence::tuple,
    };

    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    let mut tag_parser = many1(tuple((
        tag("["),
        take_until(":"),
        tag(":"),
        take_until("]"),
        tag("]"),
    )));

    let mut lrc_items = Vec::new();

    for (i, line) in lyric.split(lf).filter(|l| !l.is_empty()).enumerate() {
        match tag_parser(line) {
            Ok((text, tags)) => match tags[0] {
                ("[", attr, ":", content, "]") => match attr.trim() {
                    "ar" => lrc_items.push(LrcItem::Metadata(LrcMetadata::Artist(content.trim()))),
                    "al" => lrc_items.push(LrcItem::Metadata(LrcMetadata::Album(content.trim()))),
                    "ti" => lrc_items.push(LrcItem::Metadata(LrcMetadata::Title(content.trim()))),
                    "au" => {
                        lrc_items.push(LrcItem::Metadata(LrcMetadata::Lyricist(content.trim())))
                    }
                    "length" => {
                        lrc_items.push(LrcItem::Metadata(LrcMetadata::Length(content.trim())))
                    }
                    "by" => lrc_items.push(LrcItem::Metadata(LrcMetadata::Author(content.trim()))),
                    "offset" => lrc_items.push(LrcItem::Metadata(LrcMetadata::Offset(
                        content
                            .trim()
                            .parse()
                            .map_err(|_| LrcParseError::InvalidOffset(i))?,
                    ))),
                    "re" => {
                        lrc_items.push(LrcItem::Metadata(LrcMetadata::Application(content.trim())))
                    }
                    "ve" => {
                        lrc_items.push(LrcItem::Metadata(LrcMetadata::AppVersion(content.trim())))
                    }
                    _minute => {
                        for (_left_sq, minute, _semicon, sec, _right_sq) in tags {
                            let millisec = Decimal::from_str_exact(sec)
                                .map_err(|_| LrcParseError::InvalidTimestamp(i))
                                .unwrap()
                                * dec!(1000);
                            let timestamp = minute
                                .parse::<i64>()
                                .map_err(|_| LrcParseError::InvalidTimestamp(i))?
                                * 60
                                * 1000
                                + millisec.to_i64().unwrap();

                            lrc_items.push(LrcItem::Lyric(text, timestamp));
                        }
                    }
                },
            },
            Err(_) => return Err(LrcParseError::NoTagInNonEmptyLine(i)),
        }
    }

    Ok(lrc_items)
}