#[macro_export]
macro_rules! vec1 {
    ($x: expr) => {
        cgmath::vec1($x)
    };
}

#[macro_export]
macro_rules! vec2 {
    ($x: expr, $y: expr) => {
        cgmath::vec2($x, $y)
    };
}

#[macro_export]
macro_rules! vec3 {
    ($x: expr, $y: expr, $z: expr) => {
        cgmath::vec3($x, $y, $z)
    };
}

#[macro_export]
macro_rules! vec4 {
    ($x: expr, $y: expr, $z: expr, $w: expr) => {
        cgmath::vec4($x, $y, $z, $w)
    };
}
