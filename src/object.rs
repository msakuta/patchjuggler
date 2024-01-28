use rand::{rngs::ThreadRng, Rng};
use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};

use crate::{UpdateScanner, DELTA_TIME, SPACE_WIDTH};

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

/// A scanner to update the behavior of objects as boids.
/// It requires other objects' information, so it is a O(N^2) operation naively, which
/// turns into O(N*M), where M is the average number of other objects in the SortMap.
pub struct BoidScanner<'a> {
    rng: Option<&'a mut ThreadRng>,
    randomness: f64,
    obj1: Option<Object>,
    force: [f64; 2],
    cohesion: [f64; 2],
    cohesion_count: usize,
}

impl<'a> BoidScanner<'a> {
    pub fn new(rng: Option<&'a mut ThreadRng>, randomness: f64) -> Self {
        Self {
            rng,
            randomness,
            obj1: None,
            force: [0.; 2],
            cohesion: [0.; 2],
            cohesion_count: 0,
        }
    }
}

pub const RANDOM_MOTION: f64 = 5e-3;
pub const SEPARATION: f64 = 5e-3;
pub const SEPARATION_DIST: f64 = 0.2;
pub const PREDICTION_TIME: f64 = 0.;
pub const ALIGNMENT: f64 = 1e-2;
pub const ALIGNMENT_DIST: f64 = 0.7;
pub const COHESION: f64 = 1e-4;
pub const COHESION_DIST: f64 = 0.7;
pub const GROUP_SEPARATION: f64 = 2e-3;
pub const GROUP_SEPARATION_DIST: f64 = 1.5;
pub const DRAG: f64 = 0.;

impl<'a> UpdateScanner for BoidScanner<'a> {
    fn start(&mut self, _i: usize, obj1: &Object) {
        self.obj1 = Some(*obj1);
        self.force = [0f64; 2];
        self.cohesion = [0f64; 2];
        self.cohesion_count = 0;
    }

    fn next(&mut self, _j: usize, obj2: &Object) {
        let Some(obj1) = self.obj1 else {
            return;
        };
        let dx = obj1.pos[0] - obj2.pos[0];
        let dy = obj1.pos[1] - obj2.pos[1];
        let dist2 = dx.powi(2) + dy.powi(2);
        if dist2 == 0. {
            return;
        }
        let dist = dist2.sqrt();
        let predicted_pos = [
            obj2.pos[0] + PREDICTION_TIME * obj2.velo[0]
                - obj1.pos[0]
                - PREDICTION_TIME * obj1.velo[0],
            obj2.pos[1] + PREDICTION_TIME * obj2.velo[1]
                - obj1.pos[1]
                - PREDICTION_TIME * obj1.velo[1],
        ];
        let predicted_dist2 = predicted_pos[0].powi(2) + predicted_pos[1].powi(2);
        if predicted_dist2 < SEPARATION_DIST.powi(2) {
            let predicted_dist = predicted_dist2.sqrt();
            self.force[0] +=
                SEPARATION * dx / predicted_dist * (1. - predicted_dist / SEPARATION_DIST);
            self.force[1] +=
                SEPARATION * dy / predicted_dist * (1. - predicted_dist / SEPARATION_DIST);
        }
        if dist < ALIGNMENT_DIST {
            self.force[0] += (obj2.velo[0] - obj1.velo[0]) * ALIGNMENT;
            self.force[1] += (obj2.velo[1] - obj1.velo[1]) * ALIGNMENT;
        }
        if dist < COHESION_DIST {
            self.cohesion[0] += COHESION * dx / dist;
            self.cohesion[1] += COHESION * dy / dist;
            self.cohesion_count += 1;
        } else if dist < GROUP_SEPARATION_DIST {
            self.force[0] += GROUP_SEPARATION * dx / dist * (1. - dist / GROUP_SEPARATION_DIST);
            self.force[1] += GROUP_SEPARATION * dy / dist * (1. - dist / GROUP_SEPARATION_DIST);
        }
    }

    fn end(&mut self, _i: usize, obj: &mut Object) {
        for axis in [0, 1] {
            obj.velo[axis] += self.force[axis] - obj.velo[axis] * DRAG;
            if let Some(rng) = &mut self.rng {
                obj.velo[axis] += (rng.gen::<f64>() - 0.5) * self.randomness;
            }
            if 0 < self.cohesion_count {
                obj.velo[axis] += self.cohesion[axis] / self.cohesion_count as f64;
            }
        }

        obj.time_step();
    }
}

/// A scanner that finds neighboring objects info
pub struct FindScanner {
    find_index: Option<usize>,
    find_result: Vec<usize>,
    i: Option<usize>,
}

impl FindScanner {
    pub fn new(find_index: Option<usize>) -> Self {
        Self {
            find_index,
            find_result: vec![],
            i: None,
        }
    }

    pub fn into_find_result(self) -> Vec<usize> {
        self.find_result
    }
}

impl UpdateScanner for FindScanner {
    fn start(&mut self, i: usize, _obj1: &Object) {
        self.i = Some(i);
    }

    fn next(&mut self, j: usize, _obj2: &Object) {
        if self.i.is_some() && self.i == self.find_index {
            self.find_result.push(j);
        }
    }

    fn end(&mut self, _i: usize, _obj: &mut Object) {}
}
