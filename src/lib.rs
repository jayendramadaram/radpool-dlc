#[allow(dead_code)]
fn main() {
    println!("Hello, Radpool!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        main();
    }
}
