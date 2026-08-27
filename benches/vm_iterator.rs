use criterion::{
    criterion_group,
    criterion_main,
    Criterion,
};

use novum::{
    syntax::Lexer,
    syntax::Parser,
    vm::{
        Compiler,
        Vm,
    },
};

use std::hint::black_box;

fn run_vm(
    source: &str,
) {
    let tokens =
        Lexer::new(source)
            .lex()
            .expect("lex failed");

    let mut parser =
        Parser::new(tokens);

    let program =
        parser
            .parse()
            .expect("parse failed");

    let chunk =
        Compiler::new()
            .compile(&program)
            .expect("compile failed");

    let mut vm =
        Vm::new();

    vm.run(
        std::rc::Rc::new(chunk)
    )
    .expect("VM failed");
}

fn benchmark_iterator(
    c: &mut Criterion,
) {
    c.bench_function(
        "map_filter_collect",
        |b| {
            b.iter(|| {
                run_vm(
                    black_box(
                        r#"
                        (0..100000)
                            .map(|x| x * 2)
                            .filter(|x| x % 3 == 0)
                            .collect()
                        "#
                    )
                )
            })
        }
    );
}

criterion_group!(
    benches,
    benchmark_iterator
);

criterion_main!(
    benches
);