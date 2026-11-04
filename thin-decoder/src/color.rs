//! Colore di claim e risultati: indica da dove arriva l'incertezza.
//! Solo il blu è certo; gli altri colori passano per l'LLM.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    /// Claim estratto dalle regole, o risultato delle regole su soli claim blu.
    Blu,
    /// Claim estratto dall'LLM, o risultato dell'LLM su soli claim rossi.
    Rosso,
    /// Risultato delle regole che usa almeno un claim rosso.
    Viola,
    /// Risultato dell'LLM su soli claim blu.
    Giallo,
    /// Risultato dell'LLM su claim blu e rossi.
    Arancione,
}

impl Color {
    /// Colore di un risultato del motore a regole: blu se usa solo claim blu,
    /// viola se usa almeno un claim rosso.
    pub fn of_rules(used: impl IntoIterator<Item = Color>) -> Color {
        if used.into_iter().any(|c| c == Color::Rosso) {
            Color::Viola
        } else {
            Color::Blu
        }
    }

    /// Colore di un risultato del motore LLM dai claim citati come premesse:
    /// rosso se sono tutti rossi (o se non ce ne sono), giallo se sono tutti
    /// blu, arancione se sono misti.
    pub fn of_llm(used: impl IntoIterator<Item = Color>) -> Color {
        let (mut blu, mut rosso) = (false, false);
        for c in used {
            blu |= c == Color::Blu;
            rosso |= c == Color::Rosso;
        }
        match (blu, rosso) {
            (true, true) => Color::Arancione,
            (true, false) => Color::Giallo,
            _ => Color::Rosso,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Color::Blu => "blu",
            Color::Rosso => "rosso",
            Color::Viola => "viola",
            Color::Giallo => "giallo",
            Color::Arancione => "arancione",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_engine_colors() {
        assert_eq!(Color::of_rules([]), Color::Blu);
        assert_eq!(Color::of_rules([Color::Blu, Color::Blu]), Color::Blu);
        assert_eq!(Color::of_rules([Color::Blu, Color::Rosso]), Color::Viola);
    }

    #[test]
    fn llm_engine_colors() {
        assert_eq!(Color::of_llm([Color::Rosso, Color::Rosso]), Color::Rosso);
        assert_eq!(Color::of_llm([Color::Blu]), Color::Giallo);
        assert_eq!(Color::of_llm([Color::Blu, Color::Rosso]), Color::Arancione);
        assert_eq!(Color::of_llm([]), Color::Rosso);
    }
}
