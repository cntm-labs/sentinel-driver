use crate::types::Oid;

/// Information about a PostgreSQL built-in type.
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub oid: Oid,
    pub name: &'static str,
    pub array_oid: Option<Oid>,
}

/// Look up type info by OID.
pub fn lookup(oid: Oid) -> Option<&'static TypeInfo> {
    BUILTIN_TYPES.iter().find(|t| t.oid == oid)
}

/// Look up type info by name.
pub fn lookup_by_name(name: &str) -> Option<&'static TypeInfo> {
    BUILTIN_TYPES.iter().find(|t| t.name == name)
}

static BUILTIN_TYPES: &[TypeInfo] = &[
    TypeInfo {
        oid: Oid::BOOL,
        name: "bool",
        array_oid: Some(Oid::BOOL_ARRAY),
    },
    TypeInfo {
        oid: Oid::BYTEA,
        name: "bytea",
        array_oid: Some(Oid::BYTEA_ARRAY),
    },
    TypeInfo {
        oid: Oid::CHAR,
        name: "char",
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::INT8,
        name: "int8",
        array_oid: Some(Oid::INT8_ARRAY),
    },
    TypeInfo {
        oid: Oid::INT2,
        name: "int2",
        array_oid: Some(Oid::INT2_ARRAY),
    },
    TypeInfo {
        oid: Oid::INT4,
        name: "int4",
        array_oid: Some(Oid::INT4_ARRAY),
    },
    TypeInfo {
        oid: Oid::TEXT,
        name: "text",
        array_oid: Some(Oid::TEXT_ARRAY),
    },
    TypeInfo {
        oid: Oid::OID,
        name: "oid",
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::FLOAT4,
        name: "float4",
        array_oid: Some(Oid::FLOAT4_ARRAY),
    },
    TypeInfo {
        oid: Oid::FLOAT8,
        name: "float8",
        array_oid: Some(Oid::FLOAT8_ARRAY),
    },
    TypeInfo {
        oid: Oid::VARCHAR,
        name: "varchar",
        array_oid: Some(Oid::VARCHAR_ARRAY),
    },
    TypeInfo {
        oid: Oid::DATE,
        name: "date",
        array_oid: Some(Oid::DATE_ARRAY),
    },
    TypeInfo {
        oid: Oid::TIME,
        name: "time",
        array_oid: Some(Oid::TIME_ARRAY),
    },
    TypeInfo {
        oid: Oid::TIMETZ,
        name: "timetz",
        array_oid: Some(Oid::TIMETZ_ARRAY),
    },
    TypeInfo {
        oid: Oid::TIMESTAMP,
        name: "timestamp",
        array_oid: Some(Oid::TIMESTAMP_ARRAY),
    },
    TypeInfo {
        oid: Oid::TIMESTAMPTZ,
        name: "timestamptz",
        array_oid: Some(Oid::TIMESTAMPTZ_ARRAY),
    },
    TypeInfo {
        oid: Oid::UUID,
        name: "uuid",
        array_oid: Some(Oid::UUID_ARRAY),
    },
    TypeInfo {
        oid: Oid::JSON,
        name: "json",
        array_oid: Some(Oid::JSON_ARRAY),
    },
    TypeInfo {
        oid: Oid::JSONB,
        name: "jsonb",
        array_oid: Some(Oid::JSONB_ARRAY),
    },
    TypeInfo {
        oid: Oid::INTERVAL,
        name: "interval",
        array_oid: Some(Oid::INTERVAL_ARRAY),
    },
    TypeInfo {
        oid: Oid::INET,
        name: "inet",
        array_oid: Some(Oid::INET_ARRAY),
    },
    TypeInfo {
        oid: Oid::CIDR,
        name: "cidr",
        array_oid: Some(Oid::CIDR_ARRAY),
    },
    TypeInfo {
        oid: Oid::MACADDR,
        name: "macaddr",
        array_oid: Some(Oid::MACADDR_ARRAY),
    },
    TypeInfo {
        oid: Oid::MACADDR8,
        name: "macaddr8",
        array_oid: Some(Oid::MACADDR8_ARRAY),
    },
    TypeInfo {
        oid: Oid::NUMERIC,
        name: "numeric",
        array_oid: Some(Oid::NUMERIC_ARRAY),
    },
    TypeInfo {
        oid: Oid::INT4RANGE,
        name: "int4range",
        array_oid: Some(Oid::INT4RANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::INT8RANGE,
        name: "int8range",
        array_oid: Some(Oid::INT8RANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::NUMRANGE,
        name: "numrange",
        array_oid: Some(Oid::NUMRANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::TSRANGE,
        name: "tsrange",
        array_oid: Some(Oid::TSRANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::TSTZRANGE,
        name: "tstzrange",
        array_oid: Some(Oid::TSTZRANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::DATERANGE,
        name: "daterange",
        array_oid: Some(Oid::DATERANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::INT4MULTIRANGE,
        name: "int4multirange",
        array_oid: Some(Oid::INT4MULTIRANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::INT8MULTIRANGE,
        name: "int8multirange",
        array_oid: Some(Oid::INT8MULTIRANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::NUMMULTIRANGE,
        name: "nummultirange",
        array_oid: Some(Oid::NUMMULTIRANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::TSMULTIRANGE,
        name: "tsmultirange",
        array_oid: Some(Oid::TSMULTIRANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::TSTZMULTIRANGE,
        name: "tstzmultirange",
        array_oid: Some(Oid::TSTZMULTIRANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::DATEMULTIRANGE,
        name: "datemultirange",
        array_oid: Some(Oid::DATEMULTIRANGE_ARRAY),
    },
    TypeInfo {
        oid: Oid::MONEY,
        name: "money",
        array_oid: Some(Oid::MONEY_ARRAY),
    },
    TypeInfo {
        oid: Oid::POINT,
        name: "point",
        array_oid: Some(Oid::POINT_ARRAY),
    },
    TypeInfo {
        oid: Oid::LINE,
        name: "line",
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::LSEG,
        name: "lseg",
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::PG_BOX,
        name: "box",
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::CIRCLE,
        name: "circle",
        array_oid: Some(Oid::CIRCLE_ARRAY),
    },
    TypeInfo {
        oid: Oid::XML,
        name: "xml",
        array_oid: Some(Oid::XML_ARRAY),
    },
    TypeInfo {
        oid: Oid::PG_LSN,
        name: "pg_lsn",
        array_oid: Some(Oid::PG_LSN_ARRAY),
    },
    TypeInfo {
        oid: Oid::BIT,
        name: "bit",
        array_oid: Some(Oid::BIT_ARRAY),
    },
    TypeInfo {
        oid: Oid::VARBIT,
        name: "varbit",
        array_oid: Some(Oid::VARBIT_ARRAY),
    },
];
