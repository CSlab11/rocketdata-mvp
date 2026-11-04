//! Tokenizzazione, segmentazione in frasi e radici (stem) deterministiche.
//! Gli offset sono in byte rispetto al testo di input.

use crate::lexicon::{
    irregular_base, is_in, is_verb_stem, IRREGULAR_PARTICIPLES, NOT_PARTICIPLES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Word,
    Number,
    Punct,
}

#[derive(Debug, Clone)]
pub struct Token {
    /// Forma superficiale.
    pub text: String,
    /// Minuscolo, con le contrazioni ricondotte alla forma piena.
    pub lower: String,
    /// Radice usata per i confronti.
    pub stem: String,
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

impl Token {
    fn new(text: &str, start: usize, end: usize, kind: Kind) -> Token {
        let lower = text.to_lowercase();
        Token {
            text: text.to_string(),
            stem: stem(&lower),
            lower,
            start,
            end,
            kind,
        }
    }

    pub fn is(&self, w: &str) -> bool {
        self.lower == w
    }
}

#[derive(Debug, Clone)]
pub struct Sentence {
    /// Numero della frase, a partire da 1.
    pub id: usize,
    pub start: usize,
    pub end: usize,
    pub tokens: Vec<Token>,
}

/// Forme verbali riconosciute per un token lessicale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VForm {
    Base,
    S,
    Ed,
    Ing,
}

/// Radice deterministica: stessa radice per le forme flesse della stessa parola.
pub fn stem(lower: &str) -> String {
    let w = irregular_base(lower).unwrap_or(lower);
    if w.len() < 3 || !w.chars().all(|c| c.is_ascii_alphabetic()) {
        return w.to_string();
    }
    let mut s = w.to_string();
    if s.ends_with("sses") {
        s.truncate(s.len() - 2);
    } else if s.ends_with("ies") && s.len() > 4 {
        s.truncate(s.len() - 3);
        s.push('y');
    } else if s.ends_with("ches") || s.ends_with("shes") || s.ends_with("xes") {
        s.truncate(s.len() - 2);
    } else if s.len() > 3
        && s.ends_with('s')
        && !s.ends_with("ss")
        && !s.ends_with("us")
        && !s.ends_with("is")
    {
        s.pop();
    }
    if s.ends_with("ied") && s.len() > 4 {
        s.truncate(s.len() - 3);
        s.push('y');
    } else if s.ends_with("eed") || is_in(NOT_PARTICIPLES, &s) {
    } else if s.ends_with("ed") && s.len() >= 4 && has_vowel(&s[..s.len() - 2]) {
        s.truncate(s.len() - 2);
    } else if s.ends_with("ing") && s.len() >= 5 && has_vowel(&s[..s.len() - 3]) {
        s.truncate(s.len() - 3);
    }
    if s.len() >= 3 && s.ends_with('e') {
        s.pop();
    }
    if s.len() > 3 && s.ends_with('y') {
        s.pop();
        s.push('i');
    }
    let b = s.as_bytes();
    if b.len() >= 3 {
        let (x, y) = (b[b.len() - 1], b[b.len() - 2]);
        if x == y && x != b's' && !b"aeiou".contains(&x) {
            s.pop();
        }
    }
    s
}

fn has_vowel(s: &str) -> bool {
    s.chars().any(|c| "aeiouy".contains(c))
}

/// Participio passato o passato semplice (regolare o irregolare).
pub fn is_participle(lower: &str) -> bool {
    if is_in(IRREGULAR_PARTICIPLES, lower) {
        return true;
    }
    lower.len() > 3
        && lower.ends_with("ed")
        && lower.chars().all(|c| c.is_ascii_alphabetic())
        && !is_in(NOT_PARTICIPLES, lower)
        && stem(lower) != lower
}

/// Forma verbale di un token lessicale (non ausiliare), se è un verbo.
pub fn verb_form(t: &Token) -> Option<VForm> {
    if t.kind != Kind::Word {
        return None;
    }
    if is_participle(&t.lower) {
        return Some(VForm::Ed);
    }
    if !is_verb_stem(&t.stem) {
        return None;
    }
    if t.lower.ends_with("ing") && t.lower.len() > 4 {
        Some(VForm::Ing)
    } else if t.lower.ends_with('s') && !t.lower.ends_with("ss") {
        Some(VForm::S)
    } else {
        Some(VForm::Base)
    }
}

const CLITICS: &[(&str, &str)] = &[
    ("'s", "'s"), ("'re", "are"), ("'ll", "will"), ("'ve", "have"), ("'m", "am"), ("'d", "would"),
];

const NEG_AUX: &[(&str, &str)] = &[
    ("aren", "are"), ("can", "can"), ("couldn", "could"), ("didn", "did"), ("doesn", "does"),
    ("don", "do"), ("hadn", "had"), ("hasn", "has"), ("haven", "have"), ("isn", "is"),
    ("mustn", "must"), ("shan", "shall"), ("shouldn", "should"), ("wasn", "was"),
    ("weren", "were"), ("won", "will"), ("wouldn", "would"),
];

pub fn tokenize(text: &str) -> Vec<Token> {
    let cs: Vec<(usize, char)> = text.char_indices().collect();
    let n = cs.len();
    let at = |i: usize| cs.get(i).map(|c| c.1);
    let byte = |i: usize| if i < n { cs[i].0 } else { text.len() };
    let digit_at = |i: usize| at(i).map_or(false, |c| c.is_ascii_digit());
    let alnum_at = |i: usize| at(i).map_or(false, |c| c.is_alphanumeric());
    let mut toks: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < n {
        let c = cs[i].1;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        let after_space = i == 0 || at(i - 1).map_or(true, |p| p.is_whitespace() || p == '(');
        let kind;
        if c.is_ascii_digit() || ("+-−".contains(c) && digit_at(i + 1) && after_space) {
            i += 1;
            while i < n {
                let d = cs[i].1;
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == '.' && digit_at(i + 1) {
                    i += 1;
                } else if d == ','
                    && digit_at(i + 1)
                    && digit_at(i + 2)
                    && digit_at(i + 3)
                    && !digit_at(i + 4)
                {
                    i += 1;
                } else {
                    break;
                }
            }
            kind = Kind::Number;
        } else if c.is_alphanumeric() || ("+±°µ".contains(c) && at(i + 1).map_or(false, |d| d.is_alphabetic())) {
            i += 1;
            while i < n {
                let d = cs[i].1;
                if d.is_alphanumeric() {
                    i += 1;
                } else if "-./_".contains(d) && alnum_at(i + 1) {
                    i += 1;
                } else {
                    break;
                }
            }
            kind = Kind::Word;
        } else if (c == '\'' || c == '’')
            && at(i + 1).map_or(false, |d| d.is_alphabetic())
            && toks.last().map_or(false, |t| t.end == byte(i))
        {
            i += 1;
            while i < n && cs[i].1.is_alphabetic() {
                i += 1;
            }
            kind = Kind::Word;
        } else {
            i += 1;
            kind = Kind::Punct;
        }
        let (bs, be) = (byte(start), byte(i));
        toks.push(Token::new(&text[bs..be], bs, be, kind));
    }
    normalize_contractions(toks)
}

fn normalize_contractions(toks: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(toks.len());
    for mut t in toks {
        let l = t.lower.replace('’', "'");
        if l == "'t" {
            if let Some(prev) = out.last_mut() {
                if let Some((_, full)) = NEG_AUX.iter().find(|(f, _)| *f == prev.lower) {
                    prev.lower = full.to_string();
                    prev.stem = stem(full);
                }
            }
            t.lower = "not".into();
            t.stem = "not".into();
        } else if let Some((_, full)) = CLITICS.iter().find(|(f, _)| *f == l) {
            t.lower = full.to_string();
            t.stem = stem(full);
        } else if l == "cannot" {
            let mut not = t.clone();
            t.lower = "can".into();
            t.stem = "can".into();
            t.end = t.start + 3;
            not.start = t.end;
            not.lower = "not".into();
            not.stem = "not".into();
            out.push(t);
            out.push(not);
            continue;
        }
        out.push(t);
    }
    out
}

const ABBREVIATIONS: &[&str] = &[
    "al", "approx", "dr", "e.g", "eq", "fig", "i.e", "mr", "mrs", "ms", "no", "nr", "ref", "st",
    "vol", "vs",
];

pub fn split_sentences(text: &str, tokens: Vec<Token>) -> Vec<Sentence> {
    let mut sentences = Vec::new();
    let mut cur: Vec<Token> = Vec::new();
    let n = tokens.len();
    let mut iter = tokens.into_iter().enumerate().peekable();
    while let Some((idx, t)) = iter.next() {
        let is_end_punct = t.kind == Kind::Punct && matches!(t.text.as_str(), "." | "?" | "!");
        let after_abbrev = t.text == "."
            && cur.last().map_or(false, |p| is_in(ABBREVIATIONS, &p.lower) && p.end == t.start);
        let end = t.end;
        cur.push(t);
        let paragraph_break = iter
            .peek()
            .map_or(false, |(_, next)| text[end..next.start].contains("\n\n"));
        if (is_end_punct && !after_abbrev) || paragraph_break || idx + 1 == n {
            let s = Sentence {
                id: sentences.len() + 1,
                start: cur[0].start,
                end: cur[cur.len() - 1].end,
                tokens: std::mem::take(&mut cur),
            };
            sentences.push(s);
        }
    }
    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_are_consistent() {
        for group in [
            ["operate", "operates", "operated", "operating"],
            ["use", "uses", "used", "using"],
            ["control", "controls", "controlled", "controlling"],
            ["launch", "launches", "launched", "launching"],
            ["battery", "batteries", "battery", "batteries"],
            ["install", "installs", "installed", "installing"],
        ] {
            let s: Vec<String> = group.iter().map(|w| stem(w)).collect();
            assert!(s.iter().all(|x| *x == s[0]), "{group:?} → {s:?}");
        }
    }

    #[test]
    fn tokenizes_numbers_units_and_contractions() {
        let t = tokenize("It doesn't exceed 1,200 W or -20 °C (+X panel).");
        let lows: Vec<&str> = t.iter().map(|t| t.lower.as_str()).collect();
        assert_eq!(
            lows,
            ["it", "does", "not", "exceed", "1,200", "w", "or", "-20", "°c", "(", "+x", "panel", ")", "."]
        );
    }

    #[test]
    fn splits_sentences_but_not_decimals_or_abbreviations() {
        let text = "The mass is 2.5 kg, e.g. for the RW. It is mounted on the panel.";
        let s = split_sentences(text, tokenize(text));
        assert_eq!(s.len(), 2);
        assert_eq!(&text[s[1].start..s[1].end], "It is mounted on the panel.");
    }
}
