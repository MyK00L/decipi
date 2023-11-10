use wasi_common::pipe::*;
use wasi_common::WasiCtx;
use wasmtime::*;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    memory: u32,
    cpu: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TestEval {
    Score(f64),
    TLE,
    RTE,
}

pub fn run_gen(module: Module, engine: Engine, test_id: u64) -> String {
    let stdout = WritePipe::new_in_memory();
    let mut ctx = deterministic_wasi_ctx::build_wasi_ctx();
    ctx.set_stdout(Box::new(stdout.clone()));
    ctx.push_arg(&test_id.to_string()).unwrap();
    run_wasi(&module, &engine, ctx, None, StoreLimits::default());
    let contents: Vec<u8> = stdout.try_into_inner().unwrap().into_inner();
    String::from_utf8(contents).unwrap()
}

pub fn run_sub(module: Module, engine: Engine, input: String, limits: Limits) -> String {
    let stdin = ReadPipe::from(input.as_bytes());
    let stdout = WritePipe::new_in_memory();
    let ctx = deterministic_wasi_ctx::build_wasi_ctx();
    ctx.set_stdin(Box::new(stdin.clone()));
    ctx.set_stdout(Box::new(stdout.clone()));
    let store_limits = StoreLimitsBuilder::new().trap_on_grow_failure(true).instances(1).memories(1).memory_size(limits.memory as usize).tables(1).table_elements(limits.memory>>4).build();
    run_wasi(&module, &engine, ctx, Some(limits.cpu), store_limits);
    let contents: Vec<u8> = stdout.try_into_inner().unwrap().into_inner();
    String::from_utf8(contents).unwrap()
}

pub fn run_eval(module: Module, engine: Engine, test_id: u64, input: String) -> String {
    let stdin = ReadPipe::from(input.as_bytes());
    let stdout = WritePipe::new_in_memory();
    let mut ctx = deterministic_wasi_ctx::build_wasi_ctx();
    ctx.set_stdin(Box::new(stdin.clone()));
    ctx.set_stdout(Box::new(stdout.clone()));
    ctx.push_arg(&test_id.to_string()).unwrap();
    run_wasi(&module, &engine, ctx, None, StoreLimits::default());
    let contents: Vec<u8> = stdout.try_into_inner().unwrap().into_inner();
    String::from_utf8(contents).unwrap()
}


pub fn evaluate_on_test(
    gen_wasm: Module,
    sub_wasm: Module,
    eval_wasm: Module,
    contest_engine: Engine,
    submission_engine: Engine,
    limits: Limits,
    test_id: u64,
) -> TestEval {
    eprintln!("test {}", test_id);
    let tc = run_gen(gen_wasm,contest_engine.clone(),test_id);
    let out = run_sub(sub_wasm,submission_engine,tc,limits);
    let score = f64::from_str(run_eval(eval_wasm,contest_engine,test_id,out).trim()).unwrap();
    eprintln!("score: {}",score);
    TestEval::Score(score)
}

fn evaluate_on_testset(
    gen_wasm: Module,
    sub_wasm: Module,
    eval_wasm: Module,
    contest_engine: Engine,
    submission_engine: Engine,
    limits: Limits,
    testset_length: u64,
) -> Vec<TestEval> {
    (0..testset_length).map(|x| evaluate_on_test(gen_wasm.clone(),sub_wasm.clone(),eval_wasm.clone(),contest_engine.clone(),submission_engine.clone(),limits,x)).collect()
}

struct State {
    limits: StoreLimits,
    wasi: WasiCtx,
}

fn run_wasi(module: &Module, engine: &Engine, wasi: WasiCtx, fuel: Option<u64>, limits: StoreLimits) {
    let mut linker: Linker<State> = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |state| &mut state.wasi).unwrap();
    
    let mut store = Store::new(&engine, State{limits,wasi});
    store.limiter(|state| &mut state.limits);
    if let Some(f) = fuel {
        store.add_fuel(f).unwrap();
    }
    linker.module(&mut store, "", &module).unwrap();
    linker
        .get_default(&mut store, "")
        .unwrap()
        .typed::<(), ()>(&store)
        .unwrap()
        .call(&mut store, ())
        .unwrap();
    eprintln!("{:?}", store.fuel_consumed());
}

fn get_submission_engine() -> Engine {
    let mut config = Config::new();
    config.consume_fuel(true);
    Engine::new(&config).unwrap()
}
fn get_contest_engine() -> Engine {
    let config = Config::new();
    Engine::new(&config).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_sub() {
        let submission_engine = get_submission_engine();
        let contest_engine = get_contest_engine();
        let gen_module = Module::from_file(&contest_engine, "./test_wasm/gen.wasm").unwrap();
        let eval_module = Module::from_file(&contest_engine, "./test_wasm/eval.wasm").unwrap();
        let sub_module = Module::from_file(&submission_engine, "./test_wasm/sub_ac.wasm").unwrap();
        let limits = Limits{memory:1234124,cpu:1231241412};

        let ans = evaluate_on_testset(gen_module,sub_module,eval_module,contest_engine,submission_engine,limits,16);
        assert_eq!(ans,vec![TestEval::Score(1f64);16]);

        eprintln!("{:?}",ans);
    }
}
