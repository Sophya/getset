mod simple;

fn main() {
    let foo = simple::Foo::<u8>::default();

    let _opt = foo.public_option();
    simple::main();
}
