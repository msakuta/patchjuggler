use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};

pub const DELTA_TIME: f64 = 1. / 20.;
pub const SPACE_WIDTH: f64 = 10.;
const WALL_REPULSION: f64 = 5e-2;
const WALL_REPULSION_DIST: f64 = 0.5;
const MIN_SPEED: f64 = 0.25;
const MAX_SPEED: f64 = 0.5;
const SPEED_ADAPT: f64 = 1e-2;

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
            if self.pos[axis] < WALL_REPULSION_DIST {
                self.velo[axis] += WALL_REPULSION;
            } else if SPACE_WIDTH - WALL_REPULSION_DIST < self.pos[axis] {
                self.velo[axis] -= WALL_REPULSION;
            }
        }
        let vx = self.velo[0];
        let vy = self.velo[1];
        let speed2 = vx.powi(2) + vy.powi(2);
        if 0. < speed2 && speed2 < MIN_SPEED.powi(2) {
            let speed = speed2.sqrt();
            self.velo[0] += vx / speed * SPEED_ADAPT;
            self.velo[1] += vy / speed * SPEED_ADAPT;
        } else if MAX_SPEED.powi(2) < speed2 {
            let speed = speed2.sqrt();
            self.velo[0] -= vx / speed * SPEED_ADAPT;
            self.velo[1] -= vy / speed * SPEED_ADAPT;
        }

        for axis in [0, 1] {
            self.pos[axis] = (self.pos[axis] + DELTA_TIME * self.velo[axis]).clamp(0., SPACE_WIDTH);
        }
    }
}

pub const NUM_OBJS: usize = 1000;
pub const SCALE: f32 = 50.;
