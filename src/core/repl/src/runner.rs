// Copyright © Vow 2024-present

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//    https://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;

use reedline::{DefaultPrompt, Reedline, Signal};

type Symbol = String;
type Number = f64;
#[derive(Debug, Clone)]
enum Atom {
    Symbol(Symbol),
    Number(Number),
}
type List = Vec<Exp>;
#[derive(Debug, Clone)]
enum Exp {
    Atom(Atom),
    List(List),
}
type Env = HashMap<Symbol, Exp>;

fn tokenize(input: String) -> Vec<String> {
    input
        .replace('(', " ( ")
        .replace(')', " ) ")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

fn parse(program: String) -> Exp {
    read_from_tokens(&mut tokenize(program))
}

fn read_from_tokens(mut tokens: &mut Vec<String>) -> Exp {
    if tokens.is_empty() {
        panic!("Unexpected EOF!");
    }
    let token = tokens.remove(0);
    if token == "(" {
        let mut list = vec![];
        while tokens[0] != ")" {
            list.push(read_from_tokens(tokens));
        }
        tokens.remove(0); // Pop off ')'
        Exp::List(list)
    } else if token == ")" {
        panic!("Unexpected ')'!");
    } else {
        Exp::Atom(atom(token))
    }
}

fn atom(token: String) -> Atom {
    if let Ok(n) = token.parse::<f64>() {
        Atom::Number(n)
    } else {
        Atom::Symbol(token)
    }
}

pub fn run() {
    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::default();

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                println!("Got: {}", buffer);
                let mut tokenized = tokenize(buffer);
                println!("Tokenized: {:?}", tokenized);
                let parsed = read_from_tokens(&mut tokenized);
                println!("Parsed: {:?}", parsed);
            }
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("\nAborted!");
                break;
            }
            x => {
                println!("Event: {:?}", x);
            }
        }
    }
}
