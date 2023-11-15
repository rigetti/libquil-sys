## 0.2.2 (2023-11-15)

### Fixes

#### support all Quil types in multishot result (#56)

## 0.2.1 (2023-11-07)

### Fixes

#### take references to Python objects to prevent them being freed early (#54)

## 0.2.0 (2023-10-27)

### Breaking Changes

#### replaces Vec<u32> input with a new type MultishotAddressRequest

* update libquil version in tests

#### removes the n_qubits parameter

### Features

#### request all indices from multishot (#50)

#### wavefunction doesn't need info about number of qubits (#52)

## 0.1.5 (2023-10-16)

### Features

#### change panic to an error if cannot initialize libquil

## 0.1.4 (2023-10-16)

### Fixes

#### limit number of keywords in Cargo.toml

## 0.1.3 (2023-10-16)

### Fixes

#### add required keywords to Cargo.toml

## 0.1.2 (2023-10-16)

### Fixes

#### use --verbose in knope release

#### bump knope version

#### bump knope action to v2

## 0.1.1

### Features

- update libquil bindings
- update libquil support for QVM and quilc (#34)
- Add Python crate and improve error handling (#23)

### Fixes

- improve header and library lookup (#39)
- add Drop implementations and reclaim CString memory (#37)
