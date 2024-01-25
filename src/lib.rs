use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};

pub const DELTA_TIME: f64 = 1. / 20.;
pub const SPACE_WIDTH: f64 = 10.;

#[derive(Clone, Copy, Debug, Default, FromZeroes, FromBytes, AsBytes)]
#[repr(C)]
pub struct Object {
    pub pos: [f64; 2],
    pub velo: [f64; 2],
    pub color: [u8; 3],
    _pad: [u8; 5],
}

impl Object {
    pub fn new(pos: [f64; 2], color: [u8; 3]) -> Self {
        Self {
            pos,
            velo: [0f64; 2],
            color,
            _pad: [0; 5],
        }
    }

    pub fn time_step(&mut self) {
        for axis in [0, 1] {
            self.pos[axis] = (self.pos[axis] + DELTA_TIME * self.velo[axis]).clamp(0., SPACE_WIDTH);
        }
    }
}

pub const NUM_OBJS: usize = 1000;
pub const SCALE: f32 = 100.;
