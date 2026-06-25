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
    fs::write(codex_bundle.join("skills/dev-flow/SKILL.md"), "# dev-flow\n").unwrap();
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
    fs::write(&claude_agents, format!("{}\n## Dev Flow\n", DEV_FLOW_MARKER)).unwrap();

    let old_path = env::var("PATH").unwrap_or_default();
    let output = Command::new(env!("CARGO_BIN_EXE_dow"))
        .args(["setup", "--agent", "codex"])
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &data)
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
    assert!(codex_content.contains("Prefer Codex file edit/write tools"));
    assert_eq!(codex_content.matches(DEV_FLOW_MARKER).count(), 1);

    let claude_content = fs::read_to_string(&claude_agents).unwrap();
    assert!(!claude_content.contains(CODEX_HOOK_DISCIPLINE_MARKER));
}
