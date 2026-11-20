use radicula_crypto::{Aesgcm, Error};

use crate::support::{array, records};

#[test]
fn encrypt() {
    for (name, size) in [
        ("gcmEncryptExtIV128.rsp", 16),
        ("gcmEncryptExtIV256.rsp", 32),
    ] {
        let rows = records(name);
        assert_eq!(rows.len(), 375, "{name}");
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.key.len(), size);
            let cipher = Aesgcm::new(&row.key).unwrap();
            let mut data = row.plain.clone();
            let tag = cipher
                .seal(&array(&row.nonce), &row.aad, &mut data)
                .unwrap();
            assert_eq!(
                (&data, &tag[..]),
                (&row.cipher, &row.tag[..]),
                "{name} {index}"
            );
        }
    }
}

#[test]
fn decrypt() {
    for (name, size) in [("gcmDecrypt128.rsp", 16), ("gcmDecrypt256.rsp", 32)] {
        let rows = records(name);
        assert_eq!(rows.len(), 375, "{name}");
        let failed = rows.iter().filter(|row| row.fail).count();
        assert!(failed > 100 && failed < 275, "{name}");
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.key.len(), size);
            let cipher = Aesgcm::new(&row.key).unwrap();
            let mut data = row.cipher.clone();
            let result = cipher.open(&array(&row.nonce), &row.aad, &mut data, &row.tag);
            if row.fail {
                assert_eq!(result, Err(Error::Auth), "{name} {index}");
                assert_eq!(data, row.cipher, "{name} {index} untouched");
            } else {
                assert_eq!(result, Ok(()), "{name} {index}");
                assert_eq!(data, row.plain, "{name} {index}");
            }
        }
    }
}
