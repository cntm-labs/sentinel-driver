/// Well-known PostgreSQL type OIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Oid(pub u32);

impl Oid {
    pub const BOOL: Oid = Oid(16);
    pub const BYTEA: Oid = Oid(17);
    pub const CHAR: Oid = Oid(18);
    pub const INT8: Oid = Oid(20);
    pub const INT2: Oid = Oid(21);
    pub const INT4: Oid = Oid(23);
    pub const TEXT: Oid = Oid(25);
    pub const OID: Oid = Oid(26);
    pub const FLOAT4: Oid = Oid(700);
    pub const FLOAT8: Oid = Oid(701);
    pub const VARCHAR: Oid = Oid(1043);
    pub const DATE: Oid = Oid(1082);
    pub const TIME: Oid = Oid(1083);
    pub const TIMETZ: Oid = Oid(1266);
    pub const TIMETZ_ARRAY: Oid = Oid(1270);
    pub const TIMESTAMP: Oid = Oid(1114);
    pub const TIMESTAMPTZ: Oid = Oid(1184);
    pub const UUID: Oid = Oid(2950);
    pub const JSONB: Oid = Oid(3802);
    pub const JSON: Oid = Oid(114);
    pub const INET: Oid = Oid(869);
    pub const CIDR: Oid = Oid(650);
    pub const INET_ARRAY: Oid = Oid(1041);
    pub const CIDR_ARRAY: Oid = Oid(651);
    pub const MACADDR: Oid = Oid(829);
    pub const MACADDR_ARRAY: Oid = Oid(1040);
    pub const INTERVAL: Oid = Oid(1186);
    pub const INTERVAL_ARRAY: Oid = Oid(1187);
    pub const NUMERIC: Oid = Oid(1700);
    pub const NUMERIC_ARRAY: Oid = Oid(1231);
    pub const INT4RANGE: Oid = Oid(3904);
    pub const INT8RANGE: Oid = Oid(3926);
    pub const NUMRANGE: Oid = Oid(3906);
    pub const TSRANGE: Oid = Oid(3908);
    pub const TSTZRANGE: Oid = Oid(3910);
    pub const DATERANGE: Oid = Oid(3912);
    pub const INT4RANGE_ARRAY: Oid = Oid(3905);
    pub const INT8RANGE_ARRAY: Oid = Oid(3927);
    pub const NUMRANGE_ARRAY: Oid = Oid(3907);
    pub const TSRANGE_ARRAY: Oid = Oid(3909);
    pub const TSTZRANGE_ARRAY: Oid = Oid(3911);
    pub const DATERANGE_ARRAY: Oid = Oid(3913);
    pub const INT4MULTIRANGE: Oid = Oid(4451);
    pub const INT8MULTIRANGE: Oid = Oid(4536);
    pub const NUMMULTIRANGE: Oid = Oid(4532);
    pub const TSMULTIRANGE: Oid = Oid(4533);
    pub const TSTZMULTIRANGE: Oid = Oid(4534);
    pub const DATEMULTIRANGE: Oid = Oid(4535);
    pub const INT4MULTIRANGE_ARRAY: Oid = Oid(6150);
    pub const INT8MULTIRANGE_ARRAY: Oid = Oid(6157);
    pub const NUMMULTIRANGE_ARRAY: Oid = Oid(6151);
    pub const TSMULTIRANGE_ARRAY: Oid = Oid(6152);
    pub const TSTZMULTIRANGE_ARRAY: Oid = Oid(6153);
    pub const DATEMULTIRANGE_ARRAY: Oid = Oid(6155);
    pub const MONEY: Oid = Oid(790);
    pub const MONEY_ARRAY: Oid = Oid(791);
    pub const POINT: Oid = Oid(600);
    pub const LINE: Oid = Oid(628);
    pub const LSEG: Oid = Oid(601);
    pub const PG_BOX: Oid = Oid(603);
    pub const PATH: Oid = Oid(602);
    pub const POLYGON: Oid = Oid(604);
    pub const CIRCLE: Oid = Oid(718);
    pub const MACADDR8: Oid = Oid(774);
    pub const MACADDR8_ARRAY: Oid = Oid(775);
    pub const XML: Oid = Oid(142);
    pub const XML_ARRAY: Oid = Oid(143);
    pub const PG_LSN: Oid = Oid(3220);
    pub const PG_LSN_ARRAY: Oid = Oid(3221);
    pub const BIT: Oid = Oid(1560);
    pub const BIT_ARRAY: Oid = Oid(1561);
    pub const VARBIT: Oid = Oid(1562);
    pub const VARBIT_ARRAY: Oid = Oid(1563);

    // Array types
    pub const BOOL_ARRAY: Oid = Oid(1000);
    pub const BYTEA_ARRAY: Oid = Oid(1001);
    pub const INT2_ARRAY: Oid = Oid(1005);
    pub const INT4_ARRAY: Oid = Oid(1007);
    pub const INT8_ARRAY: Oid = Oid(1016);
    pub const FLOAT4_ARRAY: Oid = Oid(1021);
    pub const FLOAT8_ARRAY: Oid = Oid(1022);
    pub const TEXT_ARRAY: Oid = Oid(1009);
    pub const VARCHAR_ARRAY: Oid = Oid(1015);
    pub const TIMESTAMP_ARRAY: Oid = Oid(1115);
    pub const TIMESTAMPTZ_ARRAY: Oid = Oid(1185);
    pub const DATE_ARRAY: Oid = Oid(1182);
    pub const TIME_ARRAY: Oid = Oid(1183);
    pub const JSON_ARRAY: Oid = Oid(199);
    pub const JSONB_ARRAY: Oid = Oid(3807);
    pub const POINT_ARRAY: Oid = Oid(1017);
    pub const CIRCLE_ARRAY: Oid = Oid(719);
    pub const UUID_ARRAY: Oid = Oid(2951);
}

impl From<u32> for Oid {
    fn from(v: u32) -> Self {
        Oid(v)
    }
}

impl From<Oid> for u32 {
    fn from(oid: Oid) -> Self {
        oid.0
    }
}
