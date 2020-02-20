use std::env;
fn main() {
    for (var, val) in env::vars() {
        println!("{} = {}", var, val);
    }
}