//! The supported Instruction Set Architecture for a chip specification

use serde::{de::Deserializer, Deserialize, Serializer};

use std::collections::HashMap;

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Isa {
    /// The degree-0 (i.e. single qubit) hardware objects and their
    /// supported instructions.
    #[serde(rename(deserialize = "1Q"))]
    #[serde(rename(serialize = "1Q"))]
    pub one_q: HashMap<String, Option<OneQ>>,
    /// The degree-1 (i.e. two qubit) hardware objects and their
    /// supported instructions.
    #[serde(rename(deserialize = "2Q"))]
    #[serde(rename(serialize = "2Q"))]
    pub two_q: HashMap<String, Option<TwoQ>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Metadata {}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename(serialize = "1Q"))]
#[serde(rename_all = "lowercase")]
#[serde(untagged)]
pub enum OneQ {
    /// The set of supported gates
    Gates { gates: Vec<Gate> },
    /// DEPRECATED. A gateset identifier known to quilc
    ///
    /// In practice, the only supported identifier here is "Xhalves"
    /// and this style of specifying gates in the ISA is deprecated.
    Ty {
        #[serde(rename(deserialize = "type"))]
        #[serde(rename(serialize = "type"))]
        ty: String,
    },
    /// DEPRECATED. Define gates by their associated specs.
    Specs { specs: crate::chip::specs::SpecsMap },
    /// Qubit exists physically but should not be used for computation
    Dead { dead: bool },
    /// Use a set of (quilc-specified) default gates
    Defaults {},
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename(serialize = "1Q"))]
#[serde(rename_all = "lowercase")]
#[serde(untagged)]
pub enum TwoQ {
    /// The set of supported gates
    Gates { gates: Vec<Gate> },
    /// See documentation for [`OneQ::Ty`].
    Ty {
        #[serde(rename(deserialize = "type"))]
        #[serde(rename(serialize = "type"))]
        ty: Vec<String>,
    },
    /// See documentation for [`OneQ::Specs`].
    Specs { specs: crate::chip::specs::SpecsMap },
    /// Qubit exists physically but should not be used for computation
    Dead { dead: bool },
    /// Use a set of (quilc-specified) default gates
    Defaults {},
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename = "lowercase")]
#[serde(untagged)]
pub enum Gate {
    Measure {
        operator: monostate::MustBe!("MEASURE"),
        qubit: Qubit,
        target: Option<MeasurementTarget>,
        duration: f64,
        fidelity: f64,
    },
    Quantum {
        operator: String,
        parameters: Vec<Parameter>,
        arguments: Vec<Argument>,
        duration: f64,
        fidelity: f64,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[serde(untagged)]
pub enum Qubit {
    #[serde(deserialize_with = "deserialize_wildcard")]
    #[serde(serialize_with = "serialize_wildcard")]
    Wildcard,
    Index(usize),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MeasurementTarget {
    #[serde(deserialize_with = "deserialize_wildcard")]
    #[serde(serialize_with = "serialize_wildcard")]
    Wildcard,
    MemoryReference(String),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Parameter {
    #[serde(deserialize_with = "deserialize_wildcard")]
    #[serde(serialize_with = "serialize_wildcard")]
    Wildcard,
    Numeric(f64),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Argument {
    #[serde(deserialize_with = "deserialize_wildcard")]
    #[serde(serialize_with = "serialize_wildcard")]
    Wildcard,
    Index(usize),
}

fn deserialize_wildcard<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    if &buf == "_" {
        Ok(())
    } else {
        Err(serde::de::Error::custom("input does not match wildcard"))
    }
}

fn serialize_wildcard<S>(serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str("_")
}

#[cfg(test)]
mod test {
    use super::*;
    use monostate::MustBe;
    use serde_json::json;

    #[test]
    fn deserialize_oneq() {
        struct Test {
            input: serde_json::Value,
            expected: OneQ,
        }
        let tests = [
            Test {
                input: json!({}),
                expected: OneQ::Defaults {},
            },
            Test {
                input: json!({"gates": []}),
                expected: OneQ::Gates { gates: vec![] },
            },
        ];
        for test in tests {
            let actual: OneQ = serde_json::from_value(test.input).unwrap();
            assert_eq!(actual, test.expected);
        }
    }

    #[test]
    fn deserialize_gate() {
        struct Test {
            input: serde_json::Value,
            expected: Gate,
        }
        let tests = [
            Test {
                input: json!({"operator": "MEASURE", "qubit": 42, "target": "ro", "duration": 0.1, "fidelity": 0.9}),
                expected: Gate::Measure {
                    operator: MustBe!("MEASURE"),
                    qubit: Qubit::Index(42),
                    target: Some(MeasurementTarget::MemoryReference("ro".to_string())),
                    duration: 0.1,
                    fidelity: 0.9,
                },
            },
            Test {
                input: json!({"operator": "RX", "parameters": [1.5], "arguments": [42], "duration": 0.1, "fidelity": 0.9}),
                expected: Gate::Quantum {
                    operator: "RX".to_string(),
                    parameters: vec![Parameter::Numeric(1.5)],
                    arguments: vec![Argument::Index(42)],
                    duration: 0.1,
                    fidelity: 0.9,
                },
            },
        ];

        for test in tests {
            let actual: Gate = serde_json::from_value(test.input).unwrap();
            assert_eq!(actual, test.expected);
        }
    }

    #[test]
    fn deserialize_qubit() {
        struct Test {
            input: serde_json::Value,
            expected: Qubit,
        }
        let tests = [
            Test {
                input: json!("_"),
                expected: Qubit::Wildcard,
            },
            Test {
                input: json!(42),
                expected: Qubit::Index(42),
            },
        ];

        for test in tests {
            let actual: Qubit = serde_json::from_value(test.input).unwrap();
            assert_eq!(actual, test.expected);
        }
    }

    #[test]
    fn deserialize_argument() {
        struct Test {
            input: serde_json::Value,
            expected: Argument,
        }

        let tests = [
            Test {
                input: json!("_"),
                expected: Argument::Wildcard,
            },
            Test {
                input: json!(42),
                expected: Argument::Index(42),
            },
        ];

        for test in tests {
            let actual: Argument = serde_json::from_value(test.input).unwrap();
            assert_eq!(actual, test.expected);
        }
    }

    #[test]
    fn deserialize_parameter() {
        struct Test {
            input: serde_json::Value,
            expected: Parameter,
        }

        let tests = [
            Test {
                input: json!("_"),
                expected: Parameter::Wildcard,
            },
            Test {
                input: json!(1.5),
                expected: Parameter::Numeric(1.5),
            },
        ];

        for test in tests {
            let actual: Parameter = serde_json::from_value(test.input).unwrap();
            assert_eq!(actual, test.expected);
        }
    }

    #[test]
    fn deserialize_measurement_target() {
        struct Test {
            input: serde_json::Value,
            expected: MeasurementTarget,
        }

        let tests = [
            Test {
                input: json!("_"),
                expected: MeasurementTarget::Wildcard,
            },
            Test {
                input: json!("ro"),
                expected: MeasurementTarget::MemoryReference("ro".to_string()),
            },
        ];

        for test in tests {
            let actual: MeasurementTarget = serde_json::from_value(test.input).unwrap();
            assert_eq!(actual, test.expected);
        }
    }
}
