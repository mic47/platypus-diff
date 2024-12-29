mod alignment;
mod tokenizer;

use std::path::PathBuf;

use clap::Parser;

use alignment::align;
use tokenizer::{TokenParser, TokenType};

// TODO: Insert BlockStart/BlockEnd for whitespace
// TODO: Eventually better parsing -- i.e. add BlockStart/BlockEnd for non-whitesace things
// TODO: Add line and col numbers to tokens

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
