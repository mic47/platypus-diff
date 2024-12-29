use std::rc::Rc;

use colored::Colorize;

use crate::types::{AlignmentScoring, Token};

#[derive(Debug, Clone)]
pub enum AlignmentOperation<T> {
    Mutation { left: T, right: T },
    InsertLeft { left: T },
    InsertRight { right: T },
}

#[derive(Debug, Clone)]
pub enum PathList<T> {
    End,
    Node {
        payload: T,
        previous: Rc<PathList<T>>,
    },
}

impl<T: Clone> PathList<T> {
    pub fn extract_path(self) -> Vec<T> {
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

struct AlignmentData<'a, T> {
    score: f64,
    path: Rc<PathList<AlignmentOperation<&'a T>>>,
}

impl<'a, T> AlignmentData<'a, T> {
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

struct AlignmentState<'a, T> {
    last_was_mutation: AlignmentData<'a, T>,
    last_was_insert_left: AlignmentData<'a, T>,
    last_was_insert_right: AlignmentData<'a, T>,
}

impl<'a, T> AlignmentState<'a, T> {
    #[allow(clippy::collapsible_else_if)]
    pub fn pick_best(
        &self,
        payload: AlignmentOperation<&'a T>,
        mutation_score: f64,
        insert_left_score: f64,
        insert_right_score: f64,
    ) -> AlignmentData<'a, T> {
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

    #[allow(clippy::collapsible_else_if)]
    pub fn extract_best(self) -> AlignmentData<'a, T> {
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

    pub fn insert_left_score<S: AlignmentScoring<T>>(
        &self,
        scoring: &S,
        l: &'a T,
    ) -> AlignmentData<'a, T> {
        let mutation_score = self.last_was_mutation.score + scoring.insert_score(l, false);
        let insert_left_score = self.last_was_insert_left.score + scoring.insert_score(l, true);
        let insert_right_score = self.last_was_insert_right.score + scoring.insert_score(l, false);
        self.pick_best(
            AlignmentOperation::InsertLeft { left: l },
            mutation_score,
            insert_left_score,
            insert_right_score,
        )
    }

    pub fn insert_right_score<S: AlignmentScoring<T>>(
        &self,
        scoring: &S,
        r: &'a T,
    ) -> AlignmentData<'a, T> {
        let mutation_score = self.last_was_mutation.score + scoring.insert_score(r, false);
        let insert_left_score = self.last_was_insert_left.score + scoring.insert_score(r, false);
        let insert_right_score = self.last_was_insert_right.score + scoring.insert_score(r, true);
        self.pick_best(
            AlignmentOperation::InsertRight { right: r },
            mutation_score,
            insert_left_score,
            insert_right_score,
        )
    }

    pub fn mutation_score<S: AlignmentScoring<T>>(
        &self,
        scoring: &S,
        l: &'a T,
        r: &'a T,
    ) -> AlignmentData<'a, T> {
        let s = scoring.mutation_score(l, r);
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

type AlignmentLineDS<'a, T> = Vec<AlignmentState<'a, T>>;

pub fn align<'a, T, S: AlignmentScoring<T>>(
    scoring: &S,
    left: &'a [T],
    right: &'a [T],
) -> Alignment<'a, T> {
    let result_path = {
        let mut current: AlignmentLineDS<'a, T> = Vec::with_capacity(left.len() + 1);
        current.push(AlignmentState {
            last_was_mutation: AlignmentData::new(),
            last_was_insert_left: AlignmentData::unreachable(),
            last_was_insert_right: AlignmentData::unreachable(),
        });
        for l in left.iter() {
            let prev = current.last().unwrap();
            current.push(AlignmentState {
                last_was_mutation: AlignmentData::unreachable(),
                last_was_insert_left: prev.insert_left_score(scoring, l),
                last_was_insert_right: AlignmentData::unreachable(),
            })
        }
        let mut next = Vec::with_capacity(left.len() + 1);
        for r in right.iter() {
            let prev = &current[0];
            next.push(AlignmentState {
                last_was_mutation: AlignmentData::unreachable(),
                last_was_insert_left: AlignmentData::unreachable(),
                last_was_insert_right: prev.insert_right_score(scoring, r),
            });
            for (l_index, l) in left.iter().enumerate() {
                let l_index = l_index + 1;
                next.push(AlignmentState {
                    last_was_mutation: current[l_index - 1].mutation_score(scoring, l, r),
                    last_was_insert_left: next[l_index - 1].insert_left_score(scoring, l),
                    last_was_insert_right: current[l_index].insert_right_score(scoring, r),
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
    pub operations: Vec<AlignmentOperation<&'a T>>,
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

impl<'a, T: Token> Alignment<'a, T> {
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
                    let left_text = left.text();
                    let right_text = right.text();
                    if left_text.to_lowercase() == right_text.to_lowercase() {
                        left_line.extend(left_text.chars().map(|_| ' '));
                        right_line.push_str(right_text);
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
                    if left.is_whitespace() {
                        // Ignoring whitespace for left
                        if !prev_was_space {
                            left_line.push(' ');
                            right_line.extend(format!("{}", " ".red().strikethrough()).chars());
                        }
                        true
                    } else {
                        let text = left.text();
                        left_line.extend(text.chars().map(|_| ' '));
                        right_line.extend(format!("{}", text.red().strikethrough()).chars());
                        false
                    }
                }
                AlignmentOperation::InsertRight { right } => {
                    if right.is_whitespace() {
                        // TODO: handle whitespace
                        let whitespace = right.text();
                        if whitespace.contains('\n') {
                            let mut whitespace = whitespace.split('\n');
                            let first = whitespace.next().unwrap();
                            left_line.push_str(first);
                            right_line.push_str(first);
                            for space in whitespace {
                                flush(&mut left_line, &mut right_line);
                                left_line.push_str(space);
                                right_line.push_str(space);
                            }
                        } else {
                            left_line.push_str(whitespace);
                            right_line.push_str(whitespace);
                        }
                        true
                    } else {
                        let text = right.text();
                        left_line.extend(text.chars().map(|_| ' '));
                        right_line.extend(format!("{}", text.green()).chars());
                        false
                    }
                }
            }
        }
        flush(&mut left_line, &mut right_line);
    }

    pub fn interleave_tokens(mut self, left: &'a [T], right: &'a [T]) -> Self {
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
                    .map(|p| p.start() < right_position.start())
                    .unwrap_or(false)
                {
                    if let Some(right) = right.next() {
                        self.operations
                            .push(AlignmentOperation::InsertRight { right })
                    };
                }
            }
            left_position = a.left().cloned().or(left_position);
            if let Some(left_position) = left_position {
                while left
                    .peek()
                    .map(|p| p.start() < left_position.start())
                    .unwrap_or(false)
                {
                    if let Some(left) = left.next() {
                        self.operations
                            .push(AlignmentOperation::InsertLeft { left })
                    };
                }
            }
            self.operations.push(a);
        }
        self.operations
            .extend(right.map(|right| AlignmentOperation::InsertRight { right }));
        self.operations
            .extend(left.map(|left| AlignmentOperation::InsertLeft { left }));
        self
    }
}
