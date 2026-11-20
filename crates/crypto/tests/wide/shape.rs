macro_rules! shape {
    ($size:expr) => {
        use radicula_crypto::Error;

        const LIMIT: usize = 255 * $size;

        fn tag(key: &[u8], data: &[u8]) -> Vec<u8> {
            let mut mac = Mac::new(key);
            mac.update(data);
            mac.finish().to_vec()
        }

        #[test]
        fn sizes() {
            assert_eq!(Mac::SIZE, $size);
            assert_eq!(Kdf::LIMIT, LIMIT);
            assert_eq!(Mac::new(b"key").finish().len(), $size);
            assert_eq!(Hash::digest(b"x").len(), Mac::SIZE);
        }

        #[test]
        fn clones() {
            let mut base = Mac::new(b"key");
            base.update(b"abc");
            let mut twin = base.clone();
            base.update(b"def");
            twin.update(b"xyz");
            assert_eq!(base.finish().to_vec(), tag(b"key", b"abcdef"));
            assert_eq!(twin.finish().to_vec(), tag(b"key", b"abcxyz"));
        }

        #[test]
        fn verified() {
            let mut mac = Mac::new(b"key");
            mac.update(b"message");
            let good = mac.clone().finish();
            assert!(mac.clone().verify(&good));
            assert!(!mac.clone().verify(&good[..$size - 1]));
            assert!(!mac.clone().verify(&[good.as_slice(), &[0]].concat()));
            assert!(!mac.clone().verify(&[]));
            assert!(!mac.verify(&[0; $size]));
        }

        #[test]
        fn padded() {
            let mut rng = crate::support::Rng::new(11);
            for size in [0, 1, 50, 127] {
                let key = rng.bytes(size);
                let mut zeroed = key.clone();
                zeroed.push(0);
                assert_eq!(tag(&key, b"data"), tag(&zeroed, b"data"), "key {size}");
            }
            let key = rng.bytes(128);
            let mut longer = key.clone();
            longer.push(0);
            assert_ne!(tag(&key, b"data"), tag(&longer, b"data"));
        }

        #[test]
        fn hashed() {
            let mut rng = crate::support::Rng::new(12);
            for size in [129, 130, 200, 1000] {
                let key = rng.bytes(size);
                let short = Hash::digest(&key);
                assert_eq!(tag(&key, b"data"), tag(&short, b"data"), "key {size}");
            }
        }

        #[test]
        fn extracted() {
            let first = Kdf::extract(b"salt", b"input");
            let second = Kdf::new(&tag(b"salt", b"input")).unwrap();
            let mut left = [0u8; 100];
            let mut right = [0u8; 100];
            first.expand(b"info", &mut left).unwrap();
            second.expand(b"info", &mut right).unwrap();
            assert_eq!(left, right);
            let mut other = [0u8; 100];
            Kdf::extract(b"salt", b"inpuT")
                .expand(b"info", &mut other)
                .unwrap();
            assert_ne!(left, other);
            Kdf::extract(b"salt", b"input")
                .expand(b"infO", &mut other)
                .unwrap();
            assert_ne!(left, other);
        }

        #[test]
        fn prefixes() {
            let kdf = Kdf::extract(b"salt", b"input");
            let mut long = vec![0u8; 500];
            kdf.expand(b"info", &mut long).unwrap();
            for size in [0, 1, $size - 1, $size, $size + 1, 2 * $size, 499] {
                let mut short = vec![0u8; size];
                kdf.expand(b"info", &mut short).unwrap();
                assert_eq!(short[..], long[..size], "size {size}");
            }
            let mut again = vec![0u8; 500];
            kdf.expand(b"info", &mut again).unwrap();
            assert_eq!(again, long);
        }

        #[test]
        fn limit() {
            let kdf = Kdf::extract(b"salt", b"input");
            let mut fit = vec![0xee; LIMIT];
            kdf.expand(b"info", &mut fit).unwrap();
            assert!(fit.iter().any(|byte| *byte != 0xee));
            for size in [LIMIT + 1, LIMIT + 100, 100_000] {
                let mut out = vec![0xee; size];
                assert_eq!(kdf.expand(b"info", &mut out), Err(Error::Length));
                assert!(out.iter().all(|byte| *byte == 0xee), "size {size}");
            }
        }

        #[test]
        fn short() {
            for size in [0, 1, $size - 1] {
                assert_eq!(Kdf::new(&vec![7; size]).err(), Some(Error::Length));
            }
            for size in [$size, $size + 1, 128, 129, 300] {
                assert!(Kdf::new(&vec![7; size]).is_ok(), "size {size}");
            }
        }
    };
}

mod small {
    use radicula_crypto::{Hkdf384 as Kdf, Hmac384 as Mac, Sha384 as Hash};

    shape!(48);
}

mod large {
    use radicula_crypto::{Hkdf512 as Kdf, Hmac512 as Mac, Sha512 as Hash};

    shape!(64);
}
