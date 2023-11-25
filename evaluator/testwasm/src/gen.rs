use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::env;

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

    println!("{}", n)
}
