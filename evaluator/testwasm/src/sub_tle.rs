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
    let mut scan = Scanner::default();
    let n = scan.next::<u64>();
    let dim = n.max(100000000);
    let mut ans = n;
    for i in 0..dim {
        ans = ans.overflowing_add(ans.overflowing_mul(i^n).0).0;
    }
    println!("{}",ans);
}

