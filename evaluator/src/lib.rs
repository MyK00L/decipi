use blake3::Hasher;
use num_traits::identities::Zero;
use ordered_float::NotNan;
use std::str::FromStr;
use wasi_common::pipe::*;
use wasi_common::WasiCtx;
use wasmtime::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    memory: u32,
    cpu: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TestEval {
    Score(NotNan<f64>),
    TLE,
    MLE,
    RTE,
}
#[derive(Clone, Debug, PartialEq)]
pub enum SubRes {
    OK(String),
    TLE,
    MLE,
    RTE,
    MFO,
}

fn run_gen(
    module: Module,
    engine: Engine,
    test_id: u32,
    hasher: &mut Hasher,
) -> anyhow::Result<String> {
    let stdout = WritePipe::new_in_memory();
    let mut ctx = deterministic_wasi_ctx::build_wasi_ctx();
    ctx.set_stdout(Box::new(stdout.clone()));
    ctx.push_arg(&test_id.to_string())?;
    run_wasi(&module, &engine, ctx, None, StoreLimits::default(), hasher)??;
    let contents: Vec<u8> = stdout
        .try_into_inner()
        .map_err(|e| anyhow::anyhow!("error getting contents of stdout pipe: {:?}", e))?
        .into_inner();
    Ok(String::from_utf8(contents)?)
}

fn run_sub(
    module: Module,
    engine: Engine,
    input: String,
    limits: Limits,
    hasher: &mut Hasher,
) -> anyhow::Result<SubRes> {
    let stdin = ReadPipe::from(input.as_bytes());
    let stdout = WritePipe::new_in_memory();
    let ctx = deterministic_wasi_ctx::build_wasi_ctx();
    ctx.set_stdin(Box::new(stdin.clone()));
    ctx.set_stdout(Box::new(stdout.clone()));
    let store_limits = StoreLimitsBuilder::new()
        .trap_on_grow_failure(true)
        .instances(1)
        .memories(1)
        .memory_size(limits.memory as usize)
        .tables(1)
        .table_elements(limits.memory >> 4)
        .build();
    let result = run_wasi(
        &module,
        &engine,
        ctx,
        Some(limits.cpu),
        store_limits,
        hasher,
    )?;
    match result {
        Ok(()) => {
            if let Ok(inner) = stdout.try_into_inner() {
                let contents: Vec<u8> = inner.into_inner();
                Ok(SubRes::OK(String::from_utf8(contents).unwrap()))
            } else {
                Ok(SubRes::MFO) //TODO
            }
        }
        Err(e) => {
            if let Some(&t) = e.root_cause().downcast_ref::<Trap>() {
                match t {
                    Trap::OutOfFuel => Ok(SubRes::TLE),
                    Trap::MemoryOutOfBounds => Ok(SubRes::MLE),
                    Trap::TableOutOfBounds => Ok(SubRes::MLE),
                    _ => Ok(SubRes::RTE),
                }
            } else {
                // TODO: better solution
                let t = e.root_cause().to_string();
                if t.contains("forcing trap when growing memory") {
                    Ok(SubRes::MLE)
                } else {
                    Err(e)
                }
            }
        }
    }
}

fn run_eval(
    module: Module,
    engine: Engine,
    test_id: u32,
    input: String,
    hasher: &mut Hasher,
) -> anyhow::Result<String> {
    let stdin = ReadPipe::from(input.as_bytes());
    let stdout = WritePipe::new_in_memory();
    let mut ctx = deterministic_wasi_ctx::build_wasi_ctx();
    ctx.set_stdin(Box::new(stdin.clone()));
    ctx.set_stdout(Box::new(stdout.clone()));
    ctx.push_arg(&test_id.to_string())?;
    run_wasi(&module, &engine, ctx, None, StoreLimits::default(), hasher)??;
    let contents: Vec<u8> = stdout
        .try_into_inner()
        .map_err(|e| anyhow::anyhow!("error getting contents of stdout pipe: {:?}", e))?
        .into_inner();
    Ok(String::from_utf8(contents)?)
}

#[allow(clippy::too_many_arguments)]
fn evaluate_on_test(
    gen_wasm: Module,
    sub_wasm: Module,
    eval_wasm: Module,
    contest_engine: Engine,
    submission_engine: Engine,
    limits: Limits,
    test_id: u32,
    hasher: &mut Hasher,
) -> anyhow::Result<TestEval> {
    let tc = run_gen(gen_wasm, contest_engine.clone(), test_id, hasher)?;
    let sub_res = run_sub(sub_wasm, submission_engine, tc, limits, hasher)?;
    Ok(match sub_res {
        SubRes::OK(out) => {
            let score = NotNan::<f64>::from_str(
                run_eval(eval_wasm, contest_engine, test_id, out, hasher)?.trim(),
            )?;
            TestEval::Score(score)
        }
        SubRes::TLE => TestEval::TLE,
        SubRes::MLE => TestEval::MLE,
        SubRes::RTE => TestEval::RTE,
        SubRes::MFO => TestEval::Score(NotNan::zero()),
    })
}

#[allow(clippy::too_many_arguments)]
fn evaluate_on_testset(
    gen_wasm: Module,
    sub_wasm: Module,
    eval_wasm: Module,
    contest_engine: Engine,
    submission_engine: Engine,
    limits: Limits,
    testset_length: u32,
    hasher: &mut Hasher,
) -> anyhow::Result<Vec<TestEval>> {
    (0..testset_length)
        .map(|x| {
            evaluate_on_test(
                gen_wasm.clone(),
                sub_wasm.clone(),
                eval_wasm.clone(),
                contest_engine.clone(),
                submission_engine.clone(),
                limits,
                x,
                hasher,
            )
        })
        .collect()
}

pub fn evaluate_submission(
    gen: &[u8],
    eval: &[u8],
    sub: &[u8],
    max_memory: u32,
    max_cpu: u64,
    testset_length: u32,
) -> anyhow::Result<(NotNan<f64>, blake3::Hash)> {
    let submission_engine = get_submission_engine()?;
    let contest_engine = get_contest_engine()?;
    let gen_module = Module::from_binary(&contest_engine, gen)?;
    let eval_module = Module::from_binary(&contest_engine, eval)?;
    let sub_module = Module::from_binary(&submission_engine, sub)?;
    let limits = Limits {
        memory: max_memory,
        cpu: max_cpu,
    };
    let mut hasher = Hasher::new();
    let ev = evaluate_on_testset(
        gen_module,
        sub_module,
        eval_module,
        contest_engine,
        submission_engine,
        limits,
        testset_length,
        &mut hasher,
    )?;
    Ok((
        ev.into_iter()
            .map(|x| match x {
                TestEval::Score(s) => s,
                _ => NotNan::zero(),
            })
            .max()
            .ok_or(anyhow::anyhow!("max err"))?,
        hasher.finalize(),
    ))
}

fn run_wasi(
    module: &Module,
    engine: &Engine,
    wasi: WasiCtx,
    fuel: Option<u64>,
    limits: StoreLimits,
    hasher: &mut Hasher,
) -> anyhow::Result<anyhow::Result<()>> {
    struct State {
        limits: StoreLimits,
        wasi: WasiCtx,
    }
    let mut linker: Linker<State> = Linker::new(engine);
    wasmtime_wasi::add_to_linker(&mut linker, |state| &mut state.wasi)?;

    let mut store = Store::new(engine, State { limits, wasi });
    store.limiter(|state| &mut state.limits);
    if let Some(f) = fuel {
        store.add_fuel(f)?;
    }

    // make an instance and run the wasi program
    let instance = linker.instantiate(&mut store, module)?; //TODO: check the start function here consumes fuel/is not exploitable
    let result = instance
        .get_typed_func::<(), ()>(&mut store, "_start")?
        .call(&mut store, ());

    // get the execution data
    let mut _memory_used = 0;
    let fuel_used = store.fuel_consumed().unwrap_or_default();
    //TODO: is the memory always called memory?
    if let Some(memory) = instance.get_memory(&mut store, "memory") {
        hasher.update(memory.data(&store));
        _memory_used = memory.size(&store);
    }
    if fuel.is_some() {
        hasher.update(&fuel_used.to_be_bytes());
    }

    Ok(result)
}

fn get_submission_engine() -> anyhow::Result<Engine> {
    let mut config = Config::new();
    unsafe {
        config.cranelift_flag_enable("enable_nan_canonicalization");
    }
    config.consume_fuel(true);
    Engine::new(&config)
}
fn get_contest_engine() -> anyhow::Result<Engine> {
    let mut config = Config::new();
    unsafe {
        config.cranelift_flag_enable("enable_nan_canonicalization");
    }
    Engine::new(&config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::identities::One;

    fn eval_sub(sub_file: &str) -> anyhow::Result<Vec<TestEval>> {
        let submission_engine = get_submission_engine()?;
        let contest_engine = get_contest_engine()?;
        let gen_module = Module::from_file(
            &contest_engine,
            "./testwasm/target/wasm32-wasi/debug/gen.wasm",
        )?;
        let eval_module = Module::from_file(
            &contest_engine,
            "./testwasm/target/wasm32-wasi/debug/eval.wasm",
        )?;
        let sub_module = Module::from_file(&submission_engine, sub_file)?;
        let limits = Limits {
            memory: 2000000,
            cpu: 100000,
        };
        let mut hasher = Hasher::new();
        let ev = evaluate_on_testset(
            gen_module,
            sub_module,
            eval_module,
            contest_engine,
            submission_engine,
            limits,
            16,
            &mut hasher,
        );
        eprintln!("{:?}", hasher.finalize());
        ev
    }

    #[test]
    fn ac_sub() {
        let ans = eval_sub("./testwasm/target/wasm32-wasi/debug/sub_ac.wasm").unwrap();
        assert_eq!(vec![TestEval::Score(NotNan::one()); 16], ans);
    }
    #[test]
    fn wa_sub() {
        let ans = eval_sub("./testwasm/target/wasm32-wasi/debug/sub_wa.wasm").unwrap();
        assert_eq!(vec![TestEval::Score(NotNan::zero()); 16], ans);
    }
    #[test]
    fn rte_sub() {
        let ans = eval_sub("./testwasm/target/wasm32-wasi/debug/sub_rte.wasm").unwrap();
        assert_eq!(vec![TestEval::RTE; 16], ans);
    }
    #[test]
    fn tle_sub() {
        let ans = eval_sub("./testwasm/target/wasm32-wasi/debug/sub_tle.wasm").unwrap();
        assert_eq!(vec![TestEval::TLE; 16], ans);
    }
    #[test]
    fn mle_sub() {
        let ans = eval_sub("./testwasm/target/wasm32-wasi/debug/sub_mle.wasm").unwrap();
        assert_eq!(vec![TestEval::MLE; 16], ans);
    }
    #[test]
    fn attack_sub() {
        let ans = eval_sub("./testwasm/target/wasm32-wasi/debug/sub_attack.wasm").unwrap();
        assert_eq!(vec![TestEval::RTE; 16], ans);
    }
}
