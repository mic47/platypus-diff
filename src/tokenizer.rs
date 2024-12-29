use std::collections::VecDeque;

use crate::types::Token as TokenTrait;

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    WhiteSpace,
    SpecialCharacter,
    Word,
    BlockStart(usize),
    BlockEnd(usize),
}

#[derive(Clone, Debug)]
pub struct Token<'a, T> {
    /// Text of the token
    pub text: &'a str,
    /// Index of the start of the token in the original text. End is defined by length of text.
    pub start: usize,
    // TODO: should this be a metadata, or even not in this type?
    pub t: T,
}

impl<'a> TokenTrait for Token<'a, TokenType> {
    fn text(&self) -> &str {
        self.text
    }

    fn start(&self) -> usize {
        self.start
    }

    fn is_whitespace(&self) -> bool {
        self.t == TokenType::WhiteSpace
    }
}

#[derive(Debug)]
pub struct TokenParser<'a> {
    source: &'a str,
    position: usize,
    next_tokens: VecDeque<Token<'a, TokenType>>,
    prev_indentation: usize,
}

impl<'a> TokenParser<'a> {
    pub fn parse(text: &'a str) -> TokenParser<'a> {
        TokenParser {
            source: text,
            position: 0,
            next_tokens: VecDeque::new(),
            prev_indentation: 0,
        }
    }
}

#[derive(PartialEq, Debug)]
enum CharType {
    WhiteSpace,
    Word,
    BlockChar,
    Other,
}

fn char_type(c: char) -> CharType {
    if c.is_whitespace() {
        CharType::WhiteSpace
    } else if c.is_alphanumeric() || c == '_' {
        CharType::Word
    } else if c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}' {
        CharType::BlockChar
    } else {
        CharType::Other
    }
}

impl<'a> Iterator for TokenParser<'a> {
    type Item = Token<'a, TokenType>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(t) = self.next_tokens.pop_front() {
            //println!("{:?}", t);
            return Some(t);
        }
        let rest_of_text = self.source.split_at(self.position).1;
        let c_type = char_type(rest_of_text.chars().next()?);
        let len = if c_type == CharType::BlockChar {
            rest_of_text
                .chars()
                .next()
                .map(|x| x.len_utf8())
                .unwrap_or(0)
        } else {
            rest_of_text
                .chars()
                .take_while(|x| char_type(*x) == c_type)
                .map(|x| x.len_utf8())
                .sum::<usize>()
        };
        let start = self.position;
        let end = self.position + len;
        let token = Token {
            text: self.source.get(start..end).unwrap(), // This should never fail
            start,
            t: match c_type {
                CharType::WhiteSpace => TokenType::WhiteSpace,
                CharType::Word => TokenType::Word,
                CharType::Other => TokenType::SpecialCharacter,
                CharType::BlockChar => TokenType::SpecialCharacter,
            },
        };
        self.position += len;
        if c_type == CharType::WhiteSpace {
            let whitespace_text = self.source.get(self.position - len..self.position).unwrap();
            let current_indentation = if whitespace_text.contains('\n') {
                whitespace_text.split('\n').last().unwrap().len()
            } else {
                self.prev_indentation
            };
            if current_indentation != self.prev_indentation {
                self.next_tokens.push_back(Token {
                    text: self.source.get(self.position..self.position).unwrap(), // This should never fail
                    start: self.position,
                    t: if current_indentation < self.prev_indentation {
                        TokenType::BlockEnd(self.prev_indentation)
                    } else {
                        TokenType::BlockStart(current_indentation)
                    },
                });
                self.prev_indentation = current_indentation;
            }
        }
        Some(token)
    }
}
