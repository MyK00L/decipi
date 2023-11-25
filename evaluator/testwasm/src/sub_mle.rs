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
    let mut scan = Scanner::default();
    let n = scan.next::<u64>();
    let dim: usize = (10000 + (n & 7)) as usize;
    let mut v = vec![vec![]; dim];
    for i in v.iter_mut() {
        i.resize(dim, 1);
    }
    for i in 0usize..dim {
        v[i][0] = i as u64 ^ n;
        v[0][i] = !v[i][0];
    }
    for i in 1usize..dim {
        for j in 1usize..dim {
            v[i][j] = v[i - 1][j - 1]
                .overflowing_mul(v[i - 1][j].overflowing_add(v[i][j - 1]).0)
                .0;
        }
    }
    println!("{}", v[dim - 1][dim - 1] ^ v[42][42]);
}
