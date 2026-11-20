use radicula_crypto::{Aes, Error};

use crate::support::{array, field, load};

#[test]
fn block() {
    let rows = load("gcm/fips197.json")["encrypt"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(rows.len(), 3);
    for row in rows {
        let source = row["source"].as_str().unwrap();
        let cipher = Aes::new(&field(&row, "key")).unwrap();
        let mut block: [u8; 16] = array(&field(&row, "input"));
        cipher.encrypt(&mut block);
        assert_eq!(block[..], field(&row, "output")[..], "{source}");
    }
}

#[test]
fn keys() {
    for size in [0, 1, 15, 17, 23, 24, 25, 31, 33, 64] {
        assert_eq!(
            Aes::new(&vec![7u8; size]).err(),
            Some(Error::Length),
            "{size}"
        );
    }
    assert!(Aes::new(&[7u8; 16]).is_ok());
    assert!(Aes::new(&[7u8; 32]).is_ok());
}
