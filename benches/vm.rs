use criterion::{
    criterion_group,
    criterion_main,
    Criterion,
};

use novum::{
    syntax::Lexer,
    syntax::Parser,
    vm::{
        Chunk,
        Compiler,
        Vm,
    },
};

use std::{
    hint::black_box,
    rc::Rc,
};

fn compile(
    source: &str,
) -> Rc<Chunk> {
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

    Rc::new(
        Compiler::new()
            .compile(&program)
            .expect("compile failed")
    )
}

fn benchmark_iterator_map(
    c: &mut Criterion,
) {
    let chunk =
        compile(
            r#"
            let xs =
                (0..1_000_000)
                    .map(|x| x * 2)
                    .collect()
            "#
        );

    let mut vm =
        Vm::new();

    c.bench_function(
        "map_collect",
        |b| {
            b.iter(|| {
                vm.run(
                    chunk.clone()
                )
                .expect("VM failed");
            })
        }
    );
}

fn benchmark_iterator(
    c: &mut Criterion,
) {
    let chunk =
        compile(
            r#"
            let xs =
                (0..1_000_000)
                    .map(|x| x * 2)
                    .filter(|x| x % 3 == 0)
                    .collect()
            "#
        );

    let mut vm =
        Vm::new();

    c.bench_function(
        "map_filter_collect",
        |b| {
            b.iter(|| {
                vm.run(
                    black_box(
                        chunk.clone()
                    )
                )
                .expect("VM failed");
            })
        }
    );
}

fn benchmark_for(
    c: &mut Criterion,
) {
    let chunk =
        compile(
            r#"
            let x = 0

            for i in 0..1_000_000 {
                x = x + i
            }

            x
            "#
        );

    let mut vm =
        Vm::new();

    c.bench_function(
        "for_loop",
        |b| {
            b.iter(|| {
                vm.run(
                    black_box(
                        chunk.clone()
                    )
                )
                .expect("VM failed");
            })
        }
    );
}

criterion_group!(
    benches,
    benchmark_iterator_map,
    benchmark_iterator,
    benchmark_for,
);

criterion_main!(
    benches
);