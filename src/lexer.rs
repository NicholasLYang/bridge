use crate::utils::NameTable;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Display, Formatter};
use std::str::CharIndices;

#[derive(Clone, Debug, PartialEq, EnumDiscriminants, Serialize, Deserialize)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[strum_discriminants(name(TokenD))]
pub enum Token {
    False,
    True,
    Else,
    Export,
    For,
    If,
    Return,
    Struct,
    Let,
    While,
    Fn,
    Ident(usize),
    Float(f64),
    Integer(i64),
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Semicolon,
    Colon,
    Comma,
    Dot,
    Amp,
    AmpAmp,
    Pipe,
    PipePipe,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Plus,
    PlusEqual,
    Minus,
    MinusEqual,
    Div,
    DivEqual,
    Times,
    TimesEqual,
    Arrow,
    FatArrow,
    Slash,
    String(String),
}

impl Display for TokenD {
    fn fmt<'a>(&self, f: &'a mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TokenD::False => "false",
                TokenD::True => "true",
                TokenD::Else => "else",
                TokenD::Export => "export",
                TokenD::For => "for",
                TokenD::If => "if",
                TokenD::Return => "return",
                TokenD::Struct => "struct",
                TokenD::Let => "let",
                TokenD::While => "while",
                TokenD::Fn => "fn",
                TokenD::Ident => "identifier",
                TokenD::Float => "float",
                TokenD::Integer => "int",
                TokenD::LBrace => "{",
                TokenD::RBrace => "}",
                TokenD::LBracket => "[",
                TokenD::RBracket => "]",
                TokenD::LParen => "(",
                TokenD::RParen => ")",
                TokenD::Semicolon => ";",
                TokenD::Colon => ":",
                TokenD::Comma => ",",
                TokenD::Dot => ".",
                TokenD::Amp => "&",
                TokenD::AmpAmp => "&&",
                TokenD::Pipe => "|",
                TokenD::PipePipe => "||",
                TokenD::Greater => ">",
                TokenD::GreaterEqual => ">=",
                TokenD::Less => "<",
                TokenD::LessEqual => "<=",
                TokenD::Bang => "!",
                TokenD::BangEqual => "!=",
                TokenD::Equal => "=",
                TokenD::EqualEqual => "==",
                TokenD::Plus => "+",
                TokenD::PlusEqual => "+=",
                TokenD::Minus => "-",
                TokenD::MinusEqual => "-=",
                TokenD::Div => "/",
                TokenD::DivEqual => "/=",
                TokenD::Times => "*",
                TokenD::TimesEqual => "*=",
                TokenD::FatArrow => "=>",
                TokenD::Arrow => "->",
                TokenD::Slash => "\\",
                TokenD::String => "string",
            }
        )
    }
}

#[derive(PartialEq, Clone, Copy, Deserialize, Serialize)]
pub struct Location(pub usize);

impl Display for Location {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for Location {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct LocationRange(pub Location, pub Location);

impl Display for LocationRange {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let l1 = &self.0;
        let l2 = &self.1;
        write!(f, "({}---{})", l1, l2)
    }
}

impl Debug for LocationRange {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let l1 = &self.0;
        let l2 = &self.1;
        write!(f, "({}---{})", l1, l2)
    }
}

#[inline]
fn is_id_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

#[inline]
fn is_id_body(ch: char) -> bool {
    ch == '_' || ch.is_ascii_digit() || ch.is_ascii_alphabetic()
}

#[derive(Debug, Fail, PartialEq, Clone, Serialize, Deserialize)]
pub enum LexicalError {
    #[fail(display = "{}: Invalid character '{}'", location, ch)]
    InvalidCharacter { ch: char, location: LocationRange },

    #[fail(display = "{}: String was not terminated", location)]
    UnterminatedString { location: LocationRange },

    #[fail(display = "This word is reserved for implementation reasons")]
    ReservedWord {location: LocationRange }
}

impl LexicalError {
    pub fn get_location(&self) -> LocationRange {
        match self {
            LexicalError::InvalidCharacter { ch: _, location } => *location,
            LexicalError::UnterminatedString { location } => *location,
            LexicalError::ReservedWord { location} => *location,
        }
    }
}

pub struct Lexer<'input> {
    source: &'input str,
    chars: CharIndices<'input>,
    pub name_table: NameTable,
    row: usize,
    column: usize,
    index: usize,
    lookahead: Option<(usize, char)>,
    lookahead2: Option<(usize, char)>,
}

impl<'input> Lexer<'input> {
    pub fn new(source: &'input str) -> Lexer<'input> {
        let mut chars = source.char_indices();
        let lookahead = chars.next();
        let lookahead2 = chars.next();

        Lexer {
            source,
            chars,
            row: 1,
            column: 1,
            index: 0,
            name_table: NameTable::new(),
            lookahead,
            lookahead2,
        }
    }

    pub fn get_location(&self) -> Location {
        Location(self.index)
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        let next = self.lookahead;
        self.lookahead = self.lookahead2;
        self.lookahead2 = self.chars.next();
        self.index += 1;
        if let Some((_, '\n')) = next {
            self.row += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
        next
    }

    #[allow(dead_code)]
    fn peek(&self) {
        println!("{:?}", self.lookahead);
    }

    fn lookahead_match(
        &mut self,
        start_loc: Location,
        matched_token: Token,
        alt_token: Token,
        match_ch: char,
    ) -> <Lexer<'input> as Iterator>::Item {
        match self.lookahead {
            Some((_, ch)) => {
                if match_ch == ch {
                    self.bump();
                    Ok((matched_token, LocationRange(start_loc, self.get_location())))
                } else {
                    Ok((alt_token, LocationRange(start_loc, self.get_location())))
                }
            }
            None => Ok((alt_token, LocationRange(start_loc, start_loc))),
        }
    }

    fn take_until<F>(&mut self, mut terminate: F) -> Option<usize>
    where
        F: FnMut(char) -> bool,
    {
        while let Some((i, ch)) = self.lookahead {
            if terminate(ch) {
                return Some(i);
            } else {
                self.bump();
            }
        }
        None
    }

    fn take_while<F>(&mut self, mut condition: F) -> Option<usize>
    where
        F: FnMut(char) -> bool,
    {
        self.take_until(|ch| !condition(ch))
    }

    fn skip_to_line_end(&mut self) {
        self.take_while(|ch| ch != '\n');
    }

    fn skip_whitespace(&mut self) {
        self.take_while(|ch| ch.is_whitespace());
    }

    fn read_string(
        &mut self,
        start_index: usize,
        start_loc: Location,
    ) -> <Lexer<'input> as Iterator>::Item {
        match self.take_until(|ch| ch == '"') {
            Some(i) => {
                self.bump();
                let end_loc = self.get_location();
                Ok((
                    Token::String(self.source[start_index + 1..i].to_string()),
                    LocationRange(start_loc, end_loc),
                ))
            }
            None => Err(LexicalError::UnterminatedString {
                location: LocationRange(start_loc, Location(self.index)),
            }),
        }
    }

    fn read_number(
        &mut self,
        start_index: usize,
        start_loc: Location,
    ) -> <Lexer<'input> as Iterator>::Item {
        let mut end_index = self.take_while(|ch| ch.is_ascii_digit());
        let mut is_decimal = false;

        if let Some((_, '.')) = self.lookahead {
            // Check if it's a decimal or a field access
            if let Some((_, next_ch)) = self.lookahead2 {
                if next_ch.is_ascii_digit() {
                    is_decimal = true;
                    self.bump();
                    end_index = self.take_while(|ch| ch.is_ascii_digit());
                }
            }
        }
        let end_loc = self.get_location();
        let end_index = end_index.unwrap_or_else(|| self.source.len());
        if is_decimal {
            Ok((
                Token::Float(
                    self.source[start_index..end_index]
                        .parse()
                        .expect("unparseable number"),
                ),
                LocationRange(start_loc, end_loc),
            ))
        } else {
            Ok((
                Token::Integer(
                    self.source[start_index..end_index]
                        .parse()
                        .expect("unparseable number"),
                ),
                LocationRange(start_loc, end_loc),
            ))
        }
    }

    fn read_identifier(
        &mut self,
        start_index: usize,
        start_loc: Location,
    ) -> <Lexer<'input> as Iterator>::Item {
        let end_index = self
            .take_while(|ch| is_id_start(ch) || is_id_body(ch))
            .unwrap_or_else(|| self.source.len());
        let end_loc = self.get_location();
        let location = LocationRange(start_loc, end_loc);
        let token = match &self.source[start_index..end_index] {
            "else" => Token::Else,
            "false" => Token::False,
            "for" => Token::For,
            "if" => Token::If,
            "struct" => Token::Struct,
            "return" => Token::Return,
            "true" => Token::True,
            "let" => Token::Let,
            "while" => Token::While,
            "fn" => Token::Fn,
            "export" => Token::Export,
            "as" => return Err(LexicalError::ReservedWord { location }),
            "break" => return Err(LexicalError::ReservedWord { location }),
            "const" => return Err(LexicalError::ReservedWord { location }),
            "continue" => return Err(LexicalError::ReservedWord { location }),
            "crate" => return Err(LexicalError::ReservedWord { location }),
            "enum" => return Err(LexicalError::ReservedWord { location }),
            "extern" => return Err(LexicalError::ReservedWord { location }),
            "impl" => return Err(LexicalError::ReservedWord { location }),
            "in" => return Err(LexicalError::ReservedWord { location }),
            "loop" => return Err(LexicalError::ReservedWord { location }),
            "match" => return Err(LexicalError::ReservedWord { location }),
            "mod" => return Err(LexicalError::ReservedWord { location }),
            "move" => return Err(LexicalError::ReservedWord { location }),
            "mut" => return Err(LexicalError::ReservedWord { location }),
            "pub" => return Err(LexicalError::ReservedWord { location }),
            "ref" => return Err(LexicalError::ReservedWord { location }),
            "self" => return Err(LexicalError::ReservedWord { location }),
            "Self" => return Err(LexicalError::ReservedWord { location }),
            "static" => return Err(LexicalError::ReservedWord { location }),
            "super" => return Err(LexicalError::ReservedWord { location }),
            "trait" => return Err(LexicalError::ReservedWord { location }),
            "type" => return Err(LexicalError::ReservedWord { location }),
            "unsafe" => return Err(LexicalError::ReservedWord { location }),
            "use" => return Err(LexicalError::ReservedWord { location }),
            "where" => return Err(LexicalError::ReservedWord { location }),
            "async" => return Err(LexicalError::ReservedWord { location }),
            "await" => return Err(LexicalError::ReservedWord { location }),
            "dyn" => return Err(LexicalError::ReservedWord { location }),
            "abstract" => return Err(LexicalError::ReservedWord { location }),
            "become" => return Err(LexicalError::ReservedWord { location }),
            "box" => return Err(LexicalError::ReservedWord { location }),
            "do" => return Err(LexicalError::ReservedWord { location }),
            "final" => return Err(LexicalError::ReservedWord { location }),
            "macro" => return Err(LexicalError::ReservedWord { location }),
            "override" => return Err(LexicalError::ReservedWord { location }),
            "priv" => return Err(LexicalError::ReservedWord { location }),
            "typeof" => return Err(LexicalError::ReservedWord { location }),
            "unsized" => return Err(LexicalError::ReservedWord { location }),
            "virtual" => return Err(LexicalError::ReservedWord { location }),
            "yield" => return Err(LexicalError::ReservedWord { location }),
            "try" => return Err(LexicalError::ReservedWord { location }),
            ident => {
                let ident = ident.to_string();
                if let Some(id) = self.name_table.get_id(&ident) {
                    Token::Ident(*id)
                } else {
                    let id = self.name_table.insert(ident);
                    Token::Ident(id)
                }
            }
        };
        Ok((token, location))
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Result<(Token, LocationRange), LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();
        let start_loc = self.get_location();
        if let Some((i, ch)) = self.bump() {
            let end_loc = self.get_location();
            match ch {
                '{' => Some(Ok((Token::LBrace, LocationRange(start_loc, end_loc)))),
                '}' => Some(Ok((Token::RBrace, LocationRange(start_loc, end_loc)))),
                '(' => Some(Ok((Token::LParen, LocationRange(start_loc, end_loc)))),
                ')' => Some(Ok((Token::RParen, LocationRange(start_loc, end_loc)))),
                '[' => Some(Ok((Token::LBracket, LocationRange(start_loc, end_loc)))),
                ']' => Some(Ok((Token::RBracket, LocationRange(start_loc, end_loc)))),
                ';' => Some(Ok((Token::Semicolon, LocationRange(start_loc, end_loc)))),
                ',' => Some(Ok((Token::Comma, LocationRange(start_loc, end_loc)))),
                '.' => Some(Ok((Token::Dot, LocationRange(start_loc, end_loc)))),
                '\\' => Some(Ok((Token::Slash, LocationRange(start_loc, end_loc)))),
                ':' => Some(Ok((Token::Colon, LocationRange(start_loc, end_loc)))),
                '+' => Some(self.lookahead_match(start_loc, Token::PlusEqual, Token::Plus, '=')),
                '-' => match self.lookahead {
                    Some((_, '>')) => {
                        self.bump();
                        Some(Ok((
                            Token::Arrow,
                            LocationRange(start_loc, self.get_location()),
                        )))
                    }
                    Some((_, '=')) => {
                        self.bump();
                        Some(Ok((
                            Token::MinusEqual,
                            LocationRange(start_loc, self.get_location()),
                        )))
                    }
                    _ => Some(Ok((Token::Minus, LocationRange(start_loc, end_loc)))),
                },
                '*' => Some(self.lookahead_match(start_loc, Token::TimesEqual, Token::Times, '=')),
                '/' => match self.lookahead {
                    Some((_, '/')) => {
                        self.skip_to_line_end();
                        self.next()
                    }
                    Some((_, '=')) => {
                        self.bump();
                        Some(Ok((
                            Token::DivEqual,
                            LocationRange(start_loc, self.get_location()),
                        )))
                    }
                    _ => Some(Ok((Token::Div, LocationRange(start_loc, end_loc)))),
                },
                '!' => Some(self.lookahead_match(start_loc, Token::BangEqual, Token::Bang, '=')),
                '=' => match self.lookahead {
                    Some((_, '>')) => {
                        self.bump();
                        Some(Ok((
                            Token::FatArrow,
                            LocationRange(start_loc, self.get_location()),
                        )))
                    }
                    Some((_, '=')) => {
                        self.bump();
                        Some(Ok((
                            Token::EqualEqual,
                            LocationRange(start_loc, self.get_location()),
                        )))
                    }
                    _ => Some(Ok((Token::Equal, LocationRange(start_loc, end_loc)))),
                },
                '>' => {
                    Some(self.lookahead_match(start_loc, Token::GreaterEqual, Token::Greater, '='))
                }
                '<' => Some(self.lookahead_match(start_loc, Token::LessEqual, Token::Less, '=')),
                '&' => Some(self.lookahead_match(start_loc, Token::AmpAmp, Token::Amp, '&')),
                '|' => Some(self.lookahead_match(start_loc, Token::PipePipe, Token::Pipe, '|')),
                '"' => Some(self.read_string(i, start_loc)),
                ch if is_id_start(ch) => Some(self.read_identifier(i, start_loc)),
                ch if ch.is_ascii_digit() => Some(self.read_number(i, start_loc)),
                ch => {
                    let error = LexicalError::InvalidCharacter {
                        ch,
                        location: LocationRange(start_loc, end_loc),
                    };
                    Some(Err(error))
                }
            }
        } else {
            None
        }
    }
}
