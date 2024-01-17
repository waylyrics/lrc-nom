# lrc-nom

Simple LRC parser with nom.

Note: lrc-nom cannot handle UTF-8 BOM, please consider applying a `.trim_start_matches('\u{feff}')`
