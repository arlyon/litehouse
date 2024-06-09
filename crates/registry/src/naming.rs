use std::sync::Arc;

pub trait NamingScheme {
    fn name(&self, id: usize) -> String;
    fn index(&self) -> String;
}

pub struct NumericalPrefixed {
    prefix: String,
}

impl NumericalPrefixed {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }
}

impl NamingScheme for NumericalPrefixed {
    fn name(&self, id: usize) -> String {
        format!("{}/{}.bin", self.prefix, id)
    }
    fn index(&self) -> String {
        format!("{}/{}", self.prefix, "index.bin")
    }
}

impl<N: NamingScheme> NamingScheme for Arc<N> {
    fn name(&self, id: usize) -> String {
        self.as_ref().name(id)
    }
    fn index(&self) -> String {
        self.as_ref().index()
    }
}

#[cfg(test)]
mod test {
    use crate::naming::{NamingScheme, NumericalPrefixed};

    #[test]
    fn numerical_prefixed() {
        let scheme = NumericalPrefixed::new("test");
        assert_eq!(scheme.name(0), "test/0.bin");
        assert_eq!(scheme.index(), "test/index.bin");
    }
}
