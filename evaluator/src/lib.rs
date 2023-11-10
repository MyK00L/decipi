mod mem_tunable;
use std::sync::Arc;
use std::io::Read;
use wasmer::{Module, Store, EngineBuilder, CompilerConfig, Pages, Engine, BaseTunables, Target, NativeEngineExt};
use wasmer_wasix::{Pipe, WasiEnv};
use std::str::FromStr;
use wasmer_compiler_singlepass::Singlepass;
use wasmer_middlewares::Metering;
use mem_tunable::LimitingTunables;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Limits {
    memory: u32,
    cpu: u64,
}

#[derive(Clone,Copy,Debug,PartialEq)]
pub enum TestEval {
    Score(f64),
    TLE,
    RTE,
}

pub fn evaluate_on_test(
    gen_wasm: Module,
    sub_wasm: Module,
    eval_wasm: Module,
    limits: Limits,
    test_id: u64,
) -> TestEval {
    eprintln!("test {}",test_id);
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
    let score = f64::from_str(&buf.trim()).unwrap_or_default();

    TestEval::Score(score)
}

fn evaluate_on_testset(
    gen_wasm: Module,
    sub_wasm: Module,
    eval_wasm: Module,
    limits: Limits,
    testset_length: u64,
) -> Vec<TestEval> {
    (0..testset_length).map(|x| evaluate_on_test(gen_wasm.clone(),sub_wasm.clone(),eval_wasm.clone(),limits,x)).collect()
}



fn compile_submission_wasm(wasm_bytes: &[u8], limits: Limits) -> Module {
    // cpu-limit
    // TODO
    let cost_function = |operator: &wasmer::wasmparser::Operator| -> u64 {
        1
    };
    let mut compiler_config = Singlepass::new();
    compiler_config.canonicalize_nans(true);
    let metering = Arc::new(Metering::new(limits.cpu, cost_function));
    compiler_config.push_middleware(metering);
    
    // memory-limit
    let base = BaseTunables::for_target(&Target::default());
    let tunables = LimitingTunables::new(base, Pages(limits.memory));
    let mut engine: Engine = compiler_config.into();
    //engine.set_tunables(tunables);

    let mut store = Store::new(engine);
    //compile the module
    Module::new(&store, wasm_bytes).unwrap()
}

fn compile_contest_wasm(wasm_bytes: &[u8]) -> Module {
    let mut compiler_config = Singlepass::new();
    //compiler_config.canonicalize_nans(true);
    //let mut store = Store::new(EngineBuilder::new(compiler_config));
    let mut store = Store::default();

    //compile the module
    Module::new(&store, wasm_bytes).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_sub() {
        let limits = Limits{memory:1u32<<28, cpu: 1u64<<32};

        let gen_wasm = std::fs::read("test_wasm/gen.wasm").unwrap();
        let sub_wasm = std::fs::read("test_wasm/sub_ac.wasm").unwrap();
        let eval_wasm = std::fs::read("test_wasm/eval.wasm").unwrap();

        let gen = compile_contest_wasm(&gen_wasm);
        let eval = compile_contest_wasm(&eval_wasm);
        let sub = compile_submission_wasm(&sub_wasm, limits);

        assert_eq!(vec![TestEval::Score(1f64);16], evaluate_on_testset(gen,sub,eval,limits,16));
    }
}
