#[derive(Debug)]
enum TokenType {
    WhiteSpace,
    SpecialCharacter,
    Word,
}

struct Token<'a, T> {
    source: &'a str,
    start: usize,
    end: usize,
    // TODO: should this be a metadata, or even not in this type?
    t: T,
}

impl<'a, T: std::fmt::Debug> std::fmt::Debug for Token<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Token")
            .field("text", &self.source.get(self.start..self.end).unwrap())
            .field("start", &self.start)
            .field("end", &self.end)
            .field("t", &self.t)
            .finish()
    }
}

#[derive(Debug)]
struct TokenParser<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> TokenParser<'a> {
    pub fn parse(text: &'a str) -> TokenParser<'a> {
        TokenParser {
            source: text,
            position: 0,
        }
    }
}

#[derive(PartialEq, Debug)]
enum CharType {
    WhiteSpace,
    Word,
    Other,
}

fn char_type(c: char) -> CharType {
    if c.is_whitespace() {
        CharType::WhiteSpace
    } else if c.is_alphanumeric() || c == ' ' {
        CharType::Word
    } else {
        CharType::Other
    }
}

impl<'a> Iterator for TokenParser<'a> {
    type Item = Token<'a, TokenType>;
    fn next(&mut self) -> Option<Self::Item> {
        let rest_of_text = self.source.split_at(self.position).1;
        let c_type = char_type(rest_of_text.chars().next()?);
        let len = rest_of_text
            .chars()
            .take_while(|x| char_type(*x) == c_type)
            .map(|x| x.len_utf8())
            .sum::<usize>();
        let token = Token {
            source: self.source,
            start: self.position,
            end: self.position + len,
            t: match c_type {
                CharType::WhiteSpace => TokenType::WhiteSpace,
                CharType::Word => TokenType::Word,
                CharType::Other => TokenType::SpecialCharacter,
            },
        };
        self.position += len;
        Some(token)
    }
}

// TODO: Insert BlockStart/BlockEnd for whitespace
// TODO: Eventually better parsing -- i.e. add BlockStart/BlockEnd for non-whitesace things
// TODO: Add line and col numbers to tokens

fn main() {
    println!("Hello, world!");
    let text = std::fs::read_to_string("src/main.rs").unwrap();
    for token in TokenParser::parse(&text) {
        println!("{:?}", token)
    }
}
