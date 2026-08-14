// tests/
// ├── test_dow_setup.rs  -- dow setup 注册行为集成测试

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const DEV_FLOW_MARKER: &str = "<!-- dev-flow -->";
const CODEX_HOOK_DISCIPLINE_MARKER: &str = "<!-- dev-flow-codex-hooks -->";
const CODEX_DISCIPLINE_PLACEHOLDER: &str = "{CODEX DEV FLOW Discipline}";

fn write_fake_codex(bin_dir: &Path) {
    fs::create_dir_all(bin_dir).unwrap();
    let codex_path = bin_dir.join("codex");
    fs::write(&codex_path, "#!/usr/bin/env sh\nexit 0\n").unwrap();

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&codex_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&codex_path, permissions).unwrap();
    }
}

fn write_fake_pi(bin_dir: &Path) {
    fs::create_dir_all(bin_dir).unwrap();
    let pi_path = bin_dir.join("pi");
    fs::write(&pi_path, "#!/usr/bin/env sh\nexit 0\n").unwrap();

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&pi_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&pi_path, permissions).unwrap();
    }
}

fn write_minimal_codex_bundle(data_dir: &Path) -> PathBuf {
    let codex_bundle = data_dir.join("dow").join("bundle").join("codex");
    fs::create_dir_all(codex_bundle.join(".agents/plugins")).unwrap();
    fs::create_dir_all(codex_bundle.join(".codex-plugin")).unwrap();
    fs::create_dir_all(codex_bundle.join("skills/dev-flow")).unwrap();
    fs::create_dir_all(codex_bundle.join("commands")).unwrap();
    fs::create_dir_all(codex_bundle.join("agents")).unwrap();

    fs::write(
        codex_bundle.join(".agents/plugins/marketplace.json"),
        "{\"plugins\":[]}\n",
    )
    .unwrap();
    fs::write(
        codex_bundle.join("skills/dev-flow/SKILL.md"),
        "# dev-flow\n",
    )
    .unwrap();
    fs::write(codex_bundle.join(".app.json"), "{}\n").unwrap();
    fs::write(codex_bundle.join(".codex-plugin/plugin.json"), "{}\n").unwrap();

    codex_bundle
}

#[test]
fn test_setup_codex_injects_hook_discipline_without_touching_claude() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let data = temp.path().join("data");
    let bin = temp.path().join("bin");

    write_fake_codex(&bin);
    write_minimal_codex_bundle(&data);

    let codex_agents = home.join(".codex").join("AGENTS.md");
    let claude_agents = home.join(".claude").join("CLAUDE.md");
    fs::create_dir_all(codex_agents.parent().unwrap()).unwrap();
    fs::create_dir_all(claude_agents.parent().unwrap()).unwrap();
    fs::write(&codex_agents, format!("{}\n## Dev Flow\n", DEV_FLOW_MARKER)).unwrap();
    fs::write(
        &claude_agents,
        format!("{}\n## Dev Flow\n", DEV_FLOW_MARKER),
    )
    .unwrap();

    let old_path = env::var("PATH").unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["setup", "--agent", "codex"])
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &data)
        .env("CODEX_BIN", bin.join("codex"))
        .env("PATH", format!("{}:{}", bin.display(), old_path))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let codex_content = fs::read_to_string(&codex_agents).unwrap();
    assert!(codex_content.contains(CODEX_HOOK_DISCIPLINE_MARKER));
    let codex_hook_start = codex_content
        .find(CODEX_HOOK_DISCIPLINE_MARKER)
        .unwrap()
        + CODEX_HOOK_DISCIPLINE_MARKER.len();
    let codex_hook_end = codex_content[codex_hook_start..]
        .find("</dev-flow>")
        .map(|offset| codex_hook_start + offset)
        .expect("Codex injection should remain inside the dev-flow block");
    assert!(
        !codex_content[codex_hook_start..codex_hook_end]
            .trim()
            .is_empty(),
        "Codex-specific injection should not be empty"
    );
    assert!(!codex_content.contains(CODEX_DISCIPLINE_PLACEHOLDER));
    assert_eq!(codex_content.matches(DEV_FLOW_MARKER).count(), 1);

    let claude_content = fs::read_to_string(&claude_agents).unwrap();
    assert!(!claude_content.contains(CODEX_HOOK_DISCIPLINE_MARKER));
    assert!(!claude_content.contains(CODEX_DISCIPLINE_PLACEHOLDER));

    let plugin_dir = home.join(".codex").join("plugins").join("dev-flow");
    assert!(plugin_dir.is_dir());
    assert!(!home
        .join(".codex")
        .join("plugins")
        .join("plugins")
        .join("dev-flow")
        .exists());
}

#[test]
fn test_setup_codex_returns_failure_when_registration_command_fails() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let data = temp.path().join("data");
    let bin = temp.path().join("bin");

    write_fake_codex(&bin);
    let codex_path = bin.join("codex");
    fs::write(
        &codex_path,
        "#!/usr/bin/env sh\necho registration failed >&2\nexit 7\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&codex_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&codex_path, permissions).unwrap();
    }
    write_minimal_codex_bundle(&data);

    let old_path = env::var("PATH").unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["setup", "--agent", "codex"])
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &data)
        .env("CODEX_BIN", &codex_path)
        .env("PATH", format!("{}:{}", bin.display(), old_path))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Codex marketplace registration command failed"));
    assert!(!stderr.contains("[dow] Done!"));
}

#[test]
fn test_setup_codex_uses_explicit_runtime_when_not_on_path() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let data = temp.path().join("data");
    let app_runtime_dir = temp
        .path()
        .join("ChatGPT.app")
        .join("Contents")
        .join("Resources");

    write_fake_codex(&app_runtime_dir);
    write_minimal_codex_bundle(&data);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["setup", "--agent", "codex"])
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &data)
        .env("CODEX_BIN", app_runtime_dir.join("codex"))
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Using Codex runtime"));
}

#[test]
fn test_setup_preflights_all_agent_bundles_before_registration() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let data = temp.path().join("data");
    let bin = temp.path().join("bin");

    write_fake_codex(&bin);
    write_fake_pi(&bin);
    write_minimal_codex_bundle(&data);

    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["setup", "--agent", "all"])
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &data)
        .env("CODEX_BIN", bin.join("codex"))
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Plugin bundle is incomplete"));
    assert!(stderr.contains("pi"));
    assert!(!home.join(".codex/plugins/dev-flow").exists());
}
