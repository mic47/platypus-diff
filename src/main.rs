use std::{collections::VecDeque, path::PathBuf, rc::Rc};

use clap::Parser;
use colored::Colorize;

#[derive(Debug, PartialEq, Clone)]
enum TokenType {
    WhiteSpace,
    SpecialCharacter,
    Word,
    BlockStart(usize),
    BlockEnd(usize),
}

#[derive(Clone, Debug)]
struct Token<'a, T> {
    text: &'a str,
    start: usize,
    // TODO: should this be a metadata, or even not in this type?
    t: T,
}

impl<'a> Token<'a, TokenType> {
    pub fn insert_score(&self, previous_is_same: bool) -> f64 {
        let add = match self.t {
            TokenType::BlockEnd(_indent) => 1.,
            _ => 0.0,
        };
        if previous_is_same {
            0.3 + add
        } else {
            0.7 + add
        }
    }
    pub fn mutation_score(&self, other: &Self) -> f64 {
        if self.t != other.t {
            return 100.;
        }
        match self.t {
            TokenType::BlockStart(indent) | TokenType::BlockEnd(indent) => match other.t {
                TokenType::BlockStart(o_indent) | TokenType::BlockEnd(o_indent) => {
                    indent.abs_diff(o_indent) as f64
                }
                _ => {
                    panic!("This is impossible");
                }
            },
            TokenType::WhiteSpace | TokenType::SpecialCharacter | TokenType::Word => {
                if self.text.to_lowercase() == other.text.to_lowercase() {
                    return 0.;
                } else {
                    return 1.;
                }
            }
        }
    }
}

#[derive(Debug)]
struct TokenParser<'a> {
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
    Other,
}

fn char_type(c: char) -> CharType {
    if c.is_whitespace() {
        CharType::WhiteSpace
    } else if c.is_alphanumeric() || c == '_' {
        CharType::Word
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
        let len = rest_of_text
            .chars()
            .take_while(|x| char_type(*x) == c_type)
            .map(|x| x.len_utf8())
            .sum::<usize>();
        let start = self.position;
        let end = self.position + len;
        let token = Token {
            text: self.source.get(start..end).unwrap(), // This should never fail
            start,
            t: match c_type {
                CharType::WhiteSpace => TokenType::WhiteSpace,
                CharType::Word => TokenType::Word,
                CharType::Other => TokenType::SpecialCharacter,
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

// TODO: Insert BlockStart/BlockEnd for whitespace
// TODO: Eventually better parsing -- i.e. add BlockStart/BlockEnd for non-whitesace things
// TODO: Add line and col numbers to tokens

#[derive(Debug, Clone)]
enum AlignmentOperation<T> {
    Mutation { left: T, right: T },
    InsertLeft { left: T },
    InsertRight { right: T },
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

struct AlignmentData<'a> {
    score: f64,
    path: Rc<PathList<AlignmentOperation<&'a Token<'a, TokenType>>>>,
}

impl<'a> AlignmentData<'a> {
    pub fn new() -> Self {
        Self {
            score: 0.,
            path: Rc::new(PathList::End),
        }
    }
    pub fn unreachable() -> Self {
        Self {
            score: f64::INFINITY,
            path: Rc::new(PathList::End),
        }
    }
}

struct AlignmentState<'a> {
    last_was_mutation: AlignmentData<'a>,
    last_was_insert_left: AlignmentData<'a>,
    last_was_insert_right: AlignmentData<'a>,
}

impl<'a> AlignmentState<'a> {
    pub fn pick_best(
        &self,
        payload: AlignmentOperation<&'a Token<'a, TokenType>>,
        mutation_score: f64,
        insert_left_score: f64,
        insert_right_score: f64,
    ) -> AlignmentData<'a> {
        let (score, previous) = if insert_left_score < insert_right_score {
            if insert_left_score < mutation_score {
                (insert_left_score, self.last_was_insert_left.path.clone())
            } else {
                (mutation_score, self.last_was_mutation.path.clone())
            }
        } else {
            if insert_right_score < mutation_score {
                (insert_right_score, self.last_was_insert_right.path.clone())
            } else {
                (mutation_score, self.last_was_mutation.path.clone())
            }
        };
        AlignmentData {
            score,
            path: Rc::new(PathList::Node { payload, previous }),
        }
    }

    pub fn extract_best(self) -> AlignmentData<'a> {
        if self.last_was_mutation.score < self.last_was_insert_left.score {
            if self.last_was_mutation.score < self.last_was_insert_right.score {
                self.last_was_mutation
            } else {
                self.last_was_insert_right
            }
        } else {
            if self.last_was_insert_left.score < self.last_was_insert_right.score {
                self.last_was_insert_left
            } else {
                self.last_was_insert_right
            }
        }
    }

    pub fn insert_left_score(&self, l: &'a Token<'a, TokenType>) -> AlignmentData<'a> {
        let mutation_score = self.last_was_mutation.score + l.insert_score(false);
        let insert_left_score = self.last_was_insert_left.score + l.insert_score(true);
        let insert_right_score = self.last_was_insert_right.score + l.insert_score(false);
        self.pick_best(
            AlignmentOperation::InsertLeft { left: l },
            mutation_score,
            insert_left_score,
            insert_right_score,
        )
    }

    pub fn insert_right_score(&self, r: &'a Token<'a, TokenType>) -> AlignmentData<'a> {
        let mutation_score = self.last_was_mutation.score + r.insert_score(false);
        let insert_left_score = self.last_was_insert_left.score + r.insert_score(false);
        let insert_right_score = self.last_was_insert_right.score + r.insert_score(true);
        self.pick_best(
            AlignmentOperation::InsertRight { right: r },
            mutation_score,
            insert_left_score,
            insert_right_score,
        )
    }

    pub fn mutation_score(
        &self,
        l: &'a Token<'a, TokenType>,
        r: &'a Token<'a, TokenType>,
    ) -> AlignmentData<'a> {
        let s = l.mutation_score(r);
        let mutation_score = self.last_was_mutation.score + s;
        let insert_left_score = self.last_was_insert_left.score + s;
        let insert_right_score = self.last_was_insert_right.score + s;
        self.pick_best(
            AlignmentOperation::Mutation { left: l, right: r },
            mutation_score,
            insert_left_score,
            insert_right_score,
        )
    }
}

type AlignmentLineDS<'a> = Vec<AlignmentState<'a>>;

fn align<'a>(
    left: &'a [Token<'a, TokenType>],
    right: &'a [Token<'a, TokenType>],
) -> Alignment<'a, Token<'a, TokenType>> {
    let result_path = {
        let mut current: AlignmentLineDS<'a> = Vec::with_capacity(left.len() + 1);
        current.push(AlignmentState {
            last_was_mutation: AlignmentData::new(),
            last_was_insert_left: AlignmentData::unreachable(),
            last_was_insert_right: AlignmentData::unreachable(),
        });
        for l in left.iter() {
            let prev = current.last().unwrap();
            current.push(AlignmentState {
                last_was_mutation: AlignmentData::unreachable(),
                last_was_insert_left: prev.insert_left_score(l),
                last_was_insert_right: AlignmentData::unreachable(),
            })
        }
        let mut next = Vec::with_capacity(left.len() + 1);
        for r in right.iter() {
            let prev = &current[0];
            next.push(AlignmentState {
                last_was_mutation: AlignmentData::unreachable(),
                last_was_insert_left: AlignmentData::unreachable(),
                last_was_insert_right: prev.insert_right_score(r),
            });
            for (l_index, l) in left.iter().enumerate() {
                let l_index = l_index + 1;
                next.push(AlignmentState {
                    last_was_mutation: current[l_index - 1].mutation_score(l, r),
                    last_was_insert_left: next[l_index - 1].insert_left_score(l),
                    last_was_insert_right: current[l_index].insert_right_score(r),
                });
            }

            std::mem::swap(&mut current, &mut next);
            next.clear()
        }
        current.pop().unwrap().extract_best().path
    };
    Alignment {
        operations: Rc::try_unwrap(result_path)
            .unwrap_or_else(|x| {
                eprintln!("More than 1 reference!");
                (*x).clone()
            })
            .extract_path(),
    }
}

pub struct Alignment<'a, T> {
    operations: Vec<AlignmentOperation<&'a T>>,
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
    pub fn pretty(&self) {
        let mut left_line = String::new();
        let mut right_line = String::new();
        let flush = |left_line: &mut String, right_line: &mut String| {
            // TODO: add colors indicating what was added and what not.
            if left_line != right_line {
                if left_line.chars().any(|x| !x.is_whitespace()) {
                    println!("- {}", left_line);
                }
                if right_line.chars().any(|x| !x.is_whitespace()) {
                    println!("+ {}", right_line);
                }
            } else {
                println!("  {}", right_line);
            }
            left_line.clear();
            right_line.clear();
        };
        let mut prev_was_space = true;
        for operation in self.operations.iter() {
            prev_was_space = match operation {
                AlignmentOperation::Mutation { left, right } => {
                    // TODO: assuming here that newlines are
                    let left_text = left.text;
                    let right_text = right.text;
                    if left_text.to_lowercase() == right_text.to_lowercase() {
                        left_line.extend(left_text.chars().map(|_| ' '));
                        right_line.extend(right_text.chars());
                    } else {
                        left_line.extend(format!("{}", left_text.red()).chars());
                        right_line.extend(format!("{}", right_text.green()).chars());
                    }
                    if left_text.len() < right_text.len() {
                        for _ in 0..(right_text.len() - left_text.len()) {
                            left_line.push(' ');
                        }
                    } else {
                        for _ in 0..(left_text.len() - right_text.len()) {
                            right_line.push(' ');
                        }
                    }
                    false
                }
                AlignmentOperation::InsertLeft { left } => {
                    if left.t == TokenType::WhiteSpace {
                        // Ignoring whitespace for left
                        if !prev_was_space {
                            left_line.push(' ');
                            right_line.extend(format!("{}", " ".red().strikethrough()).chars());
                        }
                        true
                    } else {
                        let text = left.text;
                        left_line.extend(text.chars().map(|_| ' '));
                        right_line.extend(format!("{}", text.red().strikethrough()).chars());
                        false
                    }
                }
                AlignmentOperation::InsertRight { right } => {
                    if right.t == TokenType::WhiteSpace {
                        // TODO: handle whitespace
                        let whitespace = right.text;
                        if whitespace.contains('\n') {
                            let mut whitespace = whitespace.split('\n');
                            let first = whitespace.next().unwrap();
                            left_line.extend(first.chars());
                            right_line.extend(first.chars());
                            for space in whitespace {
                                flush(&mut left_line, &mut right_line);
                                left_line.extend(space.chars());
                                right_line.extend(space.chars());
                            }
                        } else {
                            left_line.extend(whitespace.chars());
                            right_line.extend(whitespace.chars());
                        }
                        true
                    } else {
                        let text = right.text;
                        left_line.extend(text.chars().map(|_| ' '));
                        right_line.extend(format!("{}", text.green()).chars());
                        false
                    }
                }
            }
        }
        flush(&mut left_line, &mut right_line);
    }

    pub fn add_tokens(
        &mut self,
        left: &'a [Token<'a, TokenType>],
        right: &'a [Token<'a, TokenType>],
    ) {
        let mut old_alignment =
            Vec::with_capacity(self.operations.len() + left.len() + right.len());
        std::mem::swap(&mut old_alignment, &mut self.operations);
        let mut left = left.iter().peekable();
        let mut right = right.iter().peekable();
        let mut left_position = None;
        let mut right_position = None;
        old_alignment.reverse();
        while let Some(a) = old_alignment.pop() {
            right_position = a.right().cloned().or(right_position);
            if let Some(right_position) = right_position {
                while right
                    .peek()
                    .map(|p| p.start < right_position.start)
                    .unwrap_or(false)
                {
                    right.next().map(|right| {
                        self.operations
                            .push(AlignmentOperation::InsertRight { right: &right })
                    });
                }
            }
            left_position = a.left().cloned().or(left_position);
            if let Some(left_position) = left_position {
                while left
                    .peek()
                    .map(|p| p.start < left_position.start)
                    .unwrap_or(false)
                {
                    left.next().map(|left| {
                        self.operations
                            .push(AlignmentOperation::InsertLeft { left: &left })
                    });
                }
            }
            self.operations.push(a);
        }
        self.operations
            .extend(right.map(|right| AlignmentOperation::InsertRight { right }));
        self.operations
            .extend(left.map(|left| AlignmentOperation::InsertLeft { left }));
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    debug: bool,
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
    if cli.debug {
        for op in alignment.operations.iter() {
            println!("{:?}", op);
        }
    }
    alignment.pretty();
}
