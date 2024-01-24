#[derive(Clone, Copy, Debug, Default)]
pub struct Object {
    pub pos: [f64; 2],
    pub color: [u8; 3],
}

pub const NUM_OBJS: usize = 1000;
pub const SCALE: f32 = 100.;
