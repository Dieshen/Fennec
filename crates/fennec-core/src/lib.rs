pub mod config;
pub mod session;
pub mod transcript;
pub mod command;
pub mod provider;
pub mod error;

pub use error::{FennecError, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}