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
use std::iter;

use reedline::{DefaultPrompt, Reedline, Signal};
use slotmap::{DefaultKey, SlotMap};

type Symbol = String;
type Number = f64;
type Bool = bool;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum Atom {
    Symbol(Symbol),
    Number(Number),
    Bool(Bool),
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
    result.insert_fn("+", |_, list| Exp::num(list[0].as_number() + list[1].as_number()));
    result.insert_fn("-", |_, list| Exp::num(list[0].as_number() - list[1].as_number()));
    result.insert_fn("*", |_, list| Exp::num(list[0].as_number() * list[1].as_number()));
    result.insert_fn("/", |_, list| Exp::num(list[0].as_number() / list[1].as_number()));
    result.insert_fn("<=", |_, list| Exp::bool(list[0].as_number() <= list[1].as_number()));
    result.insert_fn(">=", |_, list| Exp::bool(list[0].as_number() >= list[1].as_number()));
    result.insert_fn("<", |_, list| Exp::bool(list[0].as_number() >= list[1].as_number()));
    result.insert_fn(">", |_, list| Exp::bool(list[0].as_number() >= list[1].as_number()));
    result.insert_fn("abs", |_, list| Exp::num(list[0].as_number().abs()));
    result.insert_fn("append", |_, list| {
        Exp::List(list.iter().flat_map(|x| x.as_exp_list()).collect())
    });
    result.insert_fn("apply", |env_tree, list| {
        list[0].invoke(env_tree, list.iter().skip(1).cloned().collect())
    });
    result.insert_fn("begin", |_, list| list[list.len() - 1].clone());
    result.insert_fn("car", |_, list| list[0].clone());
    result.insert_fn("cdr", |_, list| Exp::List(list.iter().skip(1).cloned().collect()));
    result.insert_fn("cons", |_, list| {
        Exp::List(
            iter::once(list[0].clone()).chain(list[1].as_exp_list().iter().cloned()).collect(),
        )
    });
    result.insert_fn("expt", |_, list| Exp::num(list[0].as_number().powf(list[1].as_number())));
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
    result.insert("pi", Exp::Atom(Atom::Number(consts::PI)));
    result
}

fn eval(x: Exp, env_tree: &mut EnvTree, env_id: EnvId) -> Exp {
    match x {
        Exp::Atom(Atom::Symbol(s)) => env_tree.get(env_id).unwrap().resolve(env_tree, s),
        Exp::Atom(Atom::Number(..)) => x,
        Exp::Atom(Atom::Bool(..)) => x,
        Exp::Function(..) => x,
        Exp::Procedure(..) => x,
        Exp::List(list) if list.is_empty() => panic!("Cannot evaluate empty list"),
        Exp::List(list) if list[0].as_symbol() == *"quote" => list[1].clone(),
        Exp::List(list) if list[0].as_symbol() == *"if" => {
            let result = if eval(list[1].clone(), env_tree, env_id).as_bool() {
                list[2].clone()
            } else {
                list[3].clone()
            };
            eval(result, env_tree, env_id)
        }
        Exp::List(list) if list[0].as_symbol() == *"define" => {
            let result = eval(list[2].clone(), env_tree, env_id);
            env_tree.get_mut(env_id).unwrap().insert(list[1].as_symbol(), result.clone());
            result
        }
        Exp::List(list) if list[0].as_symbol() == *"set!" => {
            let symbol = list[1].clone().as_symbol();
            let exp = list[2].clone();
            let evaluated = eval(exp, env_tree, env_id);
            let target_id = env_tree.get(env_id).unwrap().find(env_tree, symbol.clone(), env_id);
            env_tree.get_mut(target_id).unwrap().insert(symbol, evaluated);
            Exp::Atom(Atom::Bool(true))
        }
        Exp::List(list) if list[0].as_symbol() == *"lambda" => Exp::Procedure(Box::new(
            Procedure::new(list[1].as_symbol_list(), list[2].clone(), env_id),
        )),
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

pub fn run() {
    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::default();
    let mut env_tree = EnvTree::default();
    let standard_env_id = env_tree.insert(standard_env());

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                println!("Got: {}", buffer);
                let mut tokenized = tokenize(buffer);
                println!("Tokenized: {:?}", tokenized);
                let parsed = read_from_tokens(&mut tokenized);
                println!("Parsed: {:?}", parsed);
                let eval = eval(parsed, &mut env_tree, standard_env_id);
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
