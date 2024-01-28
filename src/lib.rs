pub mod object;
mod render;
mod sort_map;

pub use crate::{
    object::Object,
    render::render_objects,
    sort_map::{HashEntry, SortMap, UpdateScanner},
};

pub const DELTA_TIME: f64 = 1. / 20.;
pub const SPACE_WIDTH: f64 = 10.;
pub const NUM_OBJS: usize = 1000;
pub const SCALE: f32 = 50.;
