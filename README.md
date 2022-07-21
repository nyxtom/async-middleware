# async-middleware

[![latest version](https://img.shields.io/crates/v/async-middleware.svg)](https://crates.io/crates/async-middleware)
[![documentation](https://docs.rs/async-middleware/badge.svg)](https://docs.rs/async-middleware)
![license](https://img.shields.io/crates/l/async-middleware.svg)


Provides a way to pipe a number of async middleware functions together that is type safe between each of the input -> output. This allows you to combine A -> B -> C similar to a monad but not quite as formal. Although, you could in theory use this as a monad library but it isn't really setup to handle identity, maps, unwraps, fors (a more formal and idiomatic approach https://medium.com/swlh/monad-interface-rust-edition-bd6486b93607) would be better but there are some things missing like async closures that would make this easier. Things like wrapping an existing async function is very difficult due to the lack of async closures in rust. In any case, this provides a simple middleware chaining.

## Examples

```rust
// import * as this provides all the trait implementations by default
use async_middleware::*;

async fn producer() -> i32 {
    3
}

async fn multipler(i: i32) -> i32 {
    i * 32
}

async fn stringer(i: i32) -> String {
    i.to_string()
}

async fn logger(s: String) {
    println!("{}", s);
}

async fn log_nums(i: i32) {
    println!("{}", i);
}

#[test]
fn test_piper_tuple() {
    pipe((producer, log_nums));
    pipe((producer, stringer, logger));
    pipe((producer, multipler, stringer, logger));
    pipe((multipler, multipler, multipler));
    pipe((multipler, multipler, stringer));

    // alternative syntax
    (producer, log_nums).pipe();
    (producer, stringer, logger).pipe();
    (producer, multipler, stringer, logger).pipe();
    (multipler, multipler, multipler).pipe();
    (multipler, multipler, stringer).pipe();

    // pipe different pipes
    let m = (producer, multipler).pipe(); // 3 * 32 = 96
    let m = (m, multipler).pipe(); // * 32 = 3072
    let m = pipe((m, stringer)); // 3072

    assert_eq!(String::from("3072"), m.call(()).await);
}

#[async_std::test]
async fn test_piper_tuple_inputs() {
    let m = (multipler, multipler, stringer).pipe();
    assert_eq!(String::from("1024"), m.call(1).await);
    assert_eq!(String::from("2048"), m.call(2).await);
    assert_eq!(String::from("3072"), m.call(3).await);
}

#[test]
fn test_convert_transform() {
    convert(multipler, stringer);
    convert(multipler, multipler);
}

#[test]
fn test_source_transform() {
    convert(producer, multipler);
}

#[test]
fn test_source_sink() {
    convert(producer, log_nums);
}

#[test]
fn test_transform() {
    convert(convert(producer, multipler), stringer);
}

#[test]
fn test_transform_source_transform_sink() {
    convert(convert(convert(producer, multipler), stringer), logger);
}
```
