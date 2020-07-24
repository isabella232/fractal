pub mod identity;
pub mod r0;

pub use matrix_sdk::*;
pub use url;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
