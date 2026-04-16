use assert_cmd::Command;

#[test]
fn exits_nonzero_with_no_mirrors() {
    Command::cargo_bin("select-mirror")
        .unwrap()
        .assert()
        .failure();
}
