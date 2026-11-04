//! Normalizzazione di valori numerici e unità di misura in una rappresentazione
//! unica (unità SI). "2,500 g", "2.5 kilograms" e "2.5 kg" danno lo stesso valore.

use crate::text::{Kind, Token};

#[derive(Debug, Clone, PartialEq)]
pub struct Quantity {
    /// Valore come scritto (numero).
    pub value: f64,
    /// Unità come scritta ("" se assente).
    pub unit: String,
    /// Valore nell'unità SI.
    pub si_value: f64,
    pub si_unit: &'static str,
    /// Grandezza: mass, length, time, power, ... oppure "count".
    pub dim: &'static str,
}

/// (simbolo o nome, grandezza, fattore verso SI, unità SI). I simboli sono
/// confrontati rispettando le maiuscole, i nomi in minuscolo.
const UNITS: &[(&str, &str, f64, &str)] = &[
    ("kg", "mass", 1.0, "kg"), ("g", "mass", 1e-3, "kg"), ("mg", "mass", 1e-6, "kg"),
    ("t", "mass", 1e3, "kg"), ("lb", "mass", 0.453_592_37, "kg"), ("lbs", "mass", 0.453_592_37, "kg"),
    ("m", "length", 1.0, "m"), ("km", "length", 1e3, "m"), ("cm", "length", 1e-2, "m"),
    ("mm", "length", 1e-3, "m"), ("µm", "length", 1e-6, "m"), ("um", "length", 1e-6, "m"),
    ("s", "time", 1.0, "s"), ("ms", "time", 1e-3, "s"), ("min", "time", 60.0, "s"),
    ("h", "time", 3600.0, "s"), ("hr", "time", 3600.0, "s"), ("yr", "time", 31_557_600.0, "s"),
    ("W", "power", 1.0, "W"), ("kW", "power", 1e3, "W"), ("mW", "power", 1e-3, "W"),
    ("MW", "power", 1e6, "W"),
    ("V", "voltage", 1.0, "V"), ("mV", "voltage", 1e-3, "V"), ("kV", "voltage", 1e3, "V"),
    ("A", "current", 1.0, "A"), ("mA", "current", 1e-3, "A"),
    ("Hz", "frequency", 1.0, "Hz"), ("kHz", "frequency", 1e3, "Hz"),
    ("MHz", "frequency", 1e6, "Hz"), ("GHz", "frequency", 1e9, "Hz"),
    ("bps", "datarate", 1.0, "bit/s"), ("kbps", "datarate", 1e3, "bit/s"),
    ("Mbps", "datarate", 1e6, "bit/s"), ("Gbps", "datarate", 1e9, "bit/s"),
    ("kbit/s", "datarate", 1e3, "bit/s"), ("Mbit/s", "datarate", 1e6, "bit/s"),
    ("J", "energy", 1.0, "J"), ("kJ", "energy", 1e3, "J"), ("Wh", "energy", 3600.0, "J"),
    ("kWh", "energy", 3.6e6, "J"),
    ("K", "temperature", 1.0, "K"), ("°C", "temperature", 1.0, "K"),
    ("N", "force", 1.0, "N"), ("kN", "force", 1e3, "N"), ("Nm", "torque", 1.0, "N·m"),
    ("m/s", "speed", 1.0, "m/s"), ("km/s", "speed", 1e3, "m/s"), ("km/h", "speed", 1.0 / 3.6, "m/s"),
    ("%", "ratio", 1.0, "%"), ("°", "angle", 1.0, "deg"), ("deg", "angle", 1.0, "deg"),
];

const UNIT_NAMES: &[(&str, &str)] = &[
    ("kilogram", "kg"), ("kilograms", "kg"), ("gram", "g"), ("grams", "g"),
    ("milligram", "mg"), ("milligrams", "mg"), ("tonne", "t"), ("tonnes", "t"),
    ("pound", "lb"), ("pounds", "lb"),
    ("metre", "m"), ("metres", "m"), ("meter", "m"), ("meters", "m"),
    ("kilometre", "km"), ("kilometres", "km"), ("kilometer", "km"), ("kilometers", "km"),
    ("centimetre", "cm"), ("centimetres", "cm"), ("centimeter", "cm"), ("centimeters", "cm"),
    ("millimetre", "mm"), ("millimetres", "mm"), ("millimeter", "mm"), ("millimeters", "mm"),
    ("second", "s"), ("seconds", "s"), ("sec", "s"), ("millisecond", "ms"),
    ("milliseconds", "ms"), ("minute", "min"), ("minutes", "min"), ("hour", "h"),
    ("hours", "h"), ("hrs", "h"), ("day", "day"), ("days", "day"), ("week", "week"),
    ("weeks", "week"), ("month", "month"), ("months", "month"), ("year", "yr"),
    ("years", "yr"), ("yrs", "yr"),
    ("watt", "W"), ("watts", "W"), ("kilowatt", "kW"), ("kilowatts", "kW"),
    ("milliwatt", "mW"), ("milliwatts", "mW"), ("volt", "V"), ("volts", "V"),
    ("amp", "A"), ("amps", "A"), ("ampere", "A"), ("amperes", "A"), ("hertz", "Hz"),
    ("joule", "J"), ("joules", "J"), ("kelvin", "K"), ("newton", "N"), ("newtons", "N"),
    ("percent", "%"), ("degree", "deg"), ("degrees", "deg"),
];

const EXTRA_TIME: &[(&str, f64)] = &[("day", 86_400.0), ("week", 604_800.0), ("month", 2_629_800.0)];

const NUMBER_WORDS: &[(&str, f64)] = &[
    ("one", 1.0), ("two", 2.0), ("three", 3.0), ("four", 4.0), ("five", 5.0), ("six", 6.0),
    ("seven", 7.0), ("eight", 8.0), ("nine", 9.0), ("ten", 10.0), ("eleven", 11.0),
    ("twelve", 12.0), ("dozen", 12.0), ("single", 1.0), ("dual", 2.0), ("twin", 2.0),
];

fn lookup_unit(text: &str) -> Option<(&'static str, f64, &'static str, &'static str)> {
    let canon = UNIT_NAMES
        .iter()
        .find(|(n, _)| *n == text.to_lowercase())
        .map(|(_, s)| *s)
        .unwrap_or(text);
    if let Some((_, secs)) = EXTRA_TIME.iter().find(|(n, _)| *n == canon) {
        return Some(("time", *secs, "s", canon_static(canon)));
    }
    UNITS
        .iter()
        .find(|(sym, ..)| *sym == canon)
        .map(|(sym, dim, f, si)| (*dim, *f, *si, *sym))
}

fn canon_static(c: &str) -> &'static str {
    EXTRA_TIME.iter().find(|(n, _)| *n == c).map(|(n, _)| *n).unwrap_or("")
}

pub fn number_word(lower: &str) -> Option<f64> {
    NUMBER_WORDS.iter().find(|(w, _)| *w == lower).map(|(_, v)| *v)
}

/// Anno senza unità (1000–2100): è una data, non una quantità.
pub fn is_year(t: &Token) -> bool {
    t.kind == Kind::Number
        && t.text.len() == 4
        && t.text.parse::<u32>().map_or(false, |y| (1000..=2100).contains(&y))
}

/// Prima quantità del sintagma, normalizzata.
pub fn parse(toks: &[Token]) -> Option<Quantity> {
    let i = toks.iter().position(|t| t.kind == Kind::Number && !is_year(t));
    let (value, unit_tok) = match i {
        Some(i) => {
            let v: f64 = toks[i].text.replace(',', "").replace('−', "-").parse().ok()?;
            (v, toks.get(i + 1))
        }
        None => {
            let i = toks.iter().position(|t| number_word(&t.lower).is_some())?;
            (number_word(&toks[i].lower)?, None)
        }
    };
    let unit = unit_tok.and_then(|u| lookup_unit(&u.text).map(|x| (u.text.clone(), x)));
    Some(match unit {
        Some((written, (dim, factor, si_unit, sym))) => {
            let si_value = if sym == "°C" { value + 273.15 } else { value * factor };
            Quantity { value, unit: written, si_value, si_unit, dim }
        }
        None => Quantity { value, unit: String::new(), si_value: value, si_unit: "", dim: "count" },
    })
}

/// Numero senza zeri superflui.
pub fn fmt_num(v: f64) -> String {
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

impl Quantity {
    /// Forma normalizzata, es. "2.5 kg".
    pub fn normalized(&self) -> String {
        if self.si_unit.is_empty() {
            fmt_num(self.si_value)
        } else {
            format!("{} {}", fmt_num(self.si_value), self.si_unit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::tokenize;

    fn q(s: &str) -> Quantity {
        parse(&tokenize(s)).unwrap()
    }

    #[test]
    fn different_forms_same_value() {
        for s in ["2,500 g", "2.5 kilograms", "2.5 kg", "2500 grams"] {
            let x = q(s);
            assert_eq!((x.dim, x.normalized()), ("mass", "2.5 kg".to_string()), "{s}");
        }
        assert_eq!(q("600 km").normalized(), "600000 m");
        assert_eq!(q("-20 °C").normalized(), "253.15 K");
        assert_eq!(q("two solar arrays").dim, "count");
    }

    #[test]
    fn years_are_not_quantities() {
        assert!(parse(&tokenize("March 2021")).is_none());
    }
}
