use serde::Serializer;
use serde_aux::prelude::deserialize_string_from_number;

pub(crate) mod isa;
mod parity_test;
pub(crate) mod specs;

/// A `ChipSpec` defines the various hardware objects that are available
/// when compiling a quantum program.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChipSpec {
    /// The "Instruction Set Architecture" of the chip; i.e. the
    /// instructions (or "gates") that a particular hardware object
    /// supports.
    pub isa: isa::Isa,
    /// The various operating characteristics of a hardware object (e.g
    /// readout fidelity).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs: Option<specs::Specs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(deserialize_with = "deserialize_string_from_number")]
    #[serde(serialize_with = "maybe_serialize_string_to_integer")]
    pub version: String,
}

/// Serialize the input to an `i32` if it can be parsed as such; otherwise
/// serialize it as a string.
fn maybe_serialize_string_to_integer<S>(s: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match s.parse::<i32>() {
        Ok(i) => serializer.serialize_i32(i),
        Err(_) => serializer.serialize_str(s),
    }
}
