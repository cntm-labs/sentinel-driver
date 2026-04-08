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
        // ToSql::oid (instance method)
        let val = Decimal::ZERO;
        assert_eq!(val.oid(), Oid::NUMERIC);
        // FromSql::oid (static method) — covers line 140-142
        assert_eq!(<Decimal as FromSql>::oid(), Oid::NUMERIC);
    }

    #[test]
    fn test_numeric_decode_too_short() {
        // covers lines 146-149: header too short error
        let buf = [0u8; 4];
        assert!(Decimal::from_sql(&buf).is_err());
    }

    #[test]
    fn test_numeric_decode_nan() {
        // covers lines 158-160: NaN error
        // NaN header: ndigits=0, weight=0, sign=0xC000, dscale=0
        let buf: [u8; 8] = [0, 0, 0, 0, 0xC0, 0x00, 0, 0];
        let result = Decimal::from_sql(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_numeric_decode_truncated_digits() {
        // covers lines 169-174: truncated digit data error
        // Header says 2 digits (needs 12 bytes total) but only provide 10
        let buf: [u8; 10] = [
            0, 2, // ndigits = 2
            0, 0, // weight = 0
            0, 0, // sign = positive
            0, 2, // dscale = 2
            0, 1, // only 1 digit (2 bytes) instead of 2 digits (4 bytes)
        ];
        assert!(Decimal::from_sql(&buf).is_err());
    }

    #[test]
    fn test_numeric_with_trailing_zeros() {
        // covers lines 93-95, 113-115: trailing zero stripping + padding
        roundtrip(&Decimal::new(10000, 0)); // 10000 — produces trailing zero groups
        roundtrip(&Decimal::new(100, 2)); // 1.00 — scale with trailing zeros
    }

    #[test]
    fn test_numeric_pure_fractional_leading_zeros() {
        // covers line 106: pure fractional weight = -1
        roundtrip(&Decimal::new(5, 3)); // 0.005
        roundtrip(&Decimal::new(1, 8)); // 0.00000001
    }

    #[test]
    fn test_numeric_array_roundtrip() {
        let val = vec![
            Decimal::new(100, 2),   // 1.00
            Decimal::new(-5050, 2), // -50.50
            Decimal::ZERO,
        ];
        let mut buf = BytesMut::new();
        val.to_sql(&mut buf).ok();
        let decoded = Vec::<Decimal>::from_sql(&buf).ok();
        assert_eq!(decoded.as_ref(), Some(&val));
        assert_eq!(val.oid(), Oid::NUMERIC_ARRAY);
    }
}
