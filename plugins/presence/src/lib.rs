//! Presence plugin for the Litehouse home automation system.
//!
//! This crate provides functionality for presence detection within the Litehouse system,
//! allowing for automation based on the presence or absence of individuals or devices.

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
