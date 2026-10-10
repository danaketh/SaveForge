//! Minimal parser for Valve's text `KeyValues` format (`.vdf` / `.acf`).

/// A `KeyValues` node. Objects keep insertion order and duplicate keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    /// First child whose key matches ignoring ASCII case.
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Self::Obj(entries) => entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Self::Str(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            Self::Obj(_) => None,
        }
    }
}

/// Parses a document into a root `Vdf::Obj`. Returns `None` when malformed.
pub fn parse(input: &str) -> Option<Vdf> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    let pairs = parse_pairs(&tokens, &mut pos, false)?;
    Some(Vdf::Obj(pairs))
}

enum Tok {
    Str(String),
    Open,
    Close,
}

fn tokenize(input: &str) -> Option<Vec<Tok>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '{' => tokens.push(Tok::Open),
            '}' => tokens.push(Tok::Close),
            // Platform conditionals like [$WIN32] are ignored.
            '[' => {
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
            }
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            other => s.push(other),
                        },
                        other => s.push(other),
                    }
                }
                tokens.push(Tok::Str(s));
            }
            other => {
                let mut s = String::from(other);
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || matches!(next, '{' | '}' | '"') {
                        break;
                    }
                    s.push(next);
                    chars.next();
                }
                tokens.push(Tok::Str(s));
            }
        }
    }
    Some(tokens)
}

fn parse_pairs(tokens: &[Tok], pos: &mut usize, nested: bool) -> Option<Vec<(String, Vdf)>> {
    let mut pairs = Vec::new();
    loop {
        match tokens.get(*pos) {
            None => return (!nested).then_some(pairs),
            Some(Tok::Close) => {
                *pos += 1;
                return nested.then_some(pairs);
            }
            Some(Tok::Open) => return None,
            Some(Tok::Str(key)) => {
                *pos += 1;
                let value = match tokens.get(*pos)? {
                    Tok::Str(v) => {
                        *pos += 1;
                        Vdf::Str(v.clone())
                    }
                    Tok::Open => {
                        *pos += 1;
                        Vdf::Obj(parse_pairs(tokens, pos, true)?)
                    }
                    Tok::Close => return None,
                };
                pairs.push((key.clone(), value));
            }
        }
    }
}
