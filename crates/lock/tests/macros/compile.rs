use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "radicula-macro-{}-{stamp}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::create_dir(root.join(".cargo")).unwrap();
        let package = std::env::current_dir().unwrap();
        std::fs::write(root.join("Cargo.toml"), format!(
            "[package]\nname=\"macro-consumer\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[dependencies]\ncoordinate={{package=\"radicula-lock\",path={package:?},features=[\"macros\"]}}\n"
        )).unwrap();
        std::fs::write(
            root.join(".cargo/config.toml"),
            "[registries.perish]\nindex=\"sparse+https://cargo.perish.uk/\"\n",
        )
        .unwrap();
        Self(root)
    }

    fn check(&self, attribute: &str, declaration: &str) -> std::process::Output {
        let source =
            format!("use coordinate::{{Gate,guard}};\n#[guard({attribute})]\n{declaration}\n");
        std::fs::write(self.0.join("src/lib.rs"), source).unwrap();
        Command::new("cargo")
            .args(["check", "--offline", "--target-dir"])
            .arg(self.0.join("target"))
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn contract() {
    let project = Project::new();
    let declaration = "pub fn work(gate: &Gate) -> usize { 42 }";
    let positive = project.check("gate=gate,error=|_|0", declaration);
    assert!(
        positive.status.success(),
        "{}",
        String::from_utf8_lossy(&positive.stderr)
    );
    for (attribute, expected) in [
        (
            "gate=gate",
            "guard requires gate = expression and error = handler",
        ),
        (
            "error=|_|0",
            "guard requires gate = expression and error = handler",
        ),
        ("gate=gate,error=|_|0,other=1", "expected gate or error"),
        ("gate=gate,gate=gate,error=|_|0", "duplicate guard argument"),
    ] {
        let output = project.check(attribute, declaration);
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = project.check(
        "gate=gate,error=|_|0",
        "pub const fn work(gate: &Gate) -> usize {42}",
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("guard cannot protect a const function")
    );
    let output = project.check("gate=gate,error=|_|\"wrong\"", declaration);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("expected `usize`"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
