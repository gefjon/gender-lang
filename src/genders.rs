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

pub fn string(len: usize) -> Gender {
    Gender {
        size: len,
        name: format!("string of length {}", len),
        methods: Default::default(),
    }
}

pub fn simple_vector(repeat: usize) -> Gender {
    let size = repeat * std::mem::size_of::<crate::Object>();
    Gender {
        size,
        name: format!("simple vector of length {}", repeat),
        methods: Default::default(),
    }
}
