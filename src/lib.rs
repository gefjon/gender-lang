#![feature(try_trait)]

use lalrpop_util::lalrpop_mod;
use std::{collections::HashMap, fmt};

pub mod fsize;

pub mod err;

lalrpop_mod!(pub grammar);

pub mod genders;

pub struct Method {}

impl Method {
    fn invoke_on<'thread>(&self, _obj: Object<'thread>) -> Object<'thread> {
        unimplemented!()
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
    fn try_extract_bool(&self) -> Option<bool> {
        if self.gender.eq(&genders::BOOLEAN) {
            Some(self.payload != 0)
        } else {
            None
        }
    }
    fn boolean(b: bool) -> Self {
        make_immediate(&genders::BOOLEAN, b as usize)
    }
    unsafe fn shallow_copy(&self) -> Self {
        Object {
            gender: self.gender,
            payload: self.payload,
        }
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

#[derive(Clone)]
pub enum Expr {
    Number(fsize::Fsize),
    Boolean(bool),
    Symbol(String),
    If {
        predicate: Box<Expr>,
        then_clause: Box<Expr>,
        else_clause: Box<Expr>,
    },
    Let {
        binding: String,
        initial_value: Box<Expr>,
        body: Box<Expr>,
    },
    /// Halt and Catch Fire - like panic
    Hcf,
}

#[derive(Default)]
pub struct Thread<'a> {
    bindings: Vec<(String, Object<'a>)>,
}

impl<'a> Thread<'a> {
    pub fn eval(&mut self, expr: Expr) -> err::Result<Object<'a>> {
        match expr {
            Expr::Number(n) => Ok(Object::number(n)),
            Expr::Boolean(b) => Ok(Object::boolean(b)),
            Expr::Symbol(s) => self
                .bindings
                .iter()
                .rev()
                .find_map(|(k, v)| if *k == s { Some(v) } else { None })
                .map(|v| unsafe { v.shallow_copy() })
                .ok_or(err::Error::UnboundSymbol(s)),
            Expr::If {
                predicate,
                then_clause,
                else_clause,
            } => {
                let pred = {
                    let pred_result: Object = self.eval(*predicate)?;
                    pred_result.try_extract_bool()?
                };

                if pred {
                    self.eval(*then_clause)
                } else {
                    self.eval(*else_clause)
                }
            }
            Expr::Let {
                binding,
                initial_value,
                body,
            } => {
                let initial_value = self.eval(*initial_value)?;

                self.bindings.push((binding, initial_value));

                let res = self.eval(*body)?;

                self.bindings.pop().unwrap();

                Ok(res)
            }
            Expr::Hcf => Err(err::Hcf),
            // _ => unimplemented!(),
        }
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

    #[test]
    fn simple_conditional() {
        let expr = Expr::If {
            predicate: Box::new(Expr::Boolean(true)),
            then_clause: Box::new(Expr::Number(420.69)),
            else_clause: Box::new(Expr::Hcf),
        };

        let mut thread = Thread::default();

        let obj = thread.eval(expr).unwrap();

        assert_eq!(obj, Object::number(420.69));
    }

    #[test]
    fn halt_and_catch_fire() {
        let expr = Expr::Hcf;
        let mut thread = Thread::default();

        let res = thread.eval(expr);

        assert!(res.is_err());
    }
    #[test]
    fn do_a_binding() {
        let symbol = "foo";

        let expr = Expr::Symbol(symbol.to_string());

        let mut thread = Thread {
            bindings: vec![(symbol.to_string(), Object::number(123.456))],
            ..Thread::default()
        };

        let obj = thread.eval(expr).unwrap();

        assert_eq!(obj, Object::number(123.456));
    }
    #[test]
    fn do_a_let_binding() {
        let symbol = "foo";
        let mut thread = Thread::default();

        let unbound_expr = Expr::Symbol(symbol.to_string());
        let res = thread.eval(unbound_expr.clone());
        assert!(res.is_err());

        let let_expr = Expr::Let {
            binding: symbol.to_string(),
            initial_value: Box::new(Expr::Number(123.456)),
            body: Box::new(unbound_expr),
        };
        let obj = thread.eval(let_expr).unwrap();
        assert_eq!(obj, Object::number(123.456));
    }
    #[test]
    fn parse_an_if_expr() {
        let expr = "if true true false";
        let expr = grammar::ExprParser::new().parse(expr).unwrap();

        let mut thread = Thread::default();
        let obj = thread.eval(expr).unwrap();
        assert_eq!(obj, Object::boolean(true));
    }
    #[test]
    fn parse_a_let_expr() {
        let expr = "let foo = true in foo";
        let expr = grammar::ExprParser::new().parse(expr).unwrap();

        let mut thread = Thread::default();
        let obj = thread.eval(expr).unwrap();
        assert_eq!(obj, Object::boolean(true));
    }
}
