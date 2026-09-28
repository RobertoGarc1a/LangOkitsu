use super::*;

#[test]
fn executes_hello_file() {
    assert_eq!(
        output(include_str!("../../examples/hello.oki")),
        "Hello World\n"
    );
}
