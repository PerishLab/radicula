use radicula_crypto::ct::{self, Choice};

use crate::support::Rng;

const WORDS: [u64; 6] = [
    0,
    1,
    2,
    u64::MAX,
    0x8000_0000_0000_0000,
    0xdead_beef_0123_4567,
];

#[test]
fn flag() {
    assert!(Choice::from(true).flag());
    assert!(!Choice::from(false).flag());
}

#[test]
fn eq() {
    assert!(ct::eq(&[], &[]).flag());
    for left in 0..=255u8 {
        for right in 0..=255u8 {
            assert_eq!(ct::eq(&[left], &[right]).flag(), left == right);
        }
    }
    assert!(!ct::eq(&[1, 2], &[1, 2, 3]).flag());
    assert!(!ct::eq(&[1, 2, 3], &[1, 2]).flag());
    assert!(!ct::eq(&[], &[0]).flag());
    assert!(!ct::eq(&[0], &[]).flag());
}

fn flipped(data: &[u8], at: usize) {
    for bit in [0x01, 0x10, 0x80] {
        let mut other = data.to_vec();
        other[at] ^= bit;
        assert!(
            !ct::eq(data, &other).flag(),
            "length {} at {at}",
            data.len()
        );
        assert!(
            !ct::eq(&other, data).flag(),
            "length {} at {at}",
            data.len()
        );
    }
}

#[test]
fn flips() {
    let mut rng = Rng::new(7);
    for length in [1, 2, 3, 15, 16, 17, 31, 32, 33, 64, 100, 257] {
        let data = rng.bytes(length);
        assert!(ct::eq(&data, &data.clone()).flag(), "length {length}");
        for at in 0..length {
            flipped(&data, at);
        }
        let mut other = data.clone();
        for byte in other.iter_mut() {
            *byte = !*byte;
        }
        assert!(!ct::eq(&data, &other).flag(), "length {length} all");
    }
}

#[test]
fn select() {
    for (flag, left) in [true, false]
        .into_iter()
        .flat_map(|flag| WORDS.map(|word| (flag, word)))
    {
        for right in WORDS {
            let want = if flag { left } else { right };
            assert_eq!(Choice::from(flag).select(left, right), want);
        }
    }
}

#[test]
fn assign() {
    let mut rng = Rng::new(8);
    for length in 0..=70 {
        let source = rng.bytes(length);
        let target = rng.bytes(length);
        let mut taken = target.clone();
        Choice::from(true).assign(&mut taken, &source);
        assert_eq!(taken, source, "length {length}");
        let mut kept = target.clone();
        Choice::from(false).assign(&mut kept, &source);
        assert_eq!(kept, target, "length {length}");
    }
}

#[test]
#[should_panic(expected = "equal lengths")]
fn unequal() {
    Choice::from(true).assign(&mut [0u8; 2], &[1u8; 3]);
}

#[test]
#[should_panic(expected = "equal lengths")]
fn longer() {
    Choice::from(true).assign(&mut [0u8; 3], &[1u8; 2]);
}

#[test]
fn wipe() {
    for length in 0..=130 {
        let mut bytes = vec![0xaau8; length + 2];
        ct::wipe(&mut bytes[1..=length]);
        assert_eq!(bytes[0], 0xaa);
        assert_eq!(bytes[length + 1], 0xaa);
        assert!(
            bytes[1..=length].iter().all(|byte| *byte == 0),
            "length {length}"
        );
        let mut halves = vec![0xaaaa_aaaau32; length + 2];
        ct::wipe(&mut halves[1..=length]);
        assert_eq!((halves[0], halves[length + 1]), (0xaaaa_aaaa, 0xaaaa_aaaa));
        assert!(
            halves[1..=length].iter().all(|word| *word == 0),
            "length {length}"
        );
        let mut words = vec![u64::MAX; length + 2];
        ct::wipe(&mut words[1..=length]);
        assert_eq!((words[0], words[length + 1]), (u64::MAX, u64::MAX));
        assert!(
            words[1..=length].iter().all(|word| *word == 0),
            "length {length}"
        );
    }
}
