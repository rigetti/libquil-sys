#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use std::fs::read_to_string;
    use std::path::PathBuf;

    use crate::chip::ChipSpec;

    #[rstest]
    fn test_chip_parity(#[files("test-data/*.qpu")] path: PathBuf) {
        let original_json_str = read_to_string(&path)
            .unwrap_or_else(|_| panic!("Should be able to load file: {:?}", path));
        let original_json_value: serde_json::Value =
            serde_json::from_str(&original_json_str).unwrap();

        let chip: ChipSpec = serde_json::from_str(&original_json_str).unwrap();
        let chip_str = serde_json::to_string(&chip).unwrap();
        let chip_value: serde_json::Value = serde_json::from_str(&chip_str).unwrap();

        assert_eq!(chip_value, original_json_value);
    }
}
