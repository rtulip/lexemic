use std::{collections::BTreeSet};

pub enum Fallible<T, E> {
    Ok(T),
    Recovered(T, E),
    Err(E)
}

impl<T, E> Fallible<T, E> {
    pub fn into_result(self) -> Result<T, E> {
        match self {
            Fallible::Ok(t) | Fallible::Recovered(t, _) => Ok(t),
            Fallible::Err(e) => Err(e)
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

impl<T, E> Into<Result<T, E>> for Fallible<T, E> {
    fn into(self) -> Result<T, E> {
        match self {
            Fallible::Ok(t) | Fallible::Recovered(t, _) => Ok(t),
            Fallible::Err(e) => Err(e),
        }
    }
}

#[derive(Clone)]
pub enum ParseError<'a> {
    UnknownNonTerminal(&'a str),
    BadMatchError {
        line: &'a str,
        col: usize,
        idx: usize,
        msg: String,
        terminals: BTreeSet<&'a str>,
    }
}

impl<'a> ParseError<'a> {

    pub fn new_bad_match<S: Into<String>>(source: &'a str, idx: &usize, msg: S, terminals: Vec<&'a str>) -> Self {

        let prev_newline = source[0..*idx].rfind("\n");
        let next_newline = source[*idx..].find("\n");
        let (line, col) = match (prev_newline, next_newline) {
            (Some(prev), Some(next)) => {
                if prev+1 < *idx {
                    (&source[prev+1..*idx+ next], *idx - prev - 1)
                } else {
                    todo!()
                }
            }
            (None, Some(_)) => todo!(),
            (Some(prev), None) => {
                if prev+1 < *idx {
                    (&source[prev+1..], *idx - prev - 1)
                } else {
                    todo!()
                }
            },
            (None, None) => {
                (source, *idx)
            },
        };

        Self::BadMatchError { 
            line, 
            col, 
            idx: *idx, 
            msg: msg.into(), 
            terminals: BTreeSet::from_iter(terminals), 
        }
    }

    pub fn collect_furthest(errors: Vec<Self>) -> Fallible<Option<Self>, Self> {
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

        let max = errors.iter().map(|e| match e {
            ParseError::UnknownNonTerminal(_) => unreachable!(),
            ParseError::BadMatchError { idx, .. } => *idx,
        })
        .max()
        .unwrap();

        let terminals: Vec<&str> = errors.iter().map(|e| match e {
            ParseError::UnknownNonTerminal(_) => unreachable!(),
            ParseError::BadMatchError { terminals, idx, .. } => (terminals, idx)
        })
        .filter_map(|(terms, idx)| if *idx == max {
            Some(terms)
        } else {
            None
        })
        .flatten()
        .map(|s| *s)
        .collect();

        let msg = match terminals.len() {
            0 => return Fallible::Ok(None),
            1 => format!("Expected `{}` here.", terminals[0]),
            _ => {
                let mut msg = String::from("Expected one of ");
                for t in &terminals[0..terminals.len()-1] {
                    msg = format!("{msg}`{t}`, ")
                }
                format!("{msg} or `{}`.", terminals.last().unwrap())
            }
        };

        match errors.iter().find(|e| match e {
            ParseError::UnknownNonTerminal(_) => unreachable!(),
            ParseError::BadMatchError { idx,.. } => *idx == max,
        }) {
            Some(ParseError::BadMatchError { line, col, idx, .. }) => {
                Fallible::Ok(Some(ParseError::BadMatchError { 
                    line, 
                    col: *col, 
                    idx: *idx, 
                    msg,
                    terminals: BTreeSet::from_iter(terminals)
                }))
            }
            _ => unreachable!(),
        }
    }

}

impl<'a> std::fmt::Debug for ParseError<'a> {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownNonTerminal(non_term) => write!(f, "Grammar Error - Unknown rule: `{non_term}`"),
            Self::BadMatchError { line, col, msg, .. } => {
                writeln!(f, "{}", msg)?;
                writeln!(f, "{line}")?;
                for _ in 0..*col {
                    write!(f, " ")?;
                }
                write!(f, "^")

            }
        }
        
    }

}