use std::env;
use std::hash::{Hasher,SipHasher};
use std::io::stdin;

#[derive(Default)]
struct Scanner {
    buffer: Vec<String>
}
impl Scanner {
    fn next<T: std::str::FromStr>(&mut self) -> T {
        loop {
            if let Some(token) = self.buffer.pop() {
                return token.parse().ok().expect("Failed parse");
            }
            let mut input = String::new();
            stdin().read_line(&mut input).expect("Failed read");
            self.buffer = input.split_whitespace().rev().map(String::from).collect();
        }
    }
}

fn main() {
    let args: Vec<String>  = env::args().collect();
    assert!(args.len()==2);
    let test_id = args[1].parse::<u64>().unwrap();
    let mut hasher = SipHasher::new_with_keys(42,69);
    hasher.write_u64(test_id);
    let n = hasher.finish();

    let mut scan = Scanner::default();
    let ans = scan.next::<u64>();
    if ans == n^42 {
        println!("1");
    } else {
        println!("0");
    }
}

