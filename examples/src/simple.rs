use getset::{CopyGetters, ExpectGetters, Getters, MutGetters, Setters};

#[derive(Getters, Setters, MutGetters, CopyGetters, ExpectGetters)]
pub struct Foo<T>
where
    T: Copy + Clone + Default,
{
    /// Doc comments are supported!
    /// Multiline, even.
    #[getset(get, set, get_mut)]
    private: T,

    /// Doc comments are supported!
    /// Multiline, even.
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub")]
    public: T,

    #[getset(get_expect = "pub")]
    public_option: Option<T>,
}
impl Default for Foo<u8> {
    fn default() -> Self {
        Self {
            private: 0,
            public: 0,
            public_option: Some(0),
        }
    }
}

pub fn main() {
    let mut foo = Foo::<u8>::default();
    foo.set_private(&mut 1);
    (*foo.private_mut()) += 1;
    assert_eq!(*foo.private(), 2);

    let _p = foo.public();

    let l = foo.public_option();
    println!("{}", l);
}
