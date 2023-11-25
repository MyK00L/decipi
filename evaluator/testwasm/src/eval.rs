use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::env;
use std::io::stdin;

#[derive(Default)]
struct Scanner {
    buffer: Vec<String>,
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
    let args: Vec<String> = env::args().collect();
    assert!(args.len() == 1);
    let test_id = args[0].parse::<u64>().unwrap();
    let mut seed = [42u8; 32];
    for i in seed.iter_mut().zip(test_id.to_be_bytes().iter().cycle()) {
        *i.0 ^= i.1;
    }
    let mut rng = ChaCha8Rng::from_seed(seed);
    let n: u64 = rng.gen();

    let mut scan = Scanner::default();
    let ans = scan.next::<u64>();
    if ans == n ^ 42 {
        println!("1");
    } else {
        println!("0");
    }
}
