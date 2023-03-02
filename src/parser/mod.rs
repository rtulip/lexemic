mod parse_expr; 
mod error;
use std::collections::HashMap;

pub use parse_expr::*;
pub use error::*;

pub struct Parser<'a> {
    pub rules: HashMap<&'a str, (ParseExpr<'a>, bool)>,
    pub start: &'a str
}

impl<'a> Parser<'a> {
    pub fn parse(&self, source: &'a str) -> Result<parse_expr::ParseOut<'a>, ParseError<'a>> {
        match self.rules.get(&self.start) {
            Some((rule, group)) => {
                let mut idx = 0;
                rule.parse(self.start, group, self, source, &mut idx).into_result()
            },
            _ => todo!(),
        }
    }

    pub fn grammar_parser() -> Self {
        let grammar = ParseExpr::Sequence { es: vec![
            ParseExpr::OneOrMore { e: Box::new(
                ParseExpr::Atomic(AtomicExpr::NonTerminal("rule"))
            )},
            ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
            ParseExpr::Atomic(AtomicExpr::EndOfFile)
        ] };
    
        let rule = ParseExpr::Sequence { es: vec![
            ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
            ParseExpr::Optional { e: Box::new( ParseExpr::Atomic(AtomicExpr::Terminal("@")))},
            ParseExpr::Atomic(AtomicExpr::NonTerminal("non_terminal")),
            ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
            ParseExpr::Atomic(AtomicExpr::Terminal("=")),
            ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
            ParseExpr::Atomic(AtomicExpr::NonTerminal("sequence")),
            ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
            ParseExpr::Atomic(AtomicExpr::Terminal(";")),
            ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
        ]};
    
        let sequence = ParseExpr::Choice { es: vec![
            ParseExpr::Sequence { es: vec![
                ParseExpr::Atomic(AtomicExpr::NonTerminal("modifier")),
                ParseExpr::Atomic(AtomicExpr::Regex("\\s+")),
                ParseExpr::Atomic(AtomicExpr::NonTerminal("sequence")),
            ]},
            ParseExpr::Sequence { es: vec![
                ParseExpr::Atomic(AtomicExpr::NonTerminal("modifier")),
                ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
                ParseExpr::Atomic(AtomicExpr::Terminal("|")),
                ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
                ParseExpr::Atomic(AtomicExpr::NonTerminal("sequence")),
            ]},
            ParseExpr::Atomic(AtomicExpr::NonTerminal("modifier")),
        ]};
    
        let modifier = ParseExpr::Choice { es: vec![
            ParseExpr::Sequence { es: vec![
                ParseExpr::Atomic(AtomicExpr::Terminal("_")),
                ParseExpr::Atomic(AtomicExpr::NonTerminal("primary")),
            ]},
            ParseExpr::Sequence { es: vec![
                ParseExpr::Atomic(AtomicExpr::NonTerminal("primary")),
                ParseExpr::Atomic(AtomicExpr::Terminal("+")),
            ]},
            ParseExpr::Sequence { es: vec![
                ParseExpr::Atomic(AtomicExpr::NonTerminal("primary")),
                ParseExpr::Atomic(AtomicExpr::Terminal("*")),
            ]},
            ParseExpr::Sequence { es: vec![
                ParseExpr::Atomic(AtomicExpr::NonTerminal("primary")),
                ParseExpr::Atomic(AtomicExpr::Terminal("?")),
            ]},
            ParseExpr::Atomic(AtomicExpr::NonTerminal("primary")),
        ]};
    
        let primary = ParseExpr::Choice { es: vec![
            ParseExpr::Sequence { es: vec![
                ParseExpr::Atomic(AtomicExpr::Terminal("(")),
                ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
                ParseExpr::Atomic(AtomicExpr::NonTerminal("sequence")),
                ParseExpr::Atomic(AtomicExpr::Regex("\\s*")),
                ParseExpr::Atomic(AtomicExpr::Terminal(")")),
            ]},
            ParseExpr::Atomic(AtomicExpr::NonTerminal("atomic")),
        ]};
    
        let atomic = ParseExpr::Choice { es: vec![
            ParseExpr::Atomic(AtomicExpr::NonTerminal("terminal")),
            ParseExpr::Atomic(AtomicExpr::NonTerminal("regex")),
            ParseExpr::Atomic(AtomicExpr::NonTerminal("non_terminal")),
        ]};
    
        let regex = ParseExpr::Sequence { es: vec![
            ParseExpr::Atomic(AtomicExpr::Terminal("re")),
            ParseExpr::Atomic(AtomicExpr::NonTerminal("STRING")),
        ]};
    
        let non_terminal = ParseExpr::Sequence { es: vec![
            ParseExpr::Atomic(AtomicExpr::NonTerminal("ALPHA")),
            ParseExpr::ZeroOrMore { e: Box::new(ParseExpr::Choice { es: vec![
                ParseExpr::Atomic(AtomicExpr::NonTerminal("ALPHA")),
                ParseExpr::Atomic(AtomicExpr::NonTerminal("DIGIT")),
            ]})}
        ]};
    
        let terminal = ParseExpr::Atomic(AtomicExpr::NonTerminal("STRING"));
    
        let string = ParseExpr::Sequence { es: vec![
            ParseExpr::Atomic(AtomicExpr::Terminal("\"")),
            ParseExpr::ZeroOrMore{ e: Box::new(ParseExpr::Choice { es: vec![
                ParseExpr::Atomic(AtomicExpr::NonTerminal("escape")),
                ParseExpr::Atomic(AtomicExpr::NonTerminal("char")),
            ]})},
            ParseExpr::Atomic(AtomicExpr::Terminal("\"")),
        ]};
    
        let escape = ParseExpr::Sequence { es: vec![
            ParseExpr::Atomic(AtomicExpr::Terminal("\\")),
            ParseExpr::Atomic(AtomicExpr::Regex("\\S")),
        ]};
    
        let char = ParseExpr::Atomic(AtomicExpr::Regex("[^\\|\\\\\"]"));
    
        let alpha = ParseExpr::Atomic(AtomicExpr::Regex("[a-zA-Z_]"));
        let digit = ParseExpr::Atomic(AtomicExpr::Regex("[0-9]"));
    
        let parser = Parser {
            rules: HashMap::from([
                ("grammar", (grammar, false)),
                ("rule", (rule, false)),
                ("sequence", (sequence, false)),
                ("modifier", (modifier, false)),
                ("primary", (primary, false)),
                ("atomic", (atomic, false)),
                ("terminal", (terminal, false)),
                ("non_terminal", (non_terminal, true)),
                ("regex", (regex, false)),
                ("STRING", (string, true)),
                ("escape", (escape, false)),
                ("char", (char, false)),
                ("ALPHA", (alpha, false)),
                ("DIGIT", (digit, false)),
            ]),
            start: "grammar",
        };

        parser
    }
}