pub mod types;
pub mod codec;
pub mod session;
pub mod crypto;
pub mod stream;
pub mod transfer;
pub mod replay_protection;
pub mod compression;
pub mod fec;
pub mod qos;
pub mod serialization;

#[cfg(test)]
mod session_test;
#[cfg(test)]
mod crypto_test;



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
