#![feature(try_trait)]

use std::{collections::HashMap, option::NoneError, fmt};

#[cfg(target_pointer_width = "64")]
type Fsize = f64;

#[cfg(target_pointer_width = "32")]
type Fsize = f32;

#[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
type Fsize = compile_error!("what fucking platform are you on");

fn fsize_to_usize(f: Fsize) -> usize {
    Fsize::to_bits(f) as usize
}

fn usize_to_fsize(u: usize) -> Fsize {
    Fsize::from_bits(u as _)
}

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
    fn from(e: NoneError) -> Error { Error::None(e) }
}

pub struct Method(fn(Object) -> Object);

impl Method {
    fn invoke_on(&self, obj: Object) -> Object {
        (self.0)(obj)
    }
}

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
pub struct Object {
    /// TODO: determine the right pointer type for
    /// genders. realistically, they should never get freed or
    /// mutated, so `&'static Gender` seems most correct, but it's
    /// kinda fucky
    gender: &'static Gender, 

    /// probably a `Box<[u8; gender->size]>` (or a `Box<Self>`,
    /// depending on your perspective), but might be an immediate
    /// value
    payload: usize,
}

impl Object {
    fn number(n: Fsize) -> Object {
        make_immediate(&genders::NUMBER, fsize_to_usize(n))
    }
}

pub fn dynamic_call(obj: Object, method: &str) -> Result<Object, Error> {
    let method = obj.gender.methods.get(method)?;
    let res = method.invoke_on(obj);
    Ok(res)
}

impl Drop for Object {
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

pub fn allocate_object(gender: &'static Gender) -> Object {
    debug_assert!(gender.size > 0);
    
    let vector = vec![0u8; gender.size];
    let slice = Box::leak(vector.into_boxed_slice());

    debug_assert!(slice.len() == gender.size);
    
    let payload = slice.as_mut_ptr() as usize;

    Object {
        gender,
        payload,
    }
    
}

pub fn make_immediate(gender: &'static Gender, payload: usize) -> Object {
    debug_assert!(gender.size == 0);

    Object {
        gender,
        payload,
    }
}

pub enum Expr {
    Number(Fsize),
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
    pub fn eval(&mut self, expr: Expr) -> Result<Object, Error> {
        let o = match expr {
            Expr::Number(n) => Object::number(n),
            _ => unimplemented!()
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

        assert_eq!(usize_to_fsize(obj.payload), 123.456);
        assert_eq!(obj, Object::number(123.456));
    }
}
