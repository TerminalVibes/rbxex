use std::process::Command;

#[test]
fn bare_command_prints_help_successfully() {
    let output = Command::new(env!("CARGO_BIN_EXE_rbxex"))
        .output()
        .expect("failed to run rbxex");

    assert!(
        output.status.success(),
        "expected bare rbxex to exit successfully, got {:?}",
        output.status.code()
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout was not UTF-8");
    assert!(stdout.contains("Usage: rbxex [OPTIONS] [COMMAND]"));
}

#[test]
fn unknown_subcommand_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_rbxex"))
        .arg("notacommand")
        .output()
        .expect("failed to run rbxex");

    assert!(
        !output.status.success(),
        "expected unknown subcommand to exit nonzero, got {:?}",
        output.status.code()
    );
}
