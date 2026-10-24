use std::fs;

/// Nessun crate di rete tra le dipendenze e nessun uso di `std::net` nel codice (REQ-062).
#[test]
fn nessuna_dipendenza_di_rete() {
    let lock = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")).unwrap();
    let network = ["reqwest", "hyper", "ureq", "curl", "tokio", "native-tls", "rustls", "attohttpc", "isahc"];
    for line in lock.lines().filter_map(|l| l.strip_prefix("name = ")) {
        let name = line.trim_matches('"');
        assert!(!network.contains(&name), "dipendenza di rete: {name}");
    }
    for entry in fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src")).unwrap() {
        let path = entry.unwrap().path();
        assert!(!fs::read_to_string(&path).unwrap().contains("std::net"), "{}", path.display());
    }
}
