use std::env;
use std::hash::{Hasher,SipHasher};

fn main() {
    let args: Vec<String>  = env::args().collect();
    assert!(args.len()==2);
    let test_id = args[1].parse::<u64>().unwrap();
    let mut hasher = SipHasher::new_with_keys(42,69);
    hasher.write_u64(test_id);
    let n = hasher.finish();
    println!("{}", n)
}

