/// Una sezione Markdown: dall'intestazione fino alla successiva, esclusa (REQ-055).
#[derive(Debug, PartialEq)]
pub struct Section {
    /// Testo dell'intestazione, senza `#`; vuoto per il testo prima della prima intestazione.
    pub heading: String,
    /// Riga di inizio, contata da 1.
    pub line: usize,
    /// Testo completo della sezione, intestazione compresa.
    pub text: String,
}

/// Divide un file Markdown in sezioni, scartando quelle senza testo.
pub fn split(content: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current = Section { heading: String::new(), line: 1, text: String::new() };
    let mut fence: Option<&str> = None;
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(marker) = fence {
            if trimmed.starts_with(marker) {
                fence = None;
            }
        } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(&trimmed[..3]);
        } else if let Some(heading) = heading(line) {
            push(&mut sections, current);
            current = Section { heading, line: i + 1, text: String::new() };
        }
        current.text.push_str(line);
        current.text.push('\n');
    }
    push(&mut sections, current);
    sections
}

fn push(sections: &mut Vec<Section>, section: Section) {
    if !section.text.trim().is_empty() {
        sections.push(section);
    }
}

/// Testo dell'intestazione ATX (da `#` a `######`), se la riga lo è.
fn heading(line: &str) -> Option<String> {
    let line = line.trim_start_matches(' ');
    let level = line.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &line[level..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some(rest.trim().trim_end_matches('#').trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divide_per_intestazione_di_qualsiasi_livello() {
        let sections = split("# Uno\na\n### Due\nb\n###### Tre\nc\n");
        let headings: Vec<_> = sections.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(headings, ["Uno", "Due", "Tre"]);
        assert_eq!(sections[1].text, "### Due\nb\n");
        assert_eq!(sections.iter().map(|s| s.line).collect::<Vec<_>>(), [1, 3, 5]);
    }

    #[test]
    fn testo_prima_della_prima_intestazione() {
        let sections = split("Origine: x\n\n# Titolo\ntesto\n");
        assert_eq!(sections[0], Section { heading: String::new(), line: 1, text: "Origine: x\n\n".into() });
        assert_eq!(sections[1].heading, "Titolo");
        assert_eq!(sections[1].line, 3);
    }

    #[test]
    fn ignora_le_intestazioni_nei_blocchi_di_codice() {
        let sections = split("# A\n```\n# non titolo\n```\n~~~\n## nemmeno\n~~~\n# B\n");
        assert_eq!(sections.len(), 2);
        assert!(sections[0].text.contains("# non titolo"));
    }

    #[test]
    fn non_sono_intestazioni() {
        assert_eq!(heading("#hashtag"), None);
        assert_eq!(heading("####### sette"), None);
        assert_eq!(heading("## Titolo ##"), Some("Titolo".into()));
        assert_eq!(heading("#"), Some(String::new()));
    }

    #[test]
    fn scarta_le_sezioni_vuote() {
        assert!(split("").is_empty());
        assert_eq!(split("\n\n# A\n").len(), 1);
    }
}
