extern crate failure;
#[macro_use]
extern crate failure_derive;

use std::io::BufRead;
use failure::{err_msg, Error}; // Any type that derives Fail can be cast into Error

#[derive(Debug, Fail)]
#[fail(display = "Utf8 error at index `{}`", index)]
pub struct Utf8Error {
    index: usize,
}


fn test() -> Result<(), Error> {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.chars().all(|c| c.is_whitespace()) {
            break;
        }
        if !line.starts_with("$") {
            return Err(err_msg("Didnt start with `$`"));
        }
        println!("{}", &line[1..]);
    }
    Ok(())
}

fn main() {
    println!("Hello, world!");
}
