#![feature(try_trait_v2)]
#![feature(box_patterns)]

mod parser;

use parser::*;
use std::collections::HashMap;

enum TextColour {
    None,
    Red,
    Green,
}

struct Modification<'a> {
    string: &'a str,
    colour: TextColour,
    underline: char,
}

fn modify_source<'a>(modifications: &Vec<Modification<'a>>) -> (String, String) {
    (
        modifications
            .iter()
            .map(|m| match m.colour {
                TextColour::None => format!("{}", m.string),
                TextColour::Green => format!("\x1b[32m{}\x1b[0m", m.string),
                _ => todo!(),
            })
            .collect(),
        modifications
            .iter()
            .map(|m| match m.colour {
                TextColour::None => {
                    let mut s = String::new();
                    m.string.chars().for_each(|_| s.push(m.underline));
                    s
                }
                TextColour::Green => {
                    let mut s = String::from("\x1b[32m");
                    m.string.chars().for_each(|_| s.push(m.underline));
                    s.push_str("\x1b[0m");
                    s
                }
                _ => todo!(),
            })
            .collect(),
    )
}

fn main() -> Result<(), ParseError<String>> {
    let grammar_parser = Parser::grammar_parser();
    let grammar_source = "
        prog       = param_list EOF ;
        param      = (type \":\" ident) ;
        param_list = (\"(\" param+ \")\" )
                   | (\"(\" type+ \")\")
                   | (\"(\" \")\")
                   ;
        type        = ident ;
        @ident     = ALPHA (_ALPHA | _DIGIT)* ;
        ALPHA      = _re\"[a-zA-Z_]\" ;
        DIGIT      = _re\"[0-9]\" ;
    ";

    // Error: Expected `:` here.
    // [line 0]: (u64: foo bool   :    bar    bat      )
    //                                           ^
    // Note: In a parameter list, every parameter must have a name.
    // [line 0]: (u64: foo bool   :    bar    bat: name)
    //                                           ~~~~~~
    // Help: Add a name to parameter `bat`.
    let hints_source = "
        *::param_list::param => [
            \"Note\": \"In a parameter list, every parameter must have an identifier.\",
            source_hint: ,
            \"Help\": \"Add a name to parameter `{param::0}`.\"

        ];
    ";
    let out = grammar_parser.parse(grammar_source)?;
    let generated_parser = grammar_into_parser(out);
    let source = "(u64: foo bool   :    bar    bat      )";
    let x = generated_parser.parse(source)?;
    println!("{x:#?}");

    let source = "(u64: foo bool   :    bar    bat      )";
    let bat = source.find("bat").unwrap() + "bat".len();
    let modifications = vec![
        Modification {
            string: &source[0..bat],
            colour: TextColour::None,
            underline: ' ',
        },
        Modification {
            string: ": ident",
            colour: TextColour::Green,
            underline: '+',
        },
        Modification {
            string: &source[bat..],
            colour: TextColour::None,
            underline: ' ',
        },
    ];

    let (s, u) = modify_source(&modifications);
    println!("{s}\n{u}");

    Ok(())
}

fn grammar_into_parser<'a>(out: ParseOut<'a>) -> Parser<'a> {
    assert_eq!(out.rule, "grammar");

    if let ParseGrouping::Sequence { mut ts } = out.out {
        ts.pop();
        ts.pop();

        let rules = ts.pop().expect("Should have rules");
        let mut map = HashMap::new();
        let mut start = None;
        match rules.out {
            ParseGrouping::Sequence { ts: rules } => {
                for rule in rules {
                    let (id, meta, rule) = rule_into_parse_expr(rule);
                    map.insert(id, (rule, meta));
                    if start.is_none() {
                        start = Some(id);
                    }
                }
            }
            _ => unreachable!(),
        }

        Parser {
            rules: map,
            start: start.expect("Should have at least one rule"),
        }
    } else {
        unreachable!()
    }
}

fn rule_into_parse_expr<'a>(out: ParseOut<'a>) -> (&'a str, bool, ParseExpr<'a>) {
    assert_eq!(out.rule, "rule");
    match out.out {
        ParseGrouping::Sequence { mut ts } => {
            ts.pop(); // ws
            ts.pop(); // ";"
            ts.pop(); // ws
            let sequence = ts.pop().expect("Expected a seqeuence");
            ts.pop(); // ws
            ts.pop(); // "="
            ts.pop(); // ws
            let non_terminal = ts.pop().expect("Expected a non terminal");
            let meta = ts.pop().expect("Expected some meta...");

            let meta = match meta.out {
                ParseGrouping::Optional(None) => false,
                ParseGrouping::Optional(Some(box ParseGrouping::Terminal("@"))) => true,
                _ => todo!(),
            };

            let id = match non_terminal.out {
                ParseGrouping::Terminal(id) => id,
                s => unreachable!("{s:?}"),
            };

            let parse_expr = seqeuence_into_parse_expr(sequence);
            (id, meta, parse_expr)
        }
        _ => unreachable!(),
    }
}

fn seqeuence_into_parse_expr<'a>(out: ParseOut<'a>) -> ParseExpr<'a> {
    assert_eq!(out.rule, "sequence");
    match out.out {
        ParseGrouping::Out(out) => match &out.rule {
            &"sequence" => seqeuence_into_parse_expr(*out),
            &"modifier" => modifier_into_parse_expr(*out, true),
            _ => unreachable!(),
        },
        ParseGrouping::Sequence { mut ts } => {
            match ts.len() {
                3 => {
                    let sequence = ts.pop().expect("Expected a sequence");
                    ts.pop(); // ws
                    let modifier = ts.pop().expect("Expected a modifier");

                    let e1 = modifier_into_parse_expr(modifier, true);
                    let e2 = seqeuence_into_parse_expr(sequence);
                    ParseExpr::Sequence { es: vec![e1, e2] }
                }
                5 => {
                    let sequence = ts.pop().unwrap();
                    ts.pop();
                    ts.pop();
                    ts.pop();
                    let modifier = ts.pop().unwrap();

                    let e1 = modifier_into_parse_expr(modifier, true);
                    let e2 = seqeuence_into_parse_expr(sequence);

                    ParseExpr::Choice { es: vec![e1, e2] }
                }
                _ => unreachable!(),
            }
        }
        _ => todo!(),
    }
}

fn modifier_into_parse_expr<'a>(out: ParseOut<'a>, allow_whitespace: bool) -> ParseExpr<'a> {
    assert_eq!(out.rule, "modifier");
    match out.out {
        ParseGrouping::Out(out) => match out.rule {
            "primary" => primary_into_parse_expr(*out, allow_whitespace),
            "modifier" => modifier_into_parse_expr(*out, allow_whitespace),
            _ => unreachable!(),
        },
        ParseGrouping::Sequence { mut ts } => {
            let modifier = ts.pop().expect("Expected one of +, *, ?");
            let primary = ts.pop().expect("Expected primary");

            if let ParseGrouping::Terminal(modifier) = modifier.out {
                match modifier {
                    "+" => ParseExpr::OneOrMore {
                        e: Box::new(primary_into_parse_expr(primary, true)),
                    },
                    "*" => ParseExpr::ZeroOrMore {
                        e: Box::new(primary_into_parse_expr(primary, true)),
                    },
                    "?" => ParseExpr::Optional {
                        e: Box::new(primary_into_parse_expr(primary, true)),
                    },
                    _ => unreachable!(),
                }
            } else if let ParseGrouping::Terminal("_") = primary.out {
                primary_into_parse_expr(modifier, false)
            } else {
                unreachable!()
            }
        }
        x => todo!("{x:?}"),
    }
}

fn primary_into_parse_expr<'a>(out: ParseOut<'a>, allow_whitespace: bool) -> ParseExpr<'a> {
    assert_eq!(out.rule, "primary");
    match out.out {
        ParseGrouping::Out(out) => match out.rule {
            "primary" => primary_into_parse_expr(*out, allow_whitespace),
            "atomic" => atomic_into_parse_expr(*out, allow_whitespace),
            _ => unreachable!(),
        },
        ParseGrouping::Sequence { mut ts } => {
            ts.pop();
            ts.pop();
            let sequence = ts.pop().unwrap();
            ts.pop();
            ts.pop();
            seqeuence_into_parse_expr(sequence)
        }
        _ => todo!(),
    }
}

fn atomic_into_parse_expr<'a>(out: ParseOut<'a>, allow_whitespace: bool) -> ParseExpr<'a> {
    assert_eq!(out.rule, "atomic");
    let e = match out.out {
        ParseGrouping::Out(out) => match (out.rule, out.out) {
            ("regex", ParseGrouping::Sequence { ts }) => {
                if let Some(ParseOut {
                    out: ParseGrouping::Terminal(term),
                    ..
                }) = ts.get(1)
                {
                    ParseExpr::Atomic(AtomicExpr::Regex(&term[1..term.len() - 1]))
                } else {
                    todo!("err...")
                }
            }
            ("non_terminal", ParseGrouping::Terminal(term)) => {
                if term == "EOF" {
                    ParseExpr::Atomic(AtomicExpr::EndOfFile)
                } else {
                    ParseExpr::Atomic(AtomicExpr::NonTerminal(term))
                }
            }
            ("terminal" | "STRING", ParseGrouping::Terminal(term)) => {
                ParseExpr::Atomic(AtomicExpr::Terminal(&term[1..term.len() - 1]))
            }
            (r, o) => unreachable!("{r}, {o:?}"),
        },
        _ => unreachable!(),
    };

    if allow_whitespace {
        ParseExpr::Sequence {
            es: vec![ParseExpr::Atomic(AtomicExpr::Regex("\\s*")), e],
        }
    } else {
        e
    }
}
