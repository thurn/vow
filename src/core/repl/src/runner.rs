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

#![allow(dead_code)]

use std::collections::HashMap;
use std::f64::consts;
use std::sync::{Arc, Mutex};

use reedline::{DefaultPrompt, Reedline, Signal};

type Symbol = String;
type Number = f64;
type Bool = bool;

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Atom {
    Symbol(Symbol),
    Number(Number),
    Bool(Bool),
}

type List = Vec<Exp>;

#[derive(Debug, Clone)]
enum Exp {
    Atom(Atom),
    List(List),
    Function(fn(List) -> Exp),
}

impl Exp {
    fn num(number: Number) -> Self {
        Self::Atom(Atom::Number(number))
    }

    fn as_symbol(&self) -> Symbol {
        match self {
            Exp::Atom(Atom::Symbol(s)) => s.clone(),
            _ => panic!("Expected symbol!"),
        }
    }

    fn as_number(&self) -> Number {
        match self {
            Exp::Atom(Atom::Number(n)) => *n,
            _ => panic!("Expected number!"),
        }
    }

    fn as_bool(&self) -> Bool {
        match self {
            Exp::Atom(Atom::Bool(b)) => *b,
            _ => panic!("Expected boolean!"),
        }
    }

    fn as_fn(&self) -> fn(List) -> Exp {
        match self {
            Exp::Function(f) => *f,
            _ => panic!("Expected function!"),
        }
    }
}

#[derive(Default)]
struct Env {
    outer: Arc<Mutex<Env>>,
    symbols: HashMap<Symbol, Exp>,
}

impl Env {
    pub fn new(parameters: Vec<Symbol>, arguments: List, outer: Arc<Mutex<Env>>) -> Self {
        let mut result = Self::default();
        result.symbols.extend(parameters.iter().cloned().zip(arguments.iter().cloned()));
        result.outer = outer;
        result
    }

    pub fn insert(&mut self, symbol: impl Into<String>, exp: Exp) {
        self.symbols.insert(symbol.into(), exp);
    }

    pub fn insert_fn(&mut self, symbol: impl Into<String>, function: fn(List) -> Exp) {
        self.insert(symbol, Exp::Function(function))
    }

    pub fn get(&self, symbol: impl Into<String>) -> Exp {
        self.symbols.get(&symbol.into()).unwrap().clone()
    }

    pub fn resolve(&self, symbol: impl Into<String>) -> Exp {
        let s = symbol.into();
        if self.symbols.contains_key(&s) {
            self.get(s)
        } else {
            self.outer.lock().unwrap().resolve(s)
        }
    }

    pub fn find_mut(&mut self, _symbol: impl Into<String>) -> Arc<Mutex<Env>> {
        todo!("")
    }
}

struct Procedure {
    pub parameters: Vec<Symbol>,
    pub body: Exp,
    pub env: Arc<Mutex<Env>>,
}

impl Procedure {
    pub fn new(parameters: Vec<Symbol>, body: Exp, env: Arc<Mutex<Env>>) -> Self {
        Self { parameters, body, env }
    }

    pub fn invoke(&self, arguments: List) {
        eval(
            self.body.clone(),
            &mut Env::new(self.parameters.clone(), arguments, self.env.clone()),
        );
    }
}

fn tokenize(input: String) -> Vec<String> {
    input
        .replace('(', " ( ")
        .replace(')', " ) ")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

#[allow(dead_code)]
fn parse(program: String) -> Exp {
    read_from_tokens(&mut tokenize(program))
}

fn read_from_tokens(tokens: &mut Vec<String>) -> Exp {
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

fn standard_env() -> Env {
    let mut result = Env::default();
    result.insert_fn("+", |list| Exp::num(list[0].as_number() + list[1].as_number()));
    result.insert_fn("*", |list| Exp::num(list[0].as_number() * list[1].as_number()));
    result.insert("pi", Exp::Atom(Atom::Number(consts::PI)));
    result.insert_fn("begin", |list| list[list.len() - 1].clone());
    result
}

fn eval(x: Exp, env: &mut Env) -> Exp {
    match x {
        Exp::Atom(Atom::Symbol(s)) => env.resolve(s),
        Exp::Atom(Atom::Number(..)) => x,
        Exp::Atom(Atom::Bool(..)) => x,
        Exp::Function(..) => x,
        Exp::List(list) if list.is_empty() => panic!("Cannot evaluate empty list"),
        Exp::List(list) if list[0].as_symbol() == *"quote" => list[1].clone(),
        Exp::List(list) if list[0].as_symbol() == *"if" => {
            let result = if eval(list[1].clone(), env).as_bool() {
                list[2].clone()
            } else {
                list[3].clone()
            };
            eval(result, env)
        }
        Exp::List(list) if list[0].as_symbol() == *"define" => {
            let result = eval(list[2].clone(), env);
            env.insert(list[1].as_symbol(), result.clone());
            result
        }
        Exp::List(list) if list[0].as_symbol() == *"set!" => {
            let _symbol = list[1].clone().as_symbol();
            let exp = list[2].clone();

            // env.find_mut(symbol.clone()).insert(symbol, evaluated.clone());
            eval(exp, env)
        }
        Exp::List(list) => {
            let proc = eval(list[0].clone(), env);
            let mut args: List = vec![];
            for x in list.iter().skip(1) {
                args.push(eval(x.clone(), env));
            }
            proc.as_fn()(args)
        }
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
                let eval = eval(parsed, &mut standard_env());
                println!("Result: {:?}", eval);
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
