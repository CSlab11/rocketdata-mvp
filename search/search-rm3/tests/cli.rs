use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_search-rm3")).args(args).output().unwrap()
}

#[test]
fn offre_i_comandi_index_e_query() {
    let out = run(&["--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("index"), "{stdout}");
    assert!(stdout.contains("query"), "{stdout}");
}
