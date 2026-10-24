use rust_stemmers::{Algorithm, Stemmer};

/// Stopword inglesi (elenco NLTK, con i frammenti delle forme contratte).
const STOPWORDS: &[&str] = &[
    "a", "about", "above", "after", "again", "against", "ain", "all", "am", "an", "and", "any", "are",
    "aren", "as", "at", "be", "because", "been", "before", "being", "below", "between", "both", "but",
    "by", "can", "couldn", "d", "did", "didn", "do", "does", "doesn", "doing", "don", "down", "during",
    "each", "few", "for", "from", "further", "had", "hadn", "has", "hasn", "have", "haven", "having",
    "he", "her", "here", "hers", "herself", "him", "himself", "his", "how", "i", "if", "in", "into",
    "is", "isn", "it", "its", "itself", "just", "ll", "m", "ma", "me", "mightn", "more", "most",
    "mustn", "my", "myself", "needn", "no", "nor", "not", "now", "o", "of", "off", "on", "once",
    "only", "or", "other", "our", "ours", "ourselves", "out", "over", "own", "re", "s", "same",
    "shan", "she", "should", "shouldn", "so", "some", "such", "t", "than", "that", "the", "their",
    "theirs", "them", "themselves", "then", "there", "these", "they", "this", "those", "through",
    "to", "too", "under", "until", "up", "ve", "very", "was", "wasn", "we", "were", "weren", "what",
    "when", "where", "which", "while", "who", "whom", "why", "will", "with", "won", "wouldn", "y",
    "you", "your", "yours", "yourself", "yourselves",
];

/// Analisi del testo inglese, uguale per sezioni e domanda (REQ-063).
pub struct Analyzer {
    stemmer: Stemmer,
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer { stemmer: Stemmer::create(Algorithm::English) }
    }

    /// Token del testo: minuscole, senza stopword, stemmati con Porter2 tranne gli identificatori.
    pub fn tokens(&self, text: &str) -> Vec<String> {
        unescape(&strip_link_targets(text))
            .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
            .map(|word| word.trim_matches(['_', '-']))
            .filter(|word| !word.is_empty())
            .filter_map(|word| {
                let word = word.to_lowercase();
                if is_identifier(&word) {
                    Some(word)
                } else if STOPWORDS.binary_search(&word.as_str()).is_ok() {
                    None
                } else {
                    Some(self.stemmer.stem(&word).into_owned())
                }
            })
            .collect()
    }
}

/// Token da tenere interi: con cifre o con `_` o `-` interni (es. `gpiox_crl`, `do-178c`, `apb2`).
fn is_identifier(word: &str) -> bool {
    word.contains(|c: char| c.is_numeric() || c == '_' || c == '-')
}

/// Toglie le destinazioni dei link e delle immagini Markdown (`](…)`), lasciando il testo del link.
/// Una `\]` letterale non chiude il testo di un link.
fn strip_link_targets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("](") {
        if rest[..start].ends_with('\\') {
            out.push_str(&rest[..start + 2]);
            rest = &rest[start + 2..];
            continue;
        }
        out.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        match rest.find(')') {
            Some(end) => rest = &rest[end + 1..],
            None => rest = "",
        }
        out.push(' ');
    }
    out.push_str(rest);
    out
}

/// Toglie la `\` degli escape Markdown davanti alla punteggiatura ASCII (es. `RCC\_CR` → `RCC_CR`).
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek().is_some_and(|n| n.is_ascii_punctuation()) {
            out.push(chars.next().unwrap());
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<String> {
        Analyzer::new().tokens(text)
    }

    #[test]
    fn stopword_ordinate_per_la_ricerca_binaria() {
        assert!(STOPWORDS.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn minuscole_stopword_e_porter2() {
        assert_eq!(tokens("How is the Clock ENABLED?"), ["clock", "enabl"]);
        assert_eq!(tokens("registers configuration"), ["regist", "configur"]);
    }

    #[test]
    fn identificatori_interi_e_non_stemmati() {
        assert_eq!(tokens("GPIOx_CRL DO-178C APB2 well-known"), ["gpiox_crl", "do-178c", "apb2", "well-known"]);
    }

    #[test]
    fn trattini_e_underscore_agli_estremi_non_contano() {
        assert_eq!(tokens("-clocks_ __"), ["clock"]);
    }

    #[test]
    fn toglie_gli_escape_markdown() {
        assert_eq!(tokens(r"(RCC\_APB2ENR) DO\-178C"), ["rcc_apb2enr", "do-178c"]);
        assert_eq!(tokens(r"timer\\clock"), ["timer", "clock"]);
    }

    #[test]
    fn una_parentesi_quadra_con_escape_non_apre_un_link() {
        assert_eq!(tokens(r"timer\](clock)"), ["timer", "clock"]);
    }

    #[test]
    fn ignora_le_destinazioni_dei_link() {
        assert_eq!(tokens("[manual](C:/dati/rm0008.pdf) ![](rm0008_pdf_img/img-1.png)"), ["manual"]);
    }
}
