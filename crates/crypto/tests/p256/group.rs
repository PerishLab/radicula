use radicula_crypto::Error;
use radicula_crypto::p256::Point;

use crate::support::{ORDER, Rng, array, field, hex, load, scalar};

fn encoded(point: &Point) -> Result<[u8; 65], Error> {
    point.encode()
}

fn same(left: &Point, right: &Point) -> bool {
    encoded(left) == encoded(right)
}

fn at(rng: &mut Rng) -> Point {
    Point::base().mul(&scalar(rng))
}

fn small(value: u8) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[31] = value;
    out
}

fn expect(point: &Point, text: &str) {
    if text.is_empty() {
        assert!(point.neutral());
        assert_eq!(encoded(point), Err(Error::Zero));
    } else {
        assert!(!point.neutral());
        assert_eq!(encoded(point).unwrap().to_vec(), hex(text));
    }
}

#[test]
fn doubling() {
    let mut rng = Rng::new(1);
    let mut points = vec![Point::base(), Point::base().double()];
    points.extend((0..8).map(|_| at(&mut rng)));
    for point in &points {
        assert!(same(&point.add(point), &point.double()));
    }
}

#[test]
fn opposite() {
    let mut rng = Rng::new(2);
    for _ in 0..8 {
        let point = at(&mut rng);
        assert!(point.add(&point.negate()).neutral());
        assert!(point.negate().add(&point).neutral());
        assert!(same(&point.negate().negate(), &point));
    }
    assert!(Point::base().add(&Point::base().negate()).neutral());
}

#[test]
fn neutral() {
    let mut rng = Rng::new(3);
    let zero = Point::identity();
    assert!(zero.neutral());
    assert!(zero.add(&zero).neutral());
    assert!(zero.double().neutral());
    assert!(zero.negate().neutral());
    assert_eq!(encoded(&zero), Err(Error::Zero));
    for _ in 0..4 {
        let point = at(&mut rng);
        assert!(same(&zero.add(&point), &point));
        assert!(same(&point.add(&zero), &point));
    }
    assert!(Point::base().mul(&[0u8; 32]).neutral());
    assert!(zero.mul(&scalar(&mut rng)).neutral());
}

#[test]
fn algebra() {
    let mut rng = Rng::new(4);
    for _ in 0..6 {
        let (a, b, c) = (at(&mut rng), at(&mut rng), at(&mut rng));
        assert!(same(&a.add(&b), &b.add(&a)));
        assert!(same(&a.add(&b).add(&c), &a.add(&b.add(&c))));
        assert!(same(&a.add(&b).double(), &a.double().add(&b.double())));
    }
}

#[test]
fn scaled() {
    let two = Point::base().double();
    let three = two.add(&Point::base());
    let six = three.add(&three);
    assert!(same(&six, &three.double()));
    assert!(same(&six, &Point::base().mul(&small(6))));
    let mixed = two.add(&three).add(&Point::base());
    assert!(same(&mixed, &six));
    assert!(same(&three.add(&two), &two.add(&three)));
    assert!(same(&six.add(&six.negate().add(&three)), &three));
}

#[test]
fn routes() {
    let base = Point::base();
    let two = base.add(&base);
    let three = [two.add(&base), base.add(&two), base.double().add(&base)];
    let want = base.mul(&small(3));
    for point in &three {
        assert!(same(point, &want));
    }
    assert!(same(&two, &base.mul(&small(2))));
    let encoded = two.encode().unwrap();
    assert_eq!(
        encoded[1..33].to_vec(),
        hex("7cf27b188d034f7e8a52380304b51ac3c08969e277f21b35a60b48fc47669978")
    );
    assert_eq!(
        encoded[33..].to_vec(),
        hex("07775510db8ed040293d9ac69f7430dbba7dade63ce982299e04b79d227873d1")
    );
    let triple = base.mul(&small(3)).encode().unwrap();
    assert_eq!(
        triple[1..33].to_vec(),
        hex("5ecbe4d1a6330a44c8f7ef951d4bf165e6c6b721efada985fb41661bc6e7fd6c")
    );
    assert_eq!(
        triple[33..].to_vec(),
        hex("8734640c4998ff7e374b06ce1a64a2ecd82ab036384fb83d9a79b127a27d5032")
    );
    assert!(same(&two, &base.mul(&small(1)).double()));
}

#[test]
fn multiples() {
    let data = load("multiples.json");
    let mut seen = 0;
    for row in data["base"].as_array().unwrap() {
        expect(
            &Point::base().mul(&array(&field(row, "k"))),
            row["point"].as_str().unwrap(),
        );
        seen += 1;
    }
    let other = &data["other"];
    let point = Point::decode(&field(other, "point")).unwrap();
    for row in other["cases"].as_array().unwrap() {
        expect(
            &point.mul(&array(&field(row, "k"))),
            row["point"].as_str().unwrap(),
        );
        seen += 1;
    }
    assert_eq!(seen, 128);
}

#[test]
fn order() {
    let base = Point::base();
    assert!(base.mul(&ORDER).neutral());
    let below = crate::support::subtract(&ORDER, &small(1));
    assert!(same(&base.mul(&below), &base.negate()));
    assert!(base.mul(&below).add(&base).neutral());
    let mut above = ORDER;
    above[31] += 1;
    assert!(same(&base.mul(&above), &base));
    assert!(same(
        &base.mul(&[0xff; 32]),
        &base.mul(&crate::support::subtract(&[0xff; 32], &ORDER))
    ));
}

#[test]
fn windows() {
    let base = Point::base();
    let mut chain = Point::identity();
    for digit in 0u8..16 {
        assert!(same(&chain, &base.mul(&small(digit))));
        chain = chain.add(&base);
    }
    for digit in [1u8, 15] {
        let mut want = base.mul(&small(digit));
        for position in 0..64 {
            let mut bytes = [0u8; 32];
            bytes[31 - position / 2] = digit << (4 * (position % 2));
            assert!(
                same(&base.mul(&bytes), &want),
                "digit {digit} at {position}"
            );
            for _ in 0..4 {
                want = want.double();
            }
        }
    }
}
