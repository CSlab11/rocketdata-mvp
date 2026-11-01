use std::process::Command;

fn rocket_tag() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rocket-tag"))
}

/// Il crate si compila e si esegue da solo, senza gli altri tool (REQ-079).
#[test]
fn si_esegue_da_solo() {
    let output = rocket_tag().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("rocket-tag "));
}
