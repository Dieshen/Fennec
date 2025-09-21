pub mod command;
pub mod config;
pub mod error;
pub mod provider;
pub mod session;
pub mod transcript;

pub use error::{FennecError, Result};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
