pub fn parse_c_escape(esc: char) -> Result<char, String> {
    match esc {
        'a' => Ok('\x07'),
        'b' => Ok('\x08'),
        'f' => Ok('\x0c'),
        'n' => Ok('\n'),
        'r' => Ok('\r'),
        't' => Ok('\t'),
        'v' => Ok('\x0b'),
        '0' => Ok('\0'),
        '\\' => Ok('\\'),
        '\'' => Ok('\''),
        '"' => Ok('"'),
        other => Err(format!("unsupported C escape \\{other}")),
    }
}
