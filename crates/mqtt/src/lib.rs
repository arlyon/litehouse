//! MQTT plugin for the Litehouse home automation system.
//!
//! This crate provides MQTT client functionality, allowing Litehouse to communicate
//! with devices and services using the MQTT protocol.

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
