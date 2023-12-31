use std::{path::PathBuf, rc::Rc};

use clap::Parser;

#[derive(Debug, PartialEq, Clone)]
enum TokenType {
    WhiteSpace,
    SpecialCharacter,
    Word,
}

#[derive(Clone)]
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
            .field("text", &self.text())
            .field("start", &self.start)
            .field("end", &self.end)
            .field("t", &self.t)
            .finish()
    }
}

impl<'a, T> Token<'a, T> {
    pub fn text(&self) -> &'a str {
        self.source.get(self.start..self.end).unwrap()
    }
}

impl<'a, T: PartialEq> PartialEq for Token<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.t == other.t && self.text() == other.text()
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

#[derive(Debug, Clone)]
enum AlignmentOperation<T> {
    Mutation { left: T, right: T },
    InsertLeft { left: T },
    InsertRight { right: T },
}

#[derive(Debug)]
enum AlignmentOperationType {
    Mutation,
    InsertLeft,
    InsertRight,
}

#[derive(Debug, Clone)]
enum PathList<T> {
    End,
    Node {
        payload: T,
        previous: Rc<PathList<T>>,
    },
}

impl<T: Clone> PathList<T> {
    pub fn extract_path<'a>(self: Self) -> Vec<T> {
        let mut out = vec![];
        let mut current = self;
        loop {
            current = match current {
                PathList::End => break,
                PathList::Node { payload, previous } => {
                    out.push(payload);
                    Rc::try_unwrap(previous).unwrap_or_else(|x| {
                        eprintln!("More than 1 reference!");
                        (*x).clone()
                    })
                }
            }
        }
        out.reverse();
        out
    }
}

fn align<'a>(
    left: &'a [Token<'a, TokenType>],
    right: &'a [Token<'a, TokenType>],
) -> Alignment<'a, Token<'a, TokenType>> {
    let result_path = {
        let mut current: Vec<(
            f64,
            Rc<PathList<AlignmentOperation<&'a Token<'a, TokenType>>>>,
        )> = Vec::with_capacity(left.len() + 1);
        current.push((0.0, Rc::new(PathList::End)));
        for l in left.iter() {
            let prev = current.last().unwrap();
            current.push((
                prev.0 + 1.,
                Rc::new(PathList::Node {
                    payload: AlignmentOperation::InsertLeft { left: l },
                    previous: prev.1.clone(),
                }),
            ))
        }
        let mut next = Vec::with_capacity(left.len() + 1);
        for r in right.iter() {
            let prev = &current[0];
            next.push((
                prev.0 + 1.,
                Rc::new(PathList::Node {
                    payload: AlignmentOperation::InsertRight { right: r },
                    previous: prev.1.clone(),
                }),
            ));
            for (l_index, l) in left.iter().enumerate() {
                let l_index = l_index + 1;
                let insert_right = (
                    current[l_index].0 + 1.,
                    &current[l_index].1,
                    AlignmentOperationType::InsertRight,
                );
                let prev = next.last().unwrap();
                let insert_left = (prev.0 + 1., &prev.1, AlignmentOperationType::InsertLeft);
                let diag = &current[l_index - 1];
                let mutation = (
                    diag.0 + if l == r { 0. } else { 1. },
                    &diag.1,
                    AlignmentOperationType::Mutation,
                );
                let best = if mutation.0 < insert_right.0 {
                    if mutation.0 < insert_left.0 {
                        mutation
                    } else {
                        insert_left
                    }
                } else {
                    if insert_right.0 < insert_left.0 {
                        insert_right
                    } else {
                        insert_left
                    }
                };
                next.push((
                    best.0,
                    Rc::new(PathList::Node {
                        payload: match best.2 {
                            AlignmentOperationType::Mutation => {
                                AlignmentOperation::Mutation { left: l, right: r }
                            }
                            AlignmentOperationType::InsertLeft => {
                                AlignmentOperation::InsertLeft { left: l }
                            }
                            AlignmentOperationType::InsertRight => {
                                AlignmentOperation::InsertRight { right: r }
                            }
                        },
                        previous: best.1.clone(),
                    }),
                ));
            }

            std::mem::swap(&mut current, &mut next);
            next.clear()
        }
        current.pop().unwrap().1
    };
    Alignment {
        left_tokens: left,
        right_tokens: right,
        alignment: Rc::try_unwrap(result_path)
            .unwrap_or_else(|x| {
                eprintln!("More than 1 reference!");
                (*x).clone()
            })
            .extract_path(),
    }
}

pub struct Alignment<'a, T> {
    left_tokens: &'a [T],
    right_tokens: &'a [T],
    alignment: Vec<AlignmentOperation<&'a T>>,
}

impl<T> AlignmentOperation<T> {
    pub fn left(&self) -> Option<&T> {
        match self {
            AlignmentOperation::Mutation { left, right: _ } => Some(left),
            AlignmentOperation::InsertLeft { left } => Some(left),
            AlignmentOperation::InsertRight { right: _ } => None,
        }
    }
    pub fn right(&self) -> Option<&T> {
        match self {
            AlignmentOperation::Mutation { left: _, right } => Some(right),
            AlignmentOperation::InsertLeft { left: _ } => None,
            AlignmentOperation::InsertRight { right } => Some(right),
        }
    }
}

impl<'a> Alignment<'a, Token<'a, TokenType>> {

    pub fn add_tokens(
        &mut self,
        left: &'a[Token<'a, TokenType>],
        right: &'a[Token<'a, TokenType>],
    ) {
        let mut old_alignment = Vec::with_capacity(self.alignment.len() + left.len() + right.len());
        std::mem::swap(&mut old_alignment, &mut self.alignment);
        let mut left = left.iter().peekable();
        let mut right = right.iter().peekable();
        let mut left_position = None;
        let mut right_position = None;
        old_alignment.reverse();
        while let Some(a) = old_alignment.pop() {
            left_position = a.left().cloned().or(left_position);
            if let Some(left_position) = left_position {
                while left
                    .peek()
                    .map(|p| p.start < left_position.start)
                    .unwrap_or(false)
                {
                    left.next().map(|left| {
                        self.alignment
                            .push(AlignmentOperation::InsertLeft { left: &left })
                    });
                }
            }
            right_position = a.right().cloned().or(right_position);
            if let Some(right_position) = right_position {
                while right
                    .peek()
                    .map(|p| p.start < right_position.start)
                    .unwrap_or(false)
                {
                    right.next().map(|right| {
                        self.alignment
                            .push(AlignmentOperation::InsertRight { right: &right })
                    });
                }
            }
            self.alignment.push(a);
        }
        self.alignment
            .extend(left.map(|left| AlignmentOperation::InsertLeft { left }));
        self.alignment
            .extend(right.map(|right| AlignmentOperation::InsertRight { right }));
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    left: PathBuf,
    right: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let left_text = std::fs::read_to_string(cli.left).unwrap();
    let right_text = std::fs::read_to_string(cli.right).unwrap();
    let (left_tokens, left_whitespaces): (Vec<_>, Vec<_>) =
        TokenParser::parse(&left_text).partition(|x| x.t != TokenType::WhiteSpace);
    let (right_tokens, right_whitespaces): (Vec<_>, Vec<_>) =
        TokenParser::parse(&right_text).partition(|x| x.t != TokenType::WhiteSpace);
    // TODO: removal of whitespace tokens should be implementation detail of align?
    let mut alignment = align(&left_tokens, &right_tokens);
    alignment.add_tokens(&left_whitespaces, &right_whitespaces);
    for operation in alignment.alignment.into_iter() {
        println!("{:?}", operation)
    }
}
