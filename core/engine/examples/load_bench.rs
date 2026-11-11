//! Measures the load-time cost of real scripts: context creation, parsing and
//! bytecode compilation, each timed separately over several rounds.
//!
//!     cargo run --release -p boa_engine --example load_bench -- [--rounds N] <file.js>...
//!
//! Scripts are not executed, because page scripts need a host. A file that
//! does not parse as a script is parsed as a module (parse time only).

use boa_engine::{Context, Module, Script, Source};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn main() {
    let mut rounds = 15usize;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--rounds" {
            rounds = args.next().and_then(|n| n.parse().ok()).expect("--rounds needs a number");
        } else {
            files.push(a);
        }
    }
    let sources: Vec<(String, Vec<u8>)> =
        files.iter().map(|f| (f.clone(), std::fs::read(f).expect("readable file"))).collect();
    let bytes: usize = sources.iter().map(|s| s.1.len()).sum();

    let (mut t_ctx, mut t_parse, mut t_compile) = (Vec::new(), Vec::new(), Vec::new());
    let (mut scripts, mut modules, mut failed) = (0, 0, 0);
    for round in 0..rounds {
        let t = Instant::now();
        let mut ctx = Context::default();
        t_ctx.push(t.elapsed());
        let (mut parse, mut compile) = (Duration::ZERO, Duration::ZERO);
        for (name, code) in &sources {
            let t = Instant::now();
            let script = Script::parse(Source::from_bytes(code), None, &mut ctx);
            let d = t.elapsed();
            match script {
                Ok(script) => {
                    parse += d;
                    let t = Instant::now();
                    let block = script.codeblock(&mut ctx);
                    compile += t.elapsed();
                    if round == 0 {
                        scripts += 1;
                        if let Err(e) = &block {
                            eprintln!("compile error in {name}: {e}");
                        }
                    }
                    black_box(block).ok();
                }
                Err(_) => {
                    let t = Instant::now();
                    let module = Module::parse(Source::from_bytes(code), None, &mut ctx);
                    parse += t.elapsed();
                    if round == 0 {
                        match &module {
                            Ok(_) => modules += 1,
                            Err(e) => {
                                failed += 1;
                                eprintln!("parse error in {name}: {e}");
                            }
                        }
                    }
                    black_box(module).ok();
                }
            }
        }
        t_parse.push(parse);
        t_compile.push(compile);
    }
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let min = |v: &Vec<Duration>| *v.iter().min().expect("at least one round");
    println!("{} files, {bytes} bytes: {scripts} scripts, {modules} modules, {failed} failed; {rounds} rounds", sources.len());
    println!("{:<10}{:>12}{:>12}", "stage", "median ms", "min ms");
    for (name, v) in [("context", &t_ctx), ("parse", &t_parse), ("compile", &t_compile)] {
        println!("{name:<10}{:>12.3}{:>12.3}", ms(median(v.clone())), ms(min(v)));
    }
}
