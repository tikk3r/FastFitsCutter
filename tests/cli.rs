use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn cutout() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ffc")?;
    cmd.arg("--ra").arg("218.0");
    cmd.arg("--dec").arg("34.5");
    cmd.arg("--size").arg("0.0416666666");
    cmd.arg("tests/testimg.fits");
    cmd.assert().success();
    Ok(())
}
