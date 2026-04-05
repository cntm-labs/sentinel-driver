#[cfg(feature = "with-rust-decimal")]
mod numeric_tests {
    use bytes::BytesMut;
    use rust_decimal::Decimal;
    use sentinel_driver::types::{FromSql, Oid, ToSql};

    fn roundtrip(val: &Decimal) {
        let mut buf = BytesMut::new();
        val.to_sql(&mut buf).ok();
        let decoded = Decimal::from_sql(&buf).ok();
        assert_eq!(decoded, Some(*val));
    }

    #[test]
    fn test_numeric_zero() {
        roundtrip(&Decimal::ZERO);
    }

    #[test]
    fn test_numeric_positive_integer() {
        roundtrip(&Decimal::new(12345, 0));
    }

    #[test]
    fn test_numeric_negative() {
        roundtrip(&Decimal::new(-99999, 0));
    }

    #[test]
    fn test_numeric_with_scale() {
        roundtrip(&Decimal::new(31415, 4)); // 3.1415
    }

    #[test]
    fn test_numeric_small_decimal() {
        roundtrip(&Decimal::new(1, 10)); // 0.0000000001
    }

    #[test]
    fn test_numeric_large() {
        roundtrip(&Decimal::new(999_999_999_999, 2)); // 9999999999.99
    }

    #[test]
    fn test_numeric_one() {
        roundtrip(&Decimal::ONE);
    }

    #[test]
    fn test_numeric_oid() {
        let val = Decimal::ZERO;
        assert_eq!(val.oid(), Oid::NUMERIC);
    }
}
