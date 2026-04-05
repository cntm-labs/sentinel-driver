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
        array_oid: None,
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
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::TIME,
        name: "time",
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::TIMESTAMP,
        name: "timestamp",
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::TIMESTAMPTZ,
        name: "timestamptz",
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::UUID,
        name: "uuid",
        array_oid: Some(Oid::UUID_ARRAY),
    },
    TypeInfo {
        oid: Oid::JSON,
        name: "json",
        array_oid: None,
    },
    TypeInfo {
        oid: Oid::JSONB,
        name: "jsonb",
        array_oid: None,
    },
];
