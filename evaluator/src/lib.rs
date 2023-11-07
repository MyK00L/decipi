use std::io::Read;
use wasmer::{Module};
use wasmer_wasix::{Pipe, WasiEnv};

pub struct Limits {
    memory: u64,
    cpu: u64,
}

pub enum TestEval {
    AC,
    WA,
    TLE,
    RTE,
}

pub fn evaluate_on_test(
    gen_wasm: Module,
    sub_wasm: Module,
    eval_wasm: Module,
    test_id: u64,
) -> TestEval {
    let (input_sender, input_reader) = Pipe::channel();
    let (output_sender, output_reader) = Pipe::channel();
    let (eval_sender, mut eval_reader) = Pipe::channel();

    WasiEnv::builder("gen")
        .arg(test_id.to_string())
        .stdout(Box::new(input_sender))
        .run(gen_wasm)
        .unwrap();

    WasiEnv::builder("sub")
        .stdin(Box::new(input_reader))
        .stdout(Box::new(output_sender))
        .run(sub_wasm)
        .unwrap();

    WasiEnv::builder("eval_wasm")
        .arg(test_id.to_string())
        .stdin(Box::new(output_reader))
        .stdout(Box::new(eval_sender))
        .run(eval_wasm)
        .unwrap();

    let mut buf = String::new();
    eval_reader.read_to_string(&mut buf).unwrap();
    eprintln!("eval: {}", buf);

    TestEval::AC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let engine = wasmer::Engine::default();
        let gen = Module::from_file(&engine, "test_wasm/gen.wasm").unwrap();
        let sub = Module::from_file(&engine, "test_wasm/sub.wasm").unwrap();
        let eval = Module::from_file(&engine, "test_wasm/eval.wasm").unwrap();
        evaluate_on_test(gen,sub,eval,42);
    }
}
