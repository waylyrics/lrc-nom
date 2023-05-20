use nom::IResult;
use rust_decimal::prelude::ToPrimitive;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
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

pub fn parse<'a>(lyric_lines: impl Iterator<Item = &'a str>) -> Result<Vec<LrcItem<'a>>, LrcParseError> {
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

    for (i, line) in lyric_lines.filter(|l| !l.is_empty()).enumerate() {
        let parse_result: IResult<&str, Vec<(&str, &str, &str, &str, &str)>> = tag_parser(line);
        match parse_result {
            Ok((text, tags)) => match tags[0] {
                // `[:]` is considered as comment line
                (_left_sq, "", _semicon, "", _right_sq) => continue,
                (_left_sq, attr, _semicon, content, _right_sq) => match attr.trim() {
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
                    _minute if _minute.parse::<i64>().is_ok() => lrc_items.extend(
                        tags.into_iter()
                            .map(|(_left_sq, minute, _semicon, sec, _right_sq)| {
                                let millisec = Decimal::from_str_exact(&sec.replace(':', "."))
                                    .map_err(|_| LrcParseError::InvalidTimestamp(i))?
                                    * dec!(1000);
                                let timestamp = minute
                                    .parse::<i64>()
                                    .map_err(|_| LrcParseError::InvalidTimestamp(i))?
                                    * 60
                                    * 1000
                                    + millisec
                                        .to_i64()
                                        .ok_or(LrcParseError::InvalidTimestamp(i))?;

                                Ok::<LrcItem<'_>, LrcParseError>(LrcItem::Lyric(text, timestamp))
                            })
                            .flatten(), // Errors in parsing are ignored
                    ),
                    _ => (), // ignores unrecognised tags
                },
            },
            Err(_) => return Err(LrcParseError::NoTagInNonEmptyLine(i)),
        }
    }

    Ok(lrc_items)
}