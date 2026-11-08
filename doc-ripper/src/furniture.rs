//! Intestazioni e piè di pagina riconosciuti dalla ripetizione (REQ-074): un testo nella fascia alta o bassa
//! della pagina, con lo stesso contenuto a meno delle cifre e alla stessa altezza su almeno metà delle pagine.

/// Frazione dell'altezza della pagina che forma la fascia alta e quella bassa.
const BAND: f32 = 0.15;
/// Differenza massima di altezza, in punti, tra due ripetizioni dello stesso testo.
const TOLERANCE: f32 = 5.0;
/// Pagine minime perché un testo possa dirsi ripetuto.
const MIN_PAGES: usize = 3;

/// Testo con la sua posizione: lati alto e basso misurati in punti dal bordo superiore della pagina.
#[derive(Debug, Clone)]
pub struct Placed {
    pub page: u32,
    pub text: String,
    pub top: f32,
    pub bottom: f32,
    pub page_height: f32,
}

/// Per ogni testo, se è intestazione o piè di pagina in un documento di `pages` pagine.
pub fn detect(items: &[Placed], pages: usize) -> Vec<bool> {
    let needed = MIN_PAGES.max(pages.div_ceil(2));
    if pages < MIN_PAGES {
        return vec![false; items.len()];
    }
    let keys: Vec<Option<String>> = items.iter().map(|item| in_band(item).then(|| key(&item.text))).collect();
    (0..items.len())
        .map(|i| {
            let Some(own) = &keys[i] else { return false };
            let mut seen: Vec<u32> = items
                .iter()
                .zip(&keys)
                .filter(|(other, k)| k.as_ref() == Some(own) && (other.top - items[i].top).abs() <= TOLERANCE)
                .map(|(other, _)| other.page)
                .collect();
            seen.sort_unstable();
            seen.dedup();
            seen.len() >= needed
        })
        .collect()
}

/// Estensione verticale occupata dalle intestazioni e da piè di pagina riconosciuti (REQ-075), come distanze
/// (minima, massima) dal bordo superiore per la fascia alta e dal bordo inferiore per quella bassa.
#[derive(Debug, Clone, Copy, Default)]
pub struct Zones {
    top: Option<(f32, f32)>,
    bottom: Option<(f32, f32)>,
}

impl Zones {
    /// Zone dei testi di `items` per cui `found` è vero.
    pub fn new(items: &[Placed], found: &[bool]) -> Zones {
        let mut zones = Zones::default();
        for item in items.iter().zip(found).filter(|(_, f)| **f).map(|(item, _)| item) {
            let (zone, near, far) = if item.bottom <= BAND * item.page_height {
                (&mut zones.top, item.top, item.bottom)
            } else {
                (&mut zones.bottom, item.page_height - item.bottom, item.page_height - item.top)
            };
            *zone = Some(zone.map_or((near, far), |(lo, hi)| (lo.min(near), hi.max(far))));
        }
        zones
    }

    /// Un elemento con i lati `top` e `bottom` (dal bordo superiore) sta tutto in una delle zone.
    pub fn contains(&self, top: f32, bottom: f32, page_height: f32) -> bool {
        let within = |zone: Option<(f32, f32)>, near: f32, far: f32| {
            zone.is_some_and(|(lo, hi)| near >= lo - TOLERANCE && far <= hi + TOLERANCE)
        };
        within(self.top, top, bottom) || within(self.bottom, page_height - bottom, page_height - top)
    }
}

/// Il testo sta tutto nella fascia alta o tutto in quella bassa.
fn in_band(item: &Placed) -> bool {
    let band = BAND * item.page_height;
    item.page_height > 0.0 && (item.bottom <= band || item.top >= item.page_height - band)
}

/// Contenuto confrontabile: ogni sequenza di cifre diventa `#` e gli spazi si compattano.
fn key(text: &str) -> String {
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        let mut digits = false;
        for c in word.chars() {
            if c.is_ascii_digit() {
                if !digits {
                    out.push('#');
                }
                digits = true;
            } else {
                out.push(c);
                digits = false;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(page: u32, text: &str, top: f32) -> Placed {
        Placed { page, text: text.to_string(), top, bottom: top + 10.0, page_height: 842.0 }
    }

    #[test]
    fn testo_ripetuto_in_alto_a_meno_delle_cifre() {
        let items: Vec<Placed> = (1..=4)
            .flat_map(|p| {
                [
                    placed(p, &format!("Page {p} of 53"), 40.0 + p as f32),
                    placed(p, "Doc ID 1385823", 60.0),
                    placed(p, "Doc ID 1385823", 400.0),
                    placed(p, "Confidential", 800.0),
                ]
            })
            .chain([placed(1, "Solo qui", 40.0)])
            .collect();
        let found = detect(&items, 4);
        for (item, &f) in items.iter().zip(&found) {
            let expected = item.top < 400.0 && item.text != "Solo qui" || item.top == 800.0;
            assert_eq!(f, expected, "{item:?}");
        }
    }

    #[test]
    fn altezza_diversa_non_e_ripetizione() {
        let items: Vec<Placed> = (1..=4).map(|p| placed(p, "Titolo", 20.0 + 10.0 * p as f32)).collect();
        assert_eq!(detect(&items, 4), vec![false; 4]);
    }

    #[test]
    fn meno_di_meta_delle_pagine_o_documento_corto() {
        let items: Vec<Placed> = (1..=3).map(|p| placed(p, "Testata", 30.0)).collect();
        assert_eq!(detect(&items, 7), vec![false; 3], "3 pagine su 7");
        assert_eq!(detect(&items, 6), vec![true; 3], "3 pagine su 6");
        let short: Vec<Placed> = (1..=2).map(|p| placed(p, "Testata", 30.0)).collect();
        assert_eq!(detect(&short, 2), vec![false; 2]);
    }

    #[test]
    fn zone_occupate_dai_testi_riconosciuti() {
        let items = [placed(1, "Testata", 40.0), placed(1, "Seconda riga", 90.0), placed(1, "Piede", 800.0)];
        let zones = Zones::new(&items, &[true, true, true]);
        assert!(zones.contains(38.0, 100.0, 842.0), "intestazione su due righe, entro la tolleranza");
        assert!(zones.contains(805.0, 808.0, 842.0), "dentro il piè di pagina");
        assert!(!zones.contains(38.0, 130.0, 842.0), "sconfina nel corpo");
        assert!(!zones.contains(400.0, 410.0, 842.0));
        assert!(!Zones::new(&items, &[false; 3]).contains(40.0, 50.0, 842.0), "nessun testo riconosciuto");
    }

    #[test]
    fn contenuto_normalizzato() {
        assert_eq!(key("Page  13 of 53"), key("Page 4 of 120"));
        assert_eq!(key("Rev. 5"), "Rev. #");
        assert_ne!(key("Doc A"), key("Doc B"));
    }
}
