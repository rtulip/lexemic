use regex::Regex;
use serde::Serialize;

use super::{ParseError, Fallible};

#[derive(Debug)]
pub enum AtomicExpr<'a> {
    Terminal(&'a str),
    Regex(&'a str),
    NonTerminal(&'a str),
    EndOfFile,
}

impl<'a> AtomicExpr<'a> {
    pub fn parse(&self, rule: &'a str, parser: &super::Parser<'a>, source: &'a str, idx: &mut usize) -> Fallible<ParseOut<'a>, ParseError<'a>> {
        match self {
            AtomicExpr::NonTerminal(non_term) => match parser.rules.get(non_term) {
                Some((expr, group)) => {
                    expr.parse(non_term, group, parser, source, idx) 
                },
                _ => return Fallible::Err(ParseError::UnknownNonTerminal(non_term)),
            }
            AtomicExpr::Terminal(term) => if source[*idx..].starts_with(term) {
                let s = &source[*idx..*idx+term.len()];
                *idx += term.len();
                Fallible::Ok(ParseOut {
                    rule,
                    out: ParseGrouping::Terminal(s)
                })
            } else {
                Fallible::Err(ParseError::new_bad_match(
                    source, 
                    idx, 
                    format!("Expected `{term}` here."), 
                    vec![term],
                ))
            },
            AtomicExpr::Regex(re_str) => {
                let re = Regex::new(re_str).unwrap();

                match re.find(&source[*idx..]) {
                    Some(m) => {
                        if m.start() != 0 {
                            return Fallible::Err(ParseError::new_bad_match(
                                source, 
                                idx, 
                                format!("Failed to match `{re_str}`."), 
                                vec![re_str])
                            );
                        }
                        let s = &source[*idx..*idx+m.end()];
                        *idx += m.end();
                        Fallible::Ok(ParseOut {
                            rule,
                            out: ParseGrouping::Terminal(s)
                        })
                    },
                    None => {
                        return Fallible::Err(ParseError::new_bad_match(
                            source, 
                            idx, 
                            format!("Failed to match `{re_str}`."), 
                            vec![re_str])
                        );
                    } 
                }
            },
            AtomicExpr::EndOfFile => if *idx + 1 >= source.len() {
                Fallible::Ok(ParseOut {
                    rule,
                    out: ParseGrouping::Terminal("EOF")
                })
            } else {
                todo!();
            }
        }
    }
}


#[derive(Debug)]
pub enum ParseExpr<'a> {
    Atomic(AtomicExpr<'a>),
    Sequence { es: Vec<ParseExpr<'a>> },
    Choice { es: Vec<ParseExpr<'a>> },
    ZeroOrMore { e: Box<ParseExpr<'a>> },
    OneOrMore { e: Box<ParseExpr<'a>> },
    Optional { e: Box<ParseExpr<'a>> },
}

impl <'a> ParseExpr<'a> {
    pub fn parse(&self, rule: &'a str, group: &bool, parser: &super::Parser<'a>, source: &'a str, idx: &mut usize) -> Fallible<ParseOut<'a>, ParseError<'a>> {
        let x = match self {
            ParseExpr::Atomic(atomic) => atomic.parse(rule, parser, source, idx),
            ParseExpr::Choice { es } => {
                let mut errors = vec![];
                for e in es {
                    match e.parse(rule, group, parser, source, idx) {
                        Fallible::Ok(s) => return Fallible::Ok(ParseOut { rule, out: ParseGrouping::Out(Box::new(s)) }),
                        Fallible::Recovered(s, e ) => {
                            errors.push(e);
                            return Fallible::Recovered(
                                ParseOut { rule, out: ParseGrouping::Out(Box::new(s)) }, 
                                ParseError::collect_furthest(errors)?.unwrap()
                            );
                        },
                        Fallible::Err(e) => errors.push(e),
                    }
                }

                Fallible::Err(ParseError::collect_furthest(errors)?.unwrap())
            },
            ParseExpr::OneOrMore { e } | ParseExpr::ZeroOrMore { e } => {
                let prev_idx = *idx;
                let mut outs = if matches!(self, ParseExpr::OneOrMore{ .. }){
                    vec![e.parse(rule, group, parser, source, idx)?]
                } else {
                    vec![]
                };
                let mut errors = vec![];
                loop {
                    match e.parse(rule, group, parser, source, idx) {
                        Fallible::Ok(out) => outs.push(out),
                        Fallible::Recovered(out, e) => {
                            outs.push(out);
                            errors.push(e)
                        },
                        Fallible::Err(e) => {
                            errors.push(e);
                            break;
                        }
                    }
                }
                
                let err = ParseError::collect_furthest(errors)?.expect("One or More should have at least one error.");

                if *group {
                    let s = &source[prev_idx..*idx];
                    Fallible::Recovered(ParseOut { rule, out: ParseGrouping::Terminal(s) }, err)
                } else {
                    Fallible::Recovered(ParseOut { rule, out: ParseGrouping::Sequence{ ts: outs} }, err)
                }
                
            },
            ParseExpr::Optional { e } => {
                match e.parse(rule, group, parser, source, idx){
                    Fallible::Ok(ParseOut { out, .. }) =>  Fallible::Ok(
                        ParseOut{
                            rule, 
                            out: ParseGrouping::Optional(Some(Box::new(out)))
                        }
                    ),
                    Fallible::Recovered(ParseOut { out, .. }, e) => Fallible::Recovered(
                        ParseOut{
                            rule, 
                            out: ParseGrouping::Optional(Some(Box::new(out)))
                        }, 
                        e
                    ),
                    Fallible::Err(e) => Fallible::Recovered(ParseOut{rule, out: ParseGrouping::Optional(None)}, e),
                }
            },
            ParseExpr::Sequence { es } => {
                let start_idx = *idx;
                let mut s = vec![];
                let mut errors = vec![];
                for e in es {
                    match e.parse(rule, group, parser, source, idx) {
                        Fallible::Ok(out) => s.push(out),
                        Fallible::Recovered(out, e) => {
                            s.push(out);
                            errors.push(e);
                        }
                        Fallible::Err(e) => {
                            *idx = start_idx;
                            errors.push(e);

                            return Fallible::Err(ParseError::collect_furthest(errors)?.unwrap());
                        }
                    }
                }

                let err = ParseError::collect_furthest(errors)?;  
                let out = if *group {
                    ParseOut { rule, out: ParseGrouping::Terminal(&source[start_idx..*idx]) }
                } else {
                    ParseOut { rule, out: ParseGrouping::Sequence { ts: s } }
                };
                
                match err {
                    Some(e) => Fallible::Recovered(out, e),
                    None => Fallible::Ok(out),
                }
                
            }
        }; 

        x 
    }

}

#[derive(Debug, Serialize)]
pub enum ParseGrouping<'a> {
    Terminal(&'a str),
    Sequence { ts: Vec<ParseOut<'a>> },
    Optional(Option<Box<ParseGrouping<'a>>>),
    Out(Box<ParseOut<'a>>)
}

#[derive(Debug, Serialize)]
pub struct ParseOut<'a> {
    pub rule: &'a str,
    pub out: ParseGrouping<'a>,
}