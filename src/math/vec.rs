use crate::{vec2, vec3, vec4};
use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vec2(VecN<2>);

impl Vec2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self(VecN([x, y]))
    }

    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self(VecN::splat(value))
    }

    #[inline]
    pub const fn extend(&self, value: f32) -> Vec3 {
        vec3!(self.0 .0[0], self.0 .0[1], value)
    }

    #[inline]
    pub const fn x(&self) -> f32 {
        self.0 .0[0]
    }

    #[inline]
    pub const fn y(&self) -> f32 {
        self.0 .0[1]
    }

    #[inline]
    pub const fn xy(&self) -> &[f32; 2] {
        &self.0 .0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vec3(VecN<3>);

impl Vec3 {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self(VecN([x, y, z]))
    }

    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self(VecN::splat(value))
    }

    #[inline]
    pub const fn extend(&self, value: f32) -> Vec4 {
        vec4!(self.0 .0[0], self.0 .0[1], self.0 .0[2], value)
    }

    #[inline]
    pub const fn truncate(&self) -> Vec2 {
        vec2!(self.x(), self.y())
    }

    #[inline]
    pub const fn x(&self) -> f32 {
        self.0 .0[0]
    }

    #[inline]
    pub const fn y(&self) -> f32 {
        self.0 .0[1]
    }

    #[inline]
    pub const fn z(&self) -> f32 {
        self.0 .0[2]
    }

    #[inline]
    pub const fn xyz(&self) -> &[f32; 3] {
        &self.0 .0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vec4(VecN<4>);

impl Vec4 {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self(VecN([x, y, z, w]))
    }

    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self(VecN::splat(value))
    }

    #[inline]
    pub const fn truncate(&self) -> Vec3 {
        vec3!(self.x(), self.y(), self.z())
    }

    #[inline]
    pub const fn x(&self) -> f32 {
        self.0 .0[0]
    }

    #[inline]
    pub const fn y(&self) -> f32 {
        self.0 .0[1]
    }

    #[inline]
    pub const fn z(&self) -> f32 {
        self.0 .0[2]
    }

    #[inline]
    pub const fn w(&self) -> f32 {
        self.0 .0[3]
    }

    #[inline]
    pub const fn xyzw(&self) -> &[f32; 4] {
        &self.0 .0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct VecN<const N: usize>([f32; N]);

impl<const N: usize> VecN<N> {
    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self([value; N])
    }
}

unsafe impl<const N: usize> Zeroable for VecN<N> {}
unsafe impl<const N: usize> Pod for VecN<N> {}

#[macro_export]
macro_rules! vec2 {
    ($x:expr, $y:expr) => {
        $crate::math::vec::Vec2::new($x, $y)
    };
}

#[macro_export]
macro_rules! vec3 {
    ($x:expr, $y:expr, $z:expr) => {
        $crate::math::vec::Vec3::new($x, $y, $z)
    };
}

#[macro_export]
macro_rules! vec4 {
    ($x:expr, $y:expr, $z:expr, $w:expr) => {
        $crate::math::vec::Vec4::new($x, $y, $z, $w)
    };
}

#[macro_export]
macro_rules! vec_n {
($($value:expr),*) => {
    Vec4::new($($value),*)
};
}
