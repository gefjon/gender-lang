#![feature(try_trait)]

use std::{collections::HashMap, fmt};

pub mod fsize {
    //! for, i guess, portability reasons, or something, i've hacked
    //! together this interface for interacting with pointer-width
    //! floats, since they can be embedded in an object

    #[cfg(target_pointer_width = "64")]
    pub type Fsize = f64;

    #[cfg(target_pointer_width = "32")]
    pub type Fsize = f32;

    #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
    pub type Fsize = compile_error!(
        "what fucking platform are you on. pointer_width should be either 16 or 32."
    );

    pub fn to_usize(f: Fsize) -> usize {
        Fsize::to_bits(f) as usize
    }

    pub fn from_usize(u: usize) -> Fsize {
        Fsize::from_bits(u as _)
    }
}

pub mod err {
    use std::{fmt, option::NoneError};
    pub enum Error {
        None(NoneError),
    }

    impl fmt::Debug for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Error::None(_) => write!(f, "a none error, i guess"),
            }
        }
    }

    impl From<NoneError> for Error {
        fn from(e: NoneError) -> Error {
            Error::None(e)
        }
    }

    pub type Result<T> = std::result::Result<T, Error>;
}
pub struct Method(fn(Object) -> Object);

impl Method {
    fn invoke_on<'thread>(&self, obj: Object<'thread>) -> Object<'thread> {
        (self.0)(obj)
    }
}

/// conceptually similar to a typespec or a vtable
pub struct Gender {
    /// note that `size == 0` denotes an immediate object i.e. no
    /// allocation
    size: usize,
    name: String,
    methods: HashMap<String, Method>,
}

impl fmt::Debug for Gender {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Gender {:#x}b at {:p} {}", self.size, self, self.name)
    }
}

impl PartialEq for Gender {
    /// everyone's good friend, pointer equality
    fn eq(&self, other: &Self) -> bool {
        let my_ptr = self as *const Gender as usize;
        let their_ptr = other as *const Gender as usize;
        my_ptr == their_ptr
    }
}

mod genders {
    use super::Gender;
    use lazy_static::lazy_static;

    lazy_static! {
        pub static ref NUMBER: Gender = Gender {
            size: 0,
            name: "number".into(),
            methods: Default::default(),
        };
        pub static ref BOOLEAN: Gender = Gender {
            size: 0,
            name: "boolean".into(),
            methods: Default::default(),
        };
    }
}

#[derive(PartialEq, Debug)]
/// similar to the way rust represents trait objects as "fat
/// pointers", gendered objects are two words: a pointer to a `Gender`
/// and a word of payload. In all of the conceptually interesting
/// cases, the payload is a pointer to an owned (heap?) allocation of
/// `gender->size` bytes.
pub struct Object<'thread> {
    gender: &'thread Gender,

    /// probably a `Box<[u8; gender->size]>` (or a `Box<Self>`,
    /// depending on your perspective), but might be an immediate
    /// value
    payload: usize,
}

impl<'thread> Object<'thread> {
    fn number(n: fsize::Fsize) -> Self {
        make_immediate(&genders::NUMBER, fsize::to_usize(n))
    }
}

/// look up the `fn(Object) -> Object` named by `method` in `obj`'s
/// `gender->methods` and invoke it on `obj`
pub fn dynamic_call<'thread>(
    obj: Object<'thread>,
    method: &'thread str,
) -> err::Result<Object<'thread>> {
    let method = obj.gender.methods.get(method)?;
    let res = method.invoke_on(obj);
    Ok(res)
}

impl<'thread> Drop for Object<'thread> {
    fn drop(&mut self) {
        let Object { gender, payload } = *self;
        if gender.size > 0 {
            let ptr = payload as *mut u8;
            unsafe {
                let slice = std::slice::from_raw_parts_mut(ptr, gender.size) as *mut [u8];
                let _drop_me = Box::from_raw(slice);
            }
        }
    }
}

pub fn allocate_object(gender: &Gender) -> Object {
    debug_assert!(gender.size > 0);

    let vector = vec![0u8; gender.size];
    let slice = Box::leak(vector.into_boxed_slice());

    debug_assert!(slice.len() == gender.size);

    let payload = slice.as_mut_ptr() as usize;

    Object { gender, payload }
}

pub fn make_immediate(gender: &Gender, payload: usize) -> Object {
    debug_assert!(gender.size == 0);

    Object { gender, payload }
}

pub enum Expr {
    Number(fsize::Fsize),
    Boolean(bool),
    If {
        predicate: Box<Expr>,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Let {
        binding: String,
        initial_value: Box<Expr>,
        body: Box<Expr>,
    },
}

#[derive(Default)]
pub struct Thread {}

impl Thread {
    pub fn eval(&mut self, expr: Expr) -> err::Result<Object> {
        let o = match expr {
            Expr::Number(n) => Object::number(n),
            _ => unimplemented!(),
        };

        Ok(o)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn make_an_object() {
        let gender = Box::leak(Box::new(Gender {
            size: 4,
            name: "anonymous test gender for lib.rs/test::make_an_object".into(),
            methods: Default::default(),
        }));
        let obj = allocate_object(gender);

        assert_eq!(
            obj.gender as *const Gender as usize,
            gender as *const Gender as usize
        );
    }

    #[test]
    fn eval_a_number() {
        let expr = Expr::Number(123.456);
        let mut thread = Thread::default();

        let obj = thread.eval(expr).unwrap();

        assert_eq!(fsize::from_usize(obj.payload), 123.456);
        assert_eq!(obj, Object::number(123.456));
    }
}
