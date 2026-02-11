#[derive(Debug, Clone)]
pub struct PvParams {
    pub meta: HashMap<String, String>, // e.g. TITLE, JCAMPDX, DATATYPE...
    pub params: HashMap<String, PvValue>, // everything under ##$...
}

#[derive(Debug, Clone)]
pub enum PvValue {
    Scalar(PvAtom),
    Array { dims: Vec<usize>, items: Vec<PvAtom> },
    Str(String), // for <...> style strings (we keep as String)
}

impl Display for PvValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PvValue::Scalar(x) => write!(f, "{x}"),
            PvValue::Array{items,..} => {
                let s = items.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",");
                write!(f, "[{s}]")
            }
            PvValue::Str(x) => write!(f, "{x}"),
        }
    }
}

impl PvValue {


    pub fn to_usize(&self) -> Option<usize> {
        match self {
            PvValue::Scalar(value) => Some(value.clone().into()),
            PvValue::Array { .. } => None,
            PvValue::Str(_) => None,
        }
    }

    pub fn to_vec_usize(&self) -> Option<Vec<usize>> {
        match self {
            PvValue::Scalar(atom) => Some(vec![atom.clone().into()]),
            PvValue::Array { items, .. } => Some(items.iter().cloned().map(|i| i.into()).collect()),
            PvValue::Str(_) => None
        }
    }

    pub fn to_vec_f64(&self) -> Option<Vec<f64>> {
        match self {
            PvValue::Scalar(atom) => Some(vec![atom.clone().into()]),
            PvValue::Array { items, .. } => Some(items.iter().cloned().map(|i| i.into()).collect()),
            PvValue::Str(_) => None
        }
    }

    pub fn to_vec_bool(&self) -> Option<Vec<bool>> {
        match self {
            PvValue::Scalar(atom) => Some(vec![atom.clone().into()]),
            PvValue::Array { items, .. } => Some(items.iter().cloned().map(|i| i.into()).collect()),
            PvValue::Str(_) => None
        }
    }

    pub fn to_vec_i64(&self) -> Option<Vec<i64>> {
        match self {
            PvValue::Scalar(atom) => Some(vec![atom.clone().into()]),
            PvValue::Array { items, .. } => Some(items.iter().cloned().map(|i| i.into()).collect()),
            PvValue::Str(_) => None
        }
    }

}


#[derive(Debug, Clone)]
pub enum PvAtom {
    Bool(bool),   // Yes/No/True/False
    Int(i64),
    Float(f64),
    Text(String), // fallback (e.g., ParallelExperiment)
}

impl Display for PvAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PvAtom::Bool(value) => write!(f, "{}", value),
            PvAtom::Int(value) => write!(f, "{}", value),
            PvAtom::Float(value) => write!(f, "{}", value),
            PvAtom::Text(value) => write!(f, "{}", value),
        }
    }
}

impl From<PvAtom> for f64 {
    fn from(val: PvAtom) -> Self {
        match val  {
            PvAtom::Bool(true) => 1.0,
            PvAtom::Bool(false) => 0.0,
            PvAtom::Int(i) => i as f64,
            PvAtom::Float(f) => f,
            PvAtom::Text(s) => s.parse().expect("cannot parse string as float"),
        }
    }
}

impl From<PvAtom> for usize {
    fn from(val: PvAtom) -> usize {
        match val {
            PvAtom::Bool(b) => {if b {1} else {0}},
            PvAtom::Int(i) => {i as usize},
            PvAtom::Float(f) => {f as usize},
            PvAtom::Text(s) => s.parse().expect("cannot parse string as usize"),
        }
    }
}

impl From<PvAtom> for i64 {
    fn from(val: PvAtom) -> i64 {
        match val {
            PvAtom::Bool(b) => {if b {1} else {0}},
            PvAtom::Int(i) => {i},
            PvAtom::Float(f) => {f as i64},
            PvAtom::Text(s) => s.parse().expect("cannot parse string as usize"),
        }
    }
}

impl From<PvAtom> for bool {
    fn from(val: PvAtom) -> bool {
        match val {
            PvAtom::Bool(b) => b,
            PvAtom::Int(i) => {i.abs() > 0},
            PvAtom::Float(f) => {f.abs() > 0.},
            PvAtom::Text(s) => s.parse().expect("cannot parse string as bool"),
        }
    }
}

use std::collections::HashMap;
use std::fmt::Display;
use std::io::{self, BufRead, BufReader};

#[derive(Debug)]
pub enum PvError {
    Io(io::Error),
    Parse(String),
}

impl From<io::Error> for PvError {
    fn from(e: io::Error) -> Self { PvError::Io(e) }
}

#[derive(Debug)]
enum Pending {
    // After seeing ##$KEY=( dims... ) we read the next non-comment line and decide:
    // - if it is <...> => Str
    // - else => Array and we treat that line as the first chunk of array data
    Dims { key: String, dims: Vec<usize> },
    Array { key: String, dims: Vec<usize>, need: usize, items: Vec<PvAtom> },
}


pub fn parse_paravision_params<R: BufRead>(mut reader: R) -> Result<PvParams, PvError> {
    let mut out = PvParams { meta: HashMap::new(), params: HashMap::new() };

    let mut pending: Option<Pending> = None;

    let mut line = String::new();
    while {
        line.clear();
        reader.read_line(&mut line)?
    } != 0 {
        let raw = line.trim_end_matches(&['\r', '\n'][..]);
        let s = raw.trim();
        if s.is_empty() || s.starts_with("$$") {
            continue;
        }

        // If we are collecting data for a previous header, handle that first.
        if let Some(p) = pending.take() {
            match p {
                Pending::Dims { key, dims } => {
                    // Decide string vs array based on this line
                    if let Some(txt) = parse_angle_brackets(s) {
                        out.params.insert(key, PvValue::Str(txt));
                    } else {
                        let need = dims.iter().product();
                        let mut items = Vec::new();
                        push_atoms_from_line(s, &mut items);

                        if items.len() >= need {
                            items.truncate(need);
                            out.params.insert(key, PvValue::Array { dims, items });
                        } else {
                            pending = Some(Pending::Array { key, dims, need, items });
                        }
                    }
                    continue;
                }
                Pending::Array { key, dims, need, mut items } => {
                    push_atoms_from_line(s, &mut items);
                    if items.len() >= need {
                        items.truncate(need);
                        out.params.insert(key, PvValue::Array { dims, items });
                    } else {
                        pending = Some(Pending::Array { key, dims, need, items });
                    }
                    continue;
                }
            }
        }


        // Not pending: parse a record line.
        if s.starts_with("##") {
            let rest = &s[2..];

            // Meta LDR: ##TITLE=..., ##JCAMPDX=...
            if !rest.starts_with('$') {
                let (k, v) = split_key_value(rest);
                out.meta.insert(normalize_key(k), v.trim().to_string());
                continue;
            }

            // Param record: ##$NAME=...
            // NAME can include underscores etc.
            let rest = &rest[1..]; // drop '$'
            let (k, v) = split_key_value(rest);
            let key = k.trim().to_string();
            let v = v.trim();

            // Array or bracket-string header?
            if let Some(dims) = parse_dims(v) {
                pending = Some(Pending::Dims { key, dims });
                continue;
            }


            // Scalar param
            out.params.insert(key, PvValue::Scalar(parse_atom(v)));
            continue;
        }

        // If we hit here: a non-## line with no pending state. Usually ignorable.
        // But: it can happen if someone wrote a continuation without declaring dims.
    }

    // If EOF while pending: try to finish gracefully
    if let Some(p) = pending {
        return Err(PvError::Parse(format!("Unexpected EOF while parsing pending record: {p:?}")));
    }

    Ok(out)
}

fn split_key_value(s: &str) -> (&str, &str) {
    match s.split_once('=') {
        Some((k, v)) => (k, v),
        None => (s, ""),
    }
}

fn normalize_key(k: &str) -> String {
    k.trim().to_ascii_uppercase()
}

fn parse_dims(v: &str) -> Option<Vec<usize>> {
    // matches "( 10, 2 )" or "( 4 )" with arbitrary spaces
    let v = v.trim();
    if !v.starts_with('(') || !v.ends_with(')') {
        return None;
    }
    let inner = &v[1..v.len()-1];
    let dims: Vec<usize> = inner
        .split(',')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<usize>())
        .collect::<Result<_, _>>()
        .ok()?;
    if dims.is_empty() { None } else { Some(dims) }
}

fn parse_angle_brackets(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with('<') || !s.ends_with('>') {
        return None;
    }
    Some(s[1..s.len()-1].to_string())
}

fn push_atoms_from_line(line: &str, out: &mut Vec<PvAtom>) {
    for tok in line.split_whitespace() {
        out.push(parse_atom(tok));
    }
}

fn parse_atom(tok: &str) -> PvAtom {
    let t = tok.trim();
    let tl = t.to_ascii_lowercase();
    match tl.as_str() {
        "yes" | "true" => return PvAtom::Bool(true),
        "no"  | "false" => return PvAtom::Bool(false),
        _ => {}
    }

    // int first (PV files often are integers)
    if let Ok(i) = t.parse::<i64>() {
        return PvAtom::Int(i);
    }
    if let Ok(f) = t.parse::<f64>() {
        return PvAtom::Float(f);
    }
    PvAtom::Text(t.to_string())
}