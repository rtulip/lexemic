use std::collections::BTreeSet;

pub enum Fallible<T, E> {
    Ok(T),
    Recovered(T, E),
    Err(E),
}

impl<T, E> Fallible<T, E> {
    pub fn into_result(self) -> Result<T, E> {
        match self {
            Fallible::Ok(t) | Fallible::Recovered(t, _) => Ok(t),
            Fallible::Err(e) => Err(e),
        }
    }
}

impl<T, E> std::ops::FromResidual for Fallible<T, E> {
    fn from_residual(residual: <Self as std::ops::Try>::Residual) -> Self {
        Fallible::Err(residual)
    }
}

impl<T, E> std::ops::Try for Fallible<T, E> {
    type Output = T;
    type Residual = E;

    fn from_output(output: Self::Output) -> Self {
        Fallible::Ok(output)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            Fallible::Ok(t) | Fallible::Recovered(t, _) => std::ops::ControlFlow::Continue(t),
            Fallible::Err(e) => std::ops::ControlFlow::Break(e),
        }
    }
}

#[derive(Clone)]
pub enum ParseError<Source> {
    UnknownNonTerminal(Source),
    BadMatchError {
        line: Source,
        col: usize,
        idx: usize,
        msg: String,
        terminals: BTreeSet<Source>,
        rules: Vec<Source>,
    },
}

impl<'a> ParseError<&'a str> {
    pub fn new_bad_match<S: Into<String>>(
        source: &'a str,
        idx: &usize,
        msg: S,
        terminals: Vec<&'a str>,
        rules: Vec<&'a str>,
    ) -> ParseError<&'a str> {
        let prev_newline = source[0..*idx].rfind("\n");
        let next_newline = source[*idx..].find("\n");
        let (line, col) = match (prev_newline, next_newline) {
            (Some(prev), Some(next)) => {
                if prev + 1 < *idx {
                    (&source[prev + 1..*idx + next], *idx - prev - 1)
                } else {
                    todo!()
                }
            }
            (None, Some(_)) => todo!(),
            (Some(prev), None) => {
                if prev + 1 < *idx {
                    (&source[prev + 1..], *idx - prev - 1)
                } else {
                    todo!()
                }
            }
            (None, None) => (source, *idx),
        };

        ParseError::BadMatchError {
            line,
            col,
            idx: *idx,
            msg: msg.into(),
            terminals: BTreeSet::from_iter(terminals),
            rules: rules,
        }
    }

    pub fn collect_furthest(
        errors: Vec<ParseError<&'a str>>,
    ) -> Fallible<Option<ParseError<&'a str>>, ParseError<&'a str>> {
        if errors.is_empty() {
            return Fallible::Ok(None);
        }

        let mut sizes = vec![];
        for e in &errors {
            match e {
                ParseError::UnknownNonTerminal(_) => return Fallible::Err(e.clone()),
                ParseError::BadMatchError { idx, .. } => sizes.push(idx),
            }
        }

        let max = errors
            .iter()
            .map(|e| match e {
                ParseError::UnknownNonTerminal(_) => unreachable!(),
                ParseError::BadMatchError { idx, .. } => *idx,
            })
            .max()
            .unwrap();

        let terminals: Vec<&str> = errors
            .iter()
            .map(|e| match e {
                ParseError::UnknownNonTerminal(_) => unreachable!(),
                ParseError::BadMatchError { terminals, idx, .. } => (terminals, idx),
            })
            .filter_map(|(terms, idx)| if *idx == max { Some(terms) } else { None })
            .flatten()
            .map(|s| *s)
            .collect();

        let msg = match terminals.len() {
            0 => return Fallible::Ok(None),
            1 => format!("Expected `{}` here.", terminals[0]),
            _ => {
                let mut msg = String::from("Expected one of ");
                for t in &terminals[0..terminals.len() - 1] {
                    msg = format!("{msg}`{t}`, ")
                }
                format!("{msg} or `{}`.", terminals.last().unwrap())
            }
        };

        match errors.iter().find(|e| match e {
            ParseError::UnknownNonTerminal(_) => unreachable!(),
            ParseError::BadMatchError { idx, .. } => *idx == max,
        }) {
            Some(ParseError::BadMatchError {
                line,
                col,
                idx,
                rules,
                ..
            }) => Fallible::Ok(Some(ParseError::BadMatchError {
                line,
                col: *col,
                idx: *idx,
                msg,
                terminals: BTreeSet::from_iter(terminals),
                rules: rules.clone(),
            })),
            _ => unreachable!(),
        }
    }
}

impl<Source> std::fmt::Debug for ParseError<Source>
where
    Source: std::fmt::Display + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownNonTerminal(non_term) => {
                write!(f, "Grammar Error - Unknown rule: `{non_term}`")
            }
            Self::BadMatchError {
                line,
                col,
                msg,
                rules,
                ..
            } => {
                writeln!(f, "{}", msg)?;
                writeln!(f, "{line}")?;
                for _ in 0..*col {
                    write!(f, " ")?;
                }
                write!(f, "^")?;

                writeln!(f)?;
                writeln!(f, "rules: {rules:?}")?;
                Ok(())
            }
        }
    }
}

impl<'a> From<ParseError<&'a str>> for ParseError<String> {
    fn from(value: ParseError<&'a str>) -> Self {
        match value {
            ParseError::BadMatchError {
                line,
                col,
                idx,
                msg,
                terminals,
                rules,
            } => ParseError::BadMatchError {
                line: String::from(line),
                col: col,
                idx: idx,
                msg: msg,
                terminals: terminals
                    .into_iter()
                    .map(|term| String::from(term))
                    .collect(),
                rules: rules.into_iter().map(|rule| String::from(rule)).collect(),
            },
            ParseError::UnknownNonTerminal(e) => ParseError::UnknownNonTerminal(String::from(e)),
        }
    }
}
