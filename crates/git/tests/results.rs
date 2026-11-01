mod support;

use radicula_git::{CONTRACT, Error};
use support::output;

#[test]
fn failures() {
    let failure = radicula_git::Reply::new(output(128, b"retained output"))
        .data()
        .unwrap_err();
    let Error::Failed {
        status,
        stdout,
        stderr,
    } = failure
    else {
        panic!("process failure")
    };
    assert_eq!(status.code(), Some(128));
    assert_eq!(stdout, b"retained output");
    assert_eq!(stderr, b"retained diagnostic");
    assert!(radicula_git::Reply::new(output(1, b"")).changes().is_err());
    assert!(
        radicula_git::Reply::new(output(1, b"wrong\n"))
            .path()
            .is_err()
    );
    assert!(
        radicula_git::Reply::new(output(1, b"wrong\n"))
            .object()
            .is_err()
    );
    assert!(
        radicula_git::Reply::new(output(1, b"false\n"))
            .flag()
            .is_err()
    );
    assert!(
        !radicula_git::Reply::new(output(1, b""))
            .data()
            .unwrap_err()
            .to_string()
            .contains("retained diagnostic")
    );
}

#[test]
fn booleans() {
    assert_eq!(
        radicula_git::Reply::new(output(0, b"")).ancestry(),
        Ok(true)
    );
    assert_eq!(
        radicula_git::Reply::new(output(1, b"")).ancestry(),
        Ok(false)
    );
    assert!(
        radicula_git::Reply::new(output(128, b""))
            .ancestry()
            .is_err()
    );
    assert_eq!(
        radicula_git::Reply::new(output(0, b"true\n")).flag(),
        Ok(true)
    );
    assert_eq!(
        radicula_git::Reply::new(output(0, b"false\n")).flag(),
        Ok(false)
    );
    assert!(
        radicula_git::Reply::new(output(0, b"false\nextra\n"))
            .flag()
            .is_err()
    );
}

#[test]
fn objects() {
    for length in [40, 64] {
        let hex = "a".repeat(length);
        assert_eq!(
            radicula_git::Reply::new(output(0, format!("{hex}\n").as_bytes())).object(),
            Ok(hex)
        );
    }
    for invalid in [
        "",
        "AAAA",
        "not-a-hash",
        " aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    ] {
        assert!(
            radicula_git::Reply::new(output(0, invalid.as_bytes()))
                .object()
                .is_err()
        );
    }
}

#[test]
fn vectors() {
    let contract: serde_json::Value = serde_json::from_str(CONTRACT).unwrap();
    for vector in contract["vectors"].as_array().unwrap() {
        check(vector);
    }
}

fn check(vector: &serde_json::Value) {
    let decoded =
        radicula_git::Reply::new(output(0, vector["input"].as_str().unwrap().as_bytes())).changes();
    if vector["error"] == true {
        assert!(decoded.is_err(), "{vector}");
        return;
    }
    let decoded = decoded.unwrap();
    assert_eq!(decoded.len(), vector["count"].as_u64().unwrap() as usize);
    if let Some(first) = decoded.first() {
        assert_eq!(
            first.path,
            std::path::Path::new(vector["path"].as_str().unwrap())
        );
        assert_eq!(
            first.score,
            vector["score"].as_u64().map(|value| value as u8)
        );
        assert_eq!(
            first.previous.as_deref(),
            vector["previous"].as_str().map(std::path::Path::new)
        );
    }
}

#[test]
fn paths() {
    assert_eq!(
        radicula_git::Reply::new(output(0, b" leading and trailing \n"))
            .path()
            .unwrap(),
        std::path::Path::new(" leading and trailing ")
    );
    assert_eq!(
        radicula_git::Reply::new(output(0, b"embedded\nnewline\n"))
            .path()
            .unwrap(),
        std::path::Path::new("embedded\nnewline")
    );
    assert!(
        radicula_git::Reply::new(output(0, b"a\0b\n"))
            .path()
            .is_err()
    );
    assert!(radicula_git::Reply::new(output(0, b"\n")).path().is_err());
}

#[test]
fn statuses() {
    let records = b"A\0a\0M\0m\0D\0d\0T\0t\0U\0u\0X\0x\0B\0b\0";
    assert_eq!(
        radicula_git::Reply::new(output(0, records))
            .changes()
            .unwrap()
            .len(),
        7
    );
    for invalid in [b"Rno\0a\0b\0".as_slice(), b"M999\0a\0", b"A\0a\0extra\0"] {
        assert!(
            radicula_git::Reply::new(output(0, invalid))
                .changes()
                .is_err()
        );
    }
}

#[cfg(unix)]
#[test]
fn bytes() {
    use std::os::unix::ffi::OsStrExt;
    let changes = radicula_git::Reply::new(output(0, b"A\0raw-\xff\0"))
        .changes()
        .unwrap();
    assert_eq!(changes[0].path.as_os_str().as_bytes(), b"raw-\xff");
    assert_eq!(
        radicula_git::Reply::new(output(0, b"raw-\xff\n"))
            .path()
            .unwrap()
            .as_os_str()
            .as_bytes(),
        b"raw-\xff"
    );
}
