// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use mina_serialization_versioned::Versioned;

    #[test]
    fn test_versioned() {
        type I32V1 = Versioned<i32, 2>;

        let i = I32V1::default();
        assert_eq!(i.version(), 2);
        assert_eq!(i.inner(), i32::default());
    }
}
