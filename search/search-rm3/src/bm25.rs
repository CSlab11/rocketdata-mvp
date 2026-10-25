use std::collections::HashMap;

use crate::index::Index;

/// Parametri BM25 (REQ-064).
const K1: f64 = 1.2;
const B: f64 = 0.75;

/// Punteggio BM25F delle sezioni per una query di termini pesati: frequenze e lunghezze
/// contano i token dell'intestazione `heading_weight` volte (REQ-067), e ogni termine
/// contribuisce con il proprio peso. Restituisce le sezioni con punteggio positivo, dalla più alta.
pub fn search(index: &Index, query: &[(String, f64)], heading_weight: f64) -> Vec<(u32, f64)> {
    let n = index.lengths.len() as f64;
    let length = |id: u32| {
        let (body, heading) = index.lengths[id as usize];
        body as f64 + heading_weight * heading as f64
    };
    let avg_length = (0..index.lengths.len() as u32).map(length).sum::<f64>() / n;
    let mut scores: HashMap<u32, f64> = HashMap::new();
    for (term, weight) in query {
        let Some(postings) = index.postings.get(term) else { continue };
        let df = postings.len() as f64;
        let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
        for &(id, tf_body, tf_heading) in postings {
            let tf = tf_body as f64 + heading_weight * tf_heading as f64;
            let norm = 1.0 - B + B * length(id) / avg_length;
            *scores.entry(id).or_default() += weight * idf * tf * (K1 + 1.0) / (tf + K1 * norm);
        }
    }
    let mut ranked: Vec<(u32, f64)> = scores.into_iter().filter(|&(_, s)| s > 0.0).collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tre sezioni: [clock clock bus], [clock], [register bus bus bus].
    fn corpus() -> Index {
        let postings = HashMap::from([
            ("clock".to_string(), vec![(0, 2, 0), (1, 1, 0)]),
            ("bus".to_string(), vec![(0, 1, 0), (2, 3, 0)]),
            ("register".to_string(), vec![(2, 1, 0)]),
        ]);
        Index { sections: Vec::new(), postings, lengths: vec![(3, 0), (1, 0), (4, 0)] }
    }

    fn term(t: &str, w: f64) -> (String, f64) {
        (t.to_string(), w)
    }

    #[test]
    fn punteggio_calcolato_con_k1_1_2_e_b_0_75() {
        let ranked = search(&corpus(), &[term("clock", 1.0)], 3.0);
        let idf = (1.0f64 + (3.0 - 2.0 + 0.5) / (2.0 + 0.5)).ln();
        let s0 = idf * 2.0 * 2.2 / (2.0 + 1.2 * (0.25 + 0.75 * 3.0 / (8.0 / 3.0)));
        let s1 = idf * 1.0 * 2.2 / (1.0 + 1.2 * (0.25 + 0.75 * 1.0 / (8.0 / 3.0)));
        // La sezione corta vince nonostante la tf minore: effetto della normalizzazione b.
        assert_eq!(ranked.iter().map(|r| r.0).collect::<Vec<_>>(), [1, 0]);
        assert!((ranked[0].1 - s1).abs() < 1e-12);
        assert!((ranked[1].1 - s0).abs() < 1e-12);
    }

    #[test]
    fn la_keyword_nell_intestazione_conta_di_piu() {
        // Stessa tf e stessa lunghezza: in 0 "clock" è nell'intestazione, in 1 nel resto.
        let postings = HashMap::from([
            ("clock".to_string(), vec![(0, 0, 1), (1, 1, 0)]),
            ("timer".to_string(), vec![(0, 1, 0), (1, 0, 1)]),
            ("bus".to_string(), vec![(2, 2, 0)]),
        ]);
        let index = Index { sections: Vec::new(), postings, lengths: vec![(1, 1), (1, 1), (2, 0)] };
        let ranked = search(&index, &[term("clock", 1.0)], 3.0);
        assert_eq!(ranked.iter().map(|r| r.0).collect::<Vec<_>>(), [0, 1]);
        assert!(ranked[0].1 > ranked[1].1);
    }

    #[test]
    fn il_peso_moltiplica_il_contributo_del_termine() {
        let one = search(&corpus(), &[term("register", 1.0)], 3.0);
        let half = search(&corpus(), &[term("register", 0.5)], 3.0);
        assert!((half[0].1 - one[0].1 / 2.0).abs() < 1e-12);
    }

    #[test]
    fn ordina_per_punteggio_decrescente_e_ignora_termini_assenti() {
        let ranked = search(&corpus(), &[term("bus", 1.0), term("assente", 1.0)], 3.0);
        assert_eq!(ranked.iter().map(|r| r.0).collect::<Vec<_>>(), [2, 0]);
        assert!(search(&corpus(), &[term("assente", 1.0)], 3.0).is_empty());
    }
}
