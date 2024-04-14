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
    /// Comments
    Comment(&'a str),
}

#[derive(Debug, PartialEq, Eq)]
pub enum LrcItem<'a> {
    Metadata(LrcMetadata<'a>),
    /// Lyric text and timestamp in milliseconds without offset
    Lyric(&'a str, Vec<i64>),
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

pub fn parse_single(line: &str, line_num: usize) -> Result<Option<LrcItem<'_>>, LrcParseError> {
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

    if line.trim().is_empty() {
        return Ok(None);
    }

    let parse_result: IResult<&str, Vec<(&str, &str, &str, &str, &str)>> = tag_parser(line);
    let (text, tags) = parse_result.map_err(|_| LrcParseError::NoTagInNonEmptyLine(line_num))?;
    Ok(Some(match tags[0] {
        // `[:]` is considered as comment line
        (_left_sq, "", _semicon, "", _right_sq) => LrcItem::Metadata(LrcMetadata::Comment(text)),
        (_left_sq, attr, _semicon, content, _right_sq) => match attr.trim() {
            "ar" => LrcItem::Metadata(LrcMetadata::Artist(content.trim())),
            "al" => LrcItem::Metadata(LrcMetadata::Album(content.trim())),
            "ti" => LrcItem::Metadata(LrcMetadata::Title(content.trim())),
            "au" => LrcItem::Metadata(LrcMetadata::Lyricist(content.trim())),
            "length" => LrcItem::Metadata(LrcMetadata::Length(content.trim())),
            "by" => LrcItem::Metadata(LrcMetadata::Author(content.trim())),
            "offset" => LrcItem::Metadata(LrcMetadata::Offset(
                content
                    .trim()
                    .parse()
                    .map_err(|_| LrcParseError::InvalidOffset(line_num))?,
            )),
            "re" => LrcItem::Metadata(LrcMetadata::Application(content.trim())),
            "ve" => LrcItem::Metadata(LrcMetadata::AppVersion(content.trim())),
            "#" => LrcItem::Metadata(LrcMetadata::Comment(content.trim())),
            _minute if _minute.parse::<i64>().is_ok() => {
                let mut timestamps = Vec::with_capacity(tags.len());
                for (_left_sq, minute, _semicon, sec, _right_sq) in tags {
                    let millisec = Decimal::from_str_exact(&sec.replace(':', "."))
                        .map_err(|_| LrcParseError::InvalidTimestamp(line_num))?
                        * dec!(1000);
                    let timestamp = minute
                        .parse::<i64>()
                        .map_err(|_| LrcParseError::InvalidTimestamp(line_num))?
                        * 60
                        * 1000
                        + millisec
                            .to_i64()
                            .ok_or(LrcParseError::InvalidTimestamp(line_num))?;
                    timestamps.push(timestamp);
                }
                LrcItem::Lyric(text, timestamps)
            }
            _ => return Ok(None), // ignores unrecognised tags
        },
    }))
}

pub fn parse<'a>(
    lyric_lines: impl Iterator<Item = &'a str>,
) -> Result<Vec<LrcItem<'a>>, LrcParseError> {
    let mut lrc_tags = Vec::new();

    for (line_num, line) in lyric_lines.enumerate() {
        if let Some(tag) = parse_single(line, line_num)? {
            lrc_tags.push(tag);
        }
    }

    Ok(lrc_tags)
}
