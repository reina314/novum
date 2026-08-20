# Novum v0.3.0

### For quick ideas, quick experiments, and quick results.

Novum is a lightweight programming language for small tasks, quick experiments, and everyday research work. Rather than aiming to be a large, feature-heavy language, Novum focuses on being simple, flexible, and easy to pick up. Its concise syntax and built-in statistical tools make it a convenient companion when you need to test an idea, manipulate some data, or run a quick analysis. This is a structural rewrite of the original Parvum interpreter.

## Sample Code
```py
struct Point {
    x,
    y,

    move: |self, dx, dy| {
        self.x = self.x + dx;
        self.y = self.y + dy;
    }
}

let A = matrix([
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9]
]);

let transform = |x| x * 2;

let B = A[0..2, 1..3] * 2;

let p = Point(
    transform(B[0, 0]),
    transform(B[1, 1])
);

p.move(1, 1);

welch_t(
    [p.x, p.y, 5, 6, 7],
    [8, 9, 10, 11, 12]
)
```