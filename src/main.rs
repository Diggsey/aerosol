#![recursion_limit="512"]

extern crate aerosol;
#[macro_use]
extern crate tt_call;

#[macro_export]
macro_rules! tt_debug2 {
    {
        $(
            $output:ident = [{ $($tokens:tt)* }]
        )*
    } => {
        $(
            println!("{}",
                concat!(
                    stringify!($output),
                    " = [{ ",
                    stringify!($($tokens)*),
                    " }]",
                )
            );
        )*
    }
}

aerosol::define_interface!(
    TestInterface {
        fn test_get(&self) -> Vec<u8>;
    }
);

#[allow(dead_code)]
struct FooFactory;
#[derive(Clone, Debug)]
struct Foo;
#[derive(Clone, Debug)]
struct Bar;

impl aerosol::Factory<(Bar,)> for FooFactory {
    type Object = Foo;
    fn build(_: (Bar,)) -> Result<Foo, anyhow::Error> { Ok(Foo) }
}

aerosol::define_context!(
    TestContext {
        foo: Foo [(bar) FooFactory],
        bar: Bar
    }
);

fn main() {

    //trace_macros!(true);
    //aerosol::test_macro!();
    tt_call! {
        macro = [{ aerosol::private_define_interface }]
        input = [{ TestInterface {
            fn test_get(&self) -> Vec<u8>;
        } }]
        ~~> tt_debug2
    }
    tt_call! {
        macro = [{ aerosol::private_define_context }]
        input = [{ TestContext {
            db: MyDatabase [PostgresFactory<MyDatabase>],
            pusher: PusherClient
        } }]
        ~~> tt_debug2
    }
}
