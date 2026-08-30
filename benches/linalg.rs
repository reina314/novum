use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};

use novum::runtime::Matrix;

fn bench_matrix_mul(
    c: &mut Criterion,
) {
    for size in [
        32usize,
        64,
        128,
        256,
        512,
        1024,
    ] {
        let a =
            Matrix::from_rows(
                (0..size)
                    .map(|row| {
                        (0..size)
                            .map(|col| {
                                ((row + col) % 100)
                                    as f64
                            })
                            .collect()
                    })
                    .collect()
            )
            .unwrap();

        let b =
            Matrix::from_rows(
                (0..size)
                    .map(|row| {
                        (0..size)
                            .map(|col| {
                                ((row * 2 + col) % 100)
                                    as f64
                            })
                            .collect()
                    })
                    .collect()
            )
            .unwrap();

        c.bench_function(
            &format!(
                "matrix_mul_{}",
                size
            ),
            |bench| {
                bench.iter(|| {
                    black_box(
                        a.matmul(
                            &b
                        )
                        .unwrap()
                    )
                })
            },
        );
    }
}

criterion_group!(
    benches,
    bench_matrix_mul
);

criterion_main!(
    benches
);