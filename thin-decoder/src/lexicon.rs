//! Lessici chiusi dell'inglese usati dalle regole: parole funzionali,
//! forme irregolari e verbi base. Tutte le liste sono fisse e ordinate,
//! quindi le decisioni che ne derivano sono deterministiche.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use crate::text::stem;

pub const DETERMINERS: &[&str] = &[
    "a", "an", "the", "this", "that", "these", "those", "its", "their", "his", "her", "our",
    "your", "my", "each", "every", "some", "any", "all", "both", "either", "neither",
];

pub const PREPOSITIONS: &[&str] = &[
    "about", "above", "across", "after", "against", "along", "among", "around", "as", "at",
    "before", "behind", "below", "beneath", "between", "beyond", "by", "during", "for", "from",
    "in", "inside", "into", "near", "on", "onto", "outside", "over", "per", "since", "through",
    "throughout", "to", "toward", "towards", "under", "until", "upon", "via", "with", "within",
    "without", "aboard",
];

pub const BE: &[&str] = &["am", "are", "be", "been", "being", "is", "was", "were"];
pub const HAVE: &[&str] = &["had", "has", "have", "having"];
pub const DO: &[&str] = &["did", "do", "does"];
pub const MODALS: &[&str] = &[
    "can", "could", "may", "might", "must", "shall", "should", "will", "would",
];
pub const PRONOUNS: &[&str] = &["i", "we", "you", "they", "he", "she", "it"];

/// Avverbi frequenti che possono stare dentro il gruppo verbale.
pub const ADVERBS: &[&str] = &[
    "also", "already", "always", "currently", "first", "fully", "further", "initially",
    "later", "nominally", "normally", "often", "only", "originally", "periodically",
    "primarily", "still", "subsequently", "successfully", "then", "typically", "usually",
];

/// Parole che chiudono un sintagma (inizio di subordinata).
pub const CLAUSE_BREAKERS: &[&str] = &[
    "although", "because", "if", "unless", "when", "whereas", "where", "which", "while", "who",
    "whom", "whose",
];

/// Participi e passati irregolari → forma base.
pub const IRREGULAR: &[(&str, &str)] = &[
    ("analyses", "analysis"), ("antennae", "antenna"), ("are", "be"), ("been", "be"),
    ("began", "begin"), ("begun", "begin"), ("being", "be"), ("bought", "buy"),
    ("brought", "bring"), ("built", "build"), ("caught", "catch"), ("children", "child"),
    ("chose", "choose"), ("chosen", "choose"), ("criteria", "criterion"), ("cut", "cut"),
    ("did", "do"), ("does", "do"), ("done", "do"), ("drawn", "draw"), ("drew", "draw"),
    ("driven", "drive"), ("drove", "drive"), ("fed", "feed"), ("fell", "fall"),
    ("fallen", "fall"), ("feet", "foot"), ("flew", "fly"), ("flown", "fly"),
    ("found", "find"), ("gave", "give"), ("given", "give"), ("gone", "go"), ("got", "get"),
    ("gotten", "get"), ("grew", "grow"), ("grown", "grow"), ("had", "have"), ("has", "have"),
    ("held", "hold"), ("hid", "hide"), ("hidden", "hide"), ("is", "be"), ("am", "be"),
    ("kept", "keep"), ("knew", "know"), ("known", "know"), ("laid", "lay"), ("led", "lead"),
    ("left", "leave"), ("lent", "lend"), ("lost", "lose"), ("made", "make"), ("meant", "mean"),
    ("men", "man"), ("met", "meet"), ("paid", "pay"), ("put", "put"), ("ran", "run"),
    ("read", "read"), ("rose", "rise"), ("risen", "rise"), ("said", "say"), ("sat", "sit"),
    ("saw", "see"), ("seen", "see"), ("sent", "send"), ("set", "set"), ("shown", "show"),
    ("shut", "shut"), ("sold", "sell"), ("spent", "spend"), ("split", "split"),
    ("spoke", "speak"), ("spoken", "speak"), ("stood", "stand"), ("taken", "take"),
    ("taught", "teach"), ("thought", "think"), ("threw", "throw"), ("thrown", "throw"),
    ("told", "tell"), ("took", "take"), ("was", "be"), ("went", "go"), ("were", "be"),
    ("women", "woman"), ("won", "win"), ("wore", "wear"), ("worn", "wear"), ("wrote", "write"),
    ("written", "write"), ("broke", "break"), ("broken", "break"), ("froze", "freeze"),
    ("frozen", "freeze"), ("bound", "bind"), ("wound", "wind"), ("struck", "strike"),
    ("hung", "hang"), ("spun", "spin"), ("dealt", "deal"), ("fit", "fit"),
];

/// Forme irregolari di passato/participio (sottoinsieme di IRREGULAR che sono verbi).
pub const IRREGULAR_PARTICIPLES: &[&str] = &[
    "began", "begun", "bound", "bought", "broke", "broken", "brought", "built", "caught",
    "chose", "chosen", "cut", "dealt", "done", "drawn", "drew", "driven", "drove", "fed",
    "fallen", "fell", "flew", "flown", "found", "froze", "frozen", "gave", "given", "gone",
    "got", "gotten", "grew", "grown", "held", "hid", "hidden", "hung", "kept", "knew", "known",
    "laid", "led", "lent", "lost", "made", "meant", "met", "paid", "put", "ran", "risen", "rose",
    "said", "sat", "saw", "seen", "sent", "set", "shown", "shut", "sold", "spent", "split",
    "spoke", "spoken", "spun", "stood", "struck", "taken", "taught", "thought", "threw",
    "thrown", "told", "took", "went", "won", "wore", "worn", "wound", "written", "wrote",
];

/// Parole in -ed che non sono participi.
pub const NOT_PARTICIPLES: &[&str] = &[
    "aged", "bed", "beloved", "embed", "hundred", "kindred", "naked", "need", "red", "sacred",
    "shed", "speed", "seed", "feed", "wicked", "wretched",
];

/// Verbi base frequenti nei testi tecnici. Confrontati per radice.
pub const VERBS: &[&str] = &[
    "accept", "access", "achieve", "acquire", "activate", "add", "adjust", "allocate", "allow",
    "analyse", "analyze", "apply", "approve", "archive", "arrive", "assemble", "assess",
    "assign", "attach", "attain", "authorise", "authorize", "avoid", "become", "begin",
    "belong", "boost", "break", "bring", "build", "buy", "calculate", "calibrate", "call",
    "capture", "carry", "cause", "certify", "change", "charge", "check", "choose", "collect",
    "combine", "command", "communicate", "compare", "complete", "comply", "compose",
    "comprise", "compute", "conduct", "configure", "confirm", "connect", "consist", "consume",
    "contain", "continue", "contribute", "control", "convert", "cool", "coordinate", "cost",
    "cover", "create", "cut", "damage", "deactivate", "decode", "decrease", "define",
    "deliver", "demonstrate", "depend", "deploy", "derive", "describe", "design", "detect",
    "determine", "develop", "dissipate", "distribute", "do", "downlink", "draw", "drive",
    "employ", "enable", "encode", "encrypt", "endure", "ensure", "enter", "equip", "establish",
    "estimate", "evaluate", "exceed", "execute", "exhibit", "expect", "experience", "explain",
    "export", "expose", "extend", "fail", "feature", "feed", "fill", "find", "fire", "fit",
    "fly", "follow", "form", "fund", "generate", "get", "give", "go", "guarantee", "guide",
    "handle", "happen", "heat", "help", "hold", "host", "house", "identify", "implement",
    "import", "improve", "include", "increase", "indicate", "inform", "inherit", "initiate",
    "inject", "inspect", "install", "integrate", "interface", "involve", "isolate", "issue",
    "join", "keep", "know", "land", "last", "launch", "lead", "leave", "limit", "link", "load",
    "locate", "log", "lose", "maintain", "make", "manage", "manufacture", "map", "measure",
    "meet", "migrate", "model", "monitor", "mount", "move", "name", "need", "notify",
    "observe", "obtain", "occur", "offer", "operate", "orbit", "order", "own", "pass",
    "perform", "permit", "place", "plan", "point", "position", "power", "prepare", "present",
    "prevent", "process", "procure", "produce", "program", "propel", "protect", "prove",
    "provide", "publish", "pump", "put", "qualify", "reach", "read", "receive", "record",
    "recover", "reduce", "refer", "regulate", "reject", "relay", "release", "rely", "remain",
    "remove", "replace", "report", "represent", "request", "require", "reside", "respond",
    "restart", "result", "retain", "return", "reuse", "review", "rotate", "route", "run",
    "sample", "save", "scan", "schedule", "seal", "select", "sell", "send", "sense", "separate",
    "serve", "service", "set", "share", "shield", "show", "sign", "simulate", "sit", "slew",
    "specify", "stabilise", "stabilize", "stand", "start", "stop", "store", "submit", "supply",
    "support", "survive", "switch", "take", "target", "test", "track", "train", "transfer",
    "transmit", "transport", "trigger", "turn", "undergo", "update", "upload", "uplink",
    "use", "validate", "verify", "view", "weigh", "withstand", "work", "write",
];

/// Sinonimi del verbo "have" usati nelle domande "how many N does X have".
pub const HAVE_LIKE: &[&str] = &[
    "carry", "comprise", "contain", "feature", "have", "host", "house", "include", "integrate",
    "use",
];

pub fn is_in(list: &[&str], w: &str) -> bool {
    list.contains(&w)
}

pub fn irregular_base(lower: &str) -> Option<&'static str> {
    IRREGULAR.iter().find(|(f, _)| *f == lower).map(|(_, b)| *b)
}

fn verb_stems() -> &'static BTreeSet<String> {
    static SET: OnceLock<BTreeSet<String>> = OnceLock::new();
    SET.get_or_init(|| VERBS.iter().map(|v| stem(v)).collect())
}

pub fn is_verb_stem(s: &str) -> bool {
    verb_stems().contains(s)
}

pub fn is_aux(lower: &str) -> bool {
    is_in(BE, lower) || is_in(HAVE, lower) || is_in(DO, lower) || is_in(MODALS, lower)
}

pub fn is_adverb(lower: &str) -> bool {
    is_in(ADVERBS, lower)
        || (lower.len() > 4 && lower.ends_with("ly") && !is_verb_stem(&stem(lower)))
}

/// Nomi di attributo: "the A of X" e "X has a A of V" diventano claim (X, A, V).
pub const ATTRIBUTE_NOUNS: &[&str] = &[
    "accuracy", "altitude", "area", "budget", "capacity", "colour", "color", "consumption",
    "cost", "current", "depth", "diameter", "distance", "duration", "efficiency", "energy",
    "frequency", "height", "inclination", "length", "life", "lifetime", "manufacturer",
    "mass", "material", "name", "owner", "period", "power", "pressure", "price", "radius",
    "range", "rate", "resolution", "size", "speed", "status", "temperature", "thickness",
    "time", "type", "velocity", "version", "voltage", "volume", "weight", "width",
];

pub fn is_attribute_noun(stem_word: &str) -> bool {
    static SET: OnceLock<BTreeSet<String>> = OnceLock::new();
    SET.get_or_init(|| ATTRIBUTE_NOUNS.iter().map(|w| stem(w)).collect())
        .contains(stem_word)
}
