pub extern crate tt_call;
pub extern crate failure;

mod join;
mod parse;
mod interface;
mod context;


pub trait Provide<T> {
    fn provide(&self) -> T;
}

pub trait Factory {
    type Object;
    fn build() -> Result<Self::Object, failure::Error>;
}

pub trait ProvideWith<T>: Provide<T> + Sized {
    fn provide_with<E, F: FnOnce(T) -> Result<T, E>>(&self, f: F) -> Result<Self, E>;
}



#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
