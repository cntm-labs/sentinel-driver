pub mod bit;
pub mod builtin;
pub mod decode;
pub mod encode;
pub mod geometric;
pub mod hstore;
pub mod interval;
#[cfg(feature = "with-serde-json")]
pub mod json;
pub mod lsn;
pub mod money;
pub mod multirange;
pub mod network;
#[cfg(feature = "with-rust-decimal")]
pub mod numeric;
pub mod oid;
pub mod range;
pub mod timetz;
pub mod traits;
pub mod xml;

// Re-export for backwards compatibility — all existing code uses `types::Oid`, `types::ToSql`, etc.
pub use oid::Oid;
pub use traits::{encode_param, encode_param_nullable, FromSql, ToSql};
