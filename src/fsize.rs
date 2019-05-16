//! for, i guess, portability reasons, or something, i've hacked
//! together this interface for interacting with pointer-width
//! floats, since they can be embedded in an object

#[cfg(target_pointer_width = "64")]
pub type Fsize = f64;

#[cfg(target_pointer_width = "32")]
pub type Fsize = f32;

#[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
pub type Fsize =
    compile_error!("what fucking platform are you on. pointer_width should be either 16 or 32.");

pub fn to_usize(f: Fsize) -> usize {
    Fsize::to_bits(f) as usize
}

pub fn from_usize(u: usize) -> Fsize {
    Fsize::from_bits(u as _)
}
