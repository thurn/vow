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
use std::io::{BufRead, BufReader, Read};
use std::iter;
use std::str::FromStr;

use num_complex::Complex64;
use reedline::{DefaultPrompt, Reedline, Signal};
use regex::Regex;
use slotmap::{DefaultKey, SlotMap};

type Symbol = String;
type Number = f64;
type Bool = bool;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum Atom {
    Symbol(Symbol),
    Number(Number),
    Complex(Complex64),
    Bool(Bool),
    String(String),
}

type List = Vec<Exp>;

#[derive(Clone, Debug)]
enum Exp {
    Atom(Atom),
    List(List),
    Function(fn(&mut EnvTree, List) -> Exp),
    Procedure(Box<Procedure>),
}

impl Exp {
    fn num(number: Number) -> Self {
        Self::Atom(Atom::Number(number))
    }

    fn bool(b: Bool) -> Self {
        Self::Atom(Atom::Bool(b))
    }

    fn as_symbol(&self) -> Symbol {
        match self {
            Exp::Atom(Atom::Symbol(s)) => s.clone(),
            _ => panic!("Expected symbol!"),
        }
    }

    fn is_symbol(&self, symbol: impl Into<String>) -> bool {
        match self {
            Exp::Atom(Atom::Symbol(s)) => *s == symbol.into(),
            _ => false,
        }
    }

    fn as_exp_list(&self) -> Vec<Exp> {
        match self {
            Exp::List(list) => list.clone(),
            _ => panic!("Expected list"),
        }
    }

    fn as_symbol_list(&self) -> Vec<Symbol> {
        match self {
            Exp::List(list) => list.iter().map(|e| e.as_symbol()).collect(),
            _ => panic!("Expected list"),
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
            Exp::List(list) => !list.is_empty(),
            _ => panic!("Expected boolean!"),
        }
    }

    fn invoke(&self, env_tree: &mut EnvTree, args: List) -> Exp {
        match self {
            Exp::Function(f) => f(env_tree, args),
            Exp::Procedure(p) => p.invoke(env_tree, args),
            _ => panic!("Expected function!"),
        }
    }
}

impl PartialEq for Exp {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Exp::Atom(a), Exp::Atom(b)) => a == b,
            (Exp::List(a), Exp::List(b)) => a == b,
            _ => false,
        }
    }
}

pub type EnvId = DefaultKey;
type EnvTree = SlotMap<EnvId, Env>;

#[derive(Default)]
struct Env {
    outer: Option<EnvId>,
    symbols: HashMap<Symbol, Exp>,
}

impl Env {
    pub fn insert_into(
        env_tree: &mut EnvTree,
        parameters: Vec<Symbol>,
        arguments: List,
        outer: Option<EnvId>,
    ) -> EnvId {
        let mut result = Self::default();
        result.symbols.extend(parameters.iter().cloned().zip(arguments.iter().cloned()));
        result.outer = outer;
        env_tree.insert(result)
    }

    pub fn insert(&mut self, symbol: impl Into<String>, exp: Exp) {
        self.symbols.insert(symbol.into(), exp);
    }

    pub fn insert_fn(
        &mut self,
        symbol: impl Into<String>,
        function: fn(&mut EnvTree, List) -> Exp,
    ) {
        self.insert(symbol, Exp::Function(function))
    }

    pub fn get(&self, symbol: impl Into<String>) -> Exp {
        self.symbols.get(&symbol.into()).unwrap().clone()
    }

    pub fn resolve(&self, env_tree: &EnvTree, symbol: impl Into<String>) -> Exp {
        let s = symbol.into();
        if self.symbols.contains_key(&s) {
            self.get(s)
        } else if let Some(outer) = self.outer {
            if let Some(e) = env_tree.get(outer) {
                e.resolve(env_tree, s)
            } else {
                panic!("Env not found");
            }
        } else {
            panic!("Symbol not found {s}");
        }
    }

    pub fn find(&self, env_tree: &EnvTree, symbol: impl Into<String>, current: EnvId) -> EnvId {
        let s = symbol.into();
        if self.symbols.contains_key(&s) {
            current
        } else if let Some(outer) = self.outer {
            env_tree.get(outer).unwrap().find(env_tree, s, outer)
        } else {
            panic!("Symbol not found {s}");
        }
    }
}

#[derive(Clone, Debug)]
struct Procedure {
    pub parameters: Vec<Symbol>,
    pub body: Exp,
    pub env: EnvId,
}

impl Procedure {
    pub fn new(parameters: Vec<Symbol>, body: Exp, env: EnvId) -> Self {
        Self { parameters, body, env }
    }

    pub fn invoke(&self, env_tree: &mut EnvTree, arguments: List) -> Exp {
        let env_id = Env::insert_into(env_tree, self.parameters.clone(), arguments, Some(self.env));
        eval(self.body.clone(), env_tree, env_id)
    }
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

fn standard_env() -> Env {
    let mut result = Env::default();
    result.insert_fn("+", |_, list| Exp::num(list[0].as_number() + list[1].as_number()));
    result.insert_fn("-", |_, list| Exp::num(list[0].as_number() - list[1].as_number()));
    result.insert_fn("*", |_, list| Exp::num(list[0].as_number() * list[1].as_number()));
    result.insert_fn("/", |_, list| Exp::num(list[0].as_number() / list[1].as_number()));
    result.insert_fn("<=", |_, list| Exp::bool(list[0].as_number() <= list[1].as_number()));
    result.insert_fn(">=", |_, list| Exp::bool(list[0].as_number() >= list[1].as_number()));
    result.insert_fn("<", |_, list| Exp::bool(list[0].as_number() < list[1].as_number()));
    result.insert_fn(">", |_, list| Exp::bool(list[0].as_number() > list[1].as_number()));
    result.insert_fn("abs", |_, list| Exp::num(list[0].as_number().abs()));
    result.insert_fn("append", |_, list| {
        Exp::List(list.iter().flat_map(|x| x.as_exp_list()).collect())
    });
    result.insert_fn("apply", |env_tree, list| {
        list[0].invoke(env_tree, list.iter().skip(1).cloned().collect())
    });
    result.insert_fn("begin", |_, list| list[list.len() - 1].clone());
    result.insert_fn("car", |_, list| list[0].as_exp_list()[0].clone());
    result.insert_fn("cdr", |_, list| {
        Exp::List(list[0].as_exp_list().iter().skip(1).cloned().collect())
    });
    result.insert_fn("cons", |_, list| {
        Exp::List(
            iter::once(list[0].clone()).chain(list[1].as_exp_list().iter().cloned()).collect(),
        )
    });
    result.insert_fn("expt", |_, list| Exp::num(list[0].as_number().powf(list[1].as_number())));
    result.insert_fn("=", |_, list| Exp::bool(list[0].as_number() == list[1].as_number()));
    result.insert_fn("equal?", |_, list| Exp::bool(list[0] == list[1]));
    result.insert_fn("length", |_, list| Exp::num(list[0].as_exp_list().len() as f64));
    result.insert_fn("list", |_, list| Exp::List(list));
    result.insert_fn("list?", |_, list| Exp::bool(matches!(list[0], Exp::List(..))));
    result.insert_fn("map", |env_tree, list| {
        Exp::List(
            list[1]
                .as_exp_list()
                .iter()
                .map(|exp| list[0].invoke(env_tree, vec![exp.clone()]))
                .collect(),
        )
    });
    result.insert_fn("max", |_, list| {
        Exp::num(
            list.iter()
                .map(|exp| exp.as_number())
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .expect("Expected non-empty list"),
        )
    });
    result.insert_fn("min", |_, list| {
        Exp::num(
            list.iter()
                .map(|exp| exp.as_number())
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .expect("Expected non-empty list"),
        )
    });
    result.insert_fn("not", |_, list| Exp::bool(!list[0].as_bool()));
    result.insert_fn("null?", |_, list| Exp::bool(list[0].as_exp_list().is_empty()));
    result
        .insert_fn("number?", |_, list| Exp::bool(matches!(list[0], Exp::Atom(Atom::Number(..)))));
    result.insert_fn("print", |_, list| {
        println!("{:?}", list);
        Exp::List(vec![])
    });
    result.insert_fn("procedure?", |_, list| {
        Exp::bool(matches!(list[0], Exp::Function(..) | Exp::Procedure(..)))
    });
    result.insert_fn("round", |_, list| Exp::num(list[0].as_number().round()));
    result
        .insert_fn("symbol?", |_, list| Exp::bool(matches!(list[0], Exp::Atom(Atom::Symbol(..)))));
    result.insert("pi", Exp::Atom(Atom::Number(consts::PI)));
    result
}

fn eval(x: Exp, env_tree: &mut EnvTree, env_id: EnvId) -> Exp {
    match x {
        Exp::Atom(Atom::Symbol(s)) => env_tree.get(env_id).unwrap().resolve(env_tree, s),
        Exp::Atom(Atom::Number(..)) => x,
        Exp::Atom(Atom::Complex(..)) => x,
        Exp::Atom(Atom::Bool(..)) => x,
        Exp::Atom(Atom::String(..)) => x,
        Exp::Function(..) => x,
        Exp::Procedure(..) => x,
        Exp::List(list) if list.is_empty() => panic!("Cannot evaluate empty list"),
        Exp::List(list) if list[0].is_symbol("quote") => list[1].clone(),
        Exp::List(list) if list[0].is_symbol("if") => {
            let result = if eval(list[1].clone(), env_tree, env_id).as_bool() {
                list[2].clone()
            } else {
                list[3].clone()
            };
            eval(result, env_tree, env_id)
        }
        Exp::List(list) if list[0].is_symbol("define") => {
            let result = eval(list[2].clone(), env_tree, env_id);
            env_tree.get_mut(env_id).unwrap().insert(list[1].as_symbol(), result.clone());
            result
        }
        Exp::List(list) if list[0].is_symbol("set!") => {
            let symbol = list[1].clone().as_symbol();
            let exp = list[2].clone();
            let evaluated = eval(exp, env_tree, env_id);
            let target_id = env_tree.get(env_id).unwrap().find(env_tree, symbol.clone(), env_id);
            env_tree.get_mut(target_id).unwrap().insert(symbol, evaluated);
            Exp::Atom(Atom::Bool(true))
        }
        Exp::List(list) if list[0].is_symbol("lambda") => Exp::Procedure(Box::new(Procedure::new(
            list[1].as_symbol_list(),
            list[2].clone(),
            env_id,
        ))),
        Exp::List(list) => {
            let proc = eval(list[0].clone(), env_tree, env_id);
            let mut args: List = vec![];
            for x in list.iter().skip(1) {
                args.push(eval(x.clone(), env_tree, env_id));
            }
            proc.invoke(env_tree, args)
        }
    }
}

struct InPort<T: Read> {
    pub file: BufReader<T>,
    pub line: String,
}

impl<T: Read> InPort<T> {
    pub fn next_token(&mut self) -> Option<String> {
        loop {
            if self.line.is_empty() {
                let mut line = String::new();
                let result = self.file.read_line(&mut line).expect("Error reading line");
                if result == 0 {
                    return None;
                }
                self.line = line;
            }
            let re = Regex::new(r#"\s*(,@|[('`,)]|"(?:[\\].|[^\\"])*"|;.*|[^\s('"`,;)]*)(.*)"#)
                .expect("valid regex");
            let captures = re.captures(&self.line).expect("captures");
            let token = captures.get(1).expect("token capture").as_str().to_string();
            let line = captures.get(2).expect("line capture");
            self.line = line.as_str().to_string();
            if !token.is_empty() && !token.starts_with(';') {
                return Some(token);
            }
        }
    }
}

fn in_quotes(s: &str) -> bool {
    s == "'" || s == "`" || s == "," || s == ",@"
}

fn read_ahead<T: Read>(port: &mut InPort<T>, token: String) -> Exp {
    if token == "(" {
        let mut list: Vec<Exp> = vec![];
        loop {
            let Some(next) = port.next_token() else { panic!("End of Input") };
            if next == ")" {
                return Exp::List(list);
            } else {
                list.push(read_ahead(port, next));
            }
        }
    } else if token == ")" {
        panic!("Unexpected ')");
    } else if in_quotes(&token) {
        let Some(result) = read(port) else {
            panic!("Unexpected EOF");
        };
        Exp::List(vec![Exp::Atom(Atom::Symbol(token)), result])
    } else {
        Exp::Atom(atom(token))
    }
}

fn read<T: Read>(port: &mut InPort<T>) -> Option<Exp> {
    port.next_token().map(|t| read_ahead(port, t))
}

fn atom(token: String) -> Atom {
    if token == "#t" {
        return Atom::Bool(true);
    }

    if token == "#f" {
        return Atom::Bool(false);
    }

    if token.starts_with('"') {
        return Atom::String(token[1..=token.len() - 1].to_string());
    }

    if let Ok(n) = token.parse::<f64>() {
        Atom::Number(n)
    } else if let Ok(n) = Complex64::from_str(&token) {
        Atom::Complex(n)
    } else {
        Atom::Symbol(token)
    }
}

fn to_string(x: &Exp) -> String {
    match x {
        Exp::Atom(Atom::Bool(true)) => "#t".to_string(),
        Exp::Atom(Atom::Bool(false)) => "#f".to_string(),
        Exp::Atom(Atom::Symbol(s)) => s.clone(),
        Exp::Atom(Atom::Number(n)) => format!("{n}"),
        Exp::Atom(Atom::Complex(n)) => format!("{n}"),
        Exp::Atom(Atom::String(s)) => format!("\"{s}\""),
        Exp::List(list) => {
            format!("({})", list.iter().map(to_string).collect::<Vec<_>>().join(" "))
        }
        Exp::Function(_) => "<function>".to_string(),
        Exp::Procedure(_) => "<procedure>".to_string(),
    }
}

fn parse<T: Read>(input: &mut InPort<T>) -> Option<Exp> {
    read(input)
}

pub fn run() {
    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::default();
    let mut env_tree = EnvTree::default();
    let standard_env_id = env_tree.insert(standard_env());

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                let input = BufReader::new(buffer.as_bytes());
                let mut port = InPort { file: input, line: "".to_string() };
                loop {
                    let x = parse(&mut port);
                    match x {
                        None => {
                            break;
                        }
                        Some(exp) => {
                            let result = eval(exp, &mut env_tree, standard_env_id);
                            println!("{}", to_string(&result))
                        }
                    }
                }
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
