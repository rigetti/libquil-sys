use std::collections::HashMap;

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Specs {
    #[serde(rename(deserialize = "1Q"))]
    #[serde(rename(serialize = "1Q"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_q: Option<HashMap<String, SpecsMap>>,
    #[serde(rename(deserialize = "2Q"))]
    #[serde(rename(serialize = "2Q"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_q: Option<HashMap<String, SpecsMap>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SpecsMap(HashMap<String, f64>);
