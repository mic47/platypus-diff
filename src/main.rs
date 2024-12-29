mod alignment;
mod tokenizer;
mod types;

use std::path::PathBuf;

use clap::Parser;

use alignment::align;
use tokenizer::{Token, TokenParser, TokenType};
use types::AlignmentScoring;

// TODO: Insert BlockStart/BlockEnd for whitespace
// TODO: Eventually better parsing -- i.e. add BlockStart/BlockEnd for non-whitesace things
// TODO: Add line and col numbers to tokens

struct AffineScoring {
    pub start_insert: f64,
    pub extend_insert: f64,
    pub block_end_insert_penalty: f64,
    pub mismatched_type_penalty: f64,
    pub mismatched_text_penalty: f64,
}

impl<'a> AlignmentScoring<Token<'a, TokenType>> for AffineScoring {
    fn insert_score(&self, inserted: &Token<'a, TokenType>, previous_is_same: bool) -> f64 {
        let add = match inserted.t {
            TokenType::BlockEnd(_indent) => self.block_end_insert_penalty,
            _ => 0.0,
        };
        if previous_is_same {
            self.extend_insert + add
        } else {
            self.start_insert + add
        }
    }

    fn mutation_score(&self, left: &Token<'a, TokenType>, right: &Token<'a, TokenType>) -> f64 {
        if left.t != right.t {
            return self.mismatched_type_penalty;
        }
        match left.t {
            TokenType::BlockStart(indent) | TokenType::BlockEnd(indent) => match right.t {
                TokenType::BlockStart(o_indent) | TokenType::BlockEnd(o_indent) => {
                    // TODO: this is weird scoring. Indenting block should not penalize further
                    // indentation changes in that block, only the start / end.
                    indent.abs_diff(o_indent) as f64
                }
                _ => {
                    panic!("This is impossible");
                }
            },
            TokenType::WhiteSpace | TokenType::SpecialCharacter | TokenType::Word => {
                if left.text.to_lowercase() == right.text.to_lowercase() {
                    0.
                } else {
                    self.mismatched_text_penalty
                }
            }
        }
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
    let scoring = AffineScoring {
        start_insert: 0.7,
        extend_insert: 0.3,
        block_end_insert_penalty: 1.,
        mismatched_type_penalty: 100.,
        mismatched_text_penalty: 1.,
    };
    let mut alignment = align(&scoring, &left_tokens, &right_tokens);
    alignment.add_tokens(&left_whitespaces, &right_whitespaces);
    if cli.debug {
        for op in alignment.operations.iter() {
            println!("{:?}", op);
        }
    }
    alignment.pretty();
}
