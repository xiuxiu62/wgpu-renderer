use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Mat2<T>(MatN<T, 2, 2>);

impl<T> Mat2<T> {
    pub fn new(inner: [[T; 2]; 2]) -> Self {
        Self(MatN(inner))
    }
}

unsafe impl<T: Pod> Pod for Mat2<T> {}
unsafe impl<T: Zeroable> Zeroable for Mat2<T> {}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Mat3<T>(MatN<T, 3, 3>);

impl<T> Mat3<T> {
    pub fn new(inner: [[T; 3]; 3]) -> Self {
        Self(MatN(inner))
    }
}

unsafe impl<T: Pod> Pod for Mat3<T> {}
unsafe impl<T: Zeroable> Zeroable for Mat3<T> {}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Mat4<T>(MatN<T, 4, 4>);

impl<T> Mat4<T> {
    pub fn new(inner: [[T; 4]; 4]) -> Self {
        Self(MatN(inner))
    }
}

unsafe impl<T: Pod> Pod for Mat4<T> {}
unsafe impl<T: Zeroable> Zeroable for Mat4<T> {}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct MatN<T, const W: usize, const H: usize>([[T; W]; H]);

unsafe impl<T: Pod, const W: usize, const H: usize> Pod for MatN<T, W, H> {}
unsafe impl<T: Zeroable, const W: usize, const H: usize> Zeroable for MatN<T, W, H> {}
