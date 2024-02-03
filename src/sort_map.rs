use eframe::{
    egui::{self, Painter, Response},
    epaint::{pos2, Color32, Pos2, Rect, Vec2},
};

use crate::{Object, SCALE};

#[derive(Default, Clone, Copy, Debug)]
pub struct HashEntry {
    particle_idx: usize,
    cell_hash: usize,
}

impl std::fmt::Display for HashEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}, {}}}", self.particle_idx, self.cell_hash)
    }
}

const PARTICLE_RADIUS: f32 = 1.5;
const CELL_SIZE: f32 = PARTICLE_RADIUS;

#[derive(Default)]
pub struct SortMap {
    hash_table: Vec<HashEntry>,
    start_offsets: Vec<usize>,
}

impl SortMap {
    pub fn new(num_objs: usize) -> Self {
        Self {
            hash_table: vec![HashEntry::default(); num_objs],
            start_offsets: vec![usize::MAX; num_objs],
        }
    }

    pub fn resize(&mut self, len: usize) {
        self.hash_table.resize(len, HashEntry::default());
        self.start_offsets.resize(len, usize::MAX);
    }
}

/// A trait to abstract processing the objects combinations.
/// It is used to abstract the logic in [`SortMap`].
///
/// Conceptually, it is a loop like this:
///
/// ```txt
/// for obj1 in objs:
///     start(obj1)
///     for obj2 in objs:
///         next(obj2)
///     end(obj1)
/// ```
///
/// But SortMap can optimize the process by enumerating only neighbors.
pub trait UpdateScanner {
    fn start(&mut self, i: usize, obj1: &Object);
    fn next(&mut self, j: usize, obj2: &Object);
    fn end(&mut self, i: usize, obj1: &mut Object);
}

impl SortMap {
    fn hash(grid_pos: (i32, i32), len: usize) -> usize {
        (grid_pos.0 + grid_pos.1 * 32121).rem_euclid(len as i32) as usize
    }

    pub fn update(&mut self, objs: &[impl AsRef<Object>]) {
        let len = objs.len();

        for (i, (particle_i, hash_entry)) in objs.iter().zip(self.hash_table.iter_mut()).enumerate()
        {
            let pos = particle_i.as_ref().pos;
            let grid_pos = (
                pos[0].div_euclid(CELL_SIZE as f64) as i32,
                pos[1].div_euclid(CELL_SIZE as f64) as i32,
            );
            let cell_hash = Self::hash(grid_pos, len);
            hash_entry.particle_idx = i;
            hash_entry.cell_hash = cell_hash;
        }
        self.hash_table
            .sort_unstable_by_key(|entry| entry.cell_hash);

        for i in 0..objs.len() {
            let first = self
                .hash_table
                .binary_search_by_key(&i, |entry| entry.cell_hash);
            if let Ok(first) = first {
                self.start_offsets[i] = 0;
                for j in (0..=first).rev() {
                    if self.hash_table[j].cell_hash != i {
                        self.start_offsets[i] = j + 1;
                        break;
                    }
                }
            }
        }
    }

    pub fn scan(
        &mut self,
        objs: &mut [impl AsRef<Object> + AsMut<Object>],
        update_scanner: &mut impl UpdateScanner,
    ) {
        // Although it's not idiomatic, we need to borrow the reference to the object as mutable at the end of the loop,
        // so we cannot use iter().enumerate() idiom.
        for i in 0..objs.len() {
            let obj_i = &objs[i];
            let pos = obj_i.as_ref().pos;
            let grid_pos = (
                pos[0].div_euclid(CELL_SIZE as f64) as i32,
                pos[1].div_euclid(CELL_SIZE as f64) as i32,
            );
            update_scanner.start(i, obj_i.as_ref());
            for cy in (grid_pos.1 - 1)..=(grid_pos.1 + 1) {
                for cx in (grid_pos.0 - 1)..=(grid_pos.0 + 1) {
                    let cell_hash = Self::hash((cx, cy), objs.len());
                    let Some(&cell_start) = self.start_offsets.get(cell_hash) else {
                        continue;
                    };
                    if cell_start == usize::MAX {
                        continue;
                    }
                    for entry in &self.hash_table[cell_start..] {
                        if i == entry.particle_idx {
                            continue;
                        }
                        if entry.cell_hash != cell_hash {
                            break;
                        }
                        let obj_j = &objs[entry.particle_idx];
                        update_scanner.next(entry.particle_idx, obj_j.as_ref());
                        // update_speed(obj_i, obj_j);
                    }
                }
            }
            update_scanner.end(i, objs[i].as_mut());
        }
    }

    pub fn render_grid(
        objs: impl Iterator<Item = [f32; 2]>,
        response: &Response,
        painter: &Painter,
    ) {
        // let font_id = FontId::monospace(16.);
        for pos in objs {
            let grid_pos = (
                pos[0].div_euclid(CELL_SIZE) as i32,
                pos[1].div_euclid(CELL_SIZE) as i32,
            );
            let rect = Rect::from_min_size(
                pos2(grid_pos.0 as f32 * CELL_SIZE, grid_pos.1 as f32 * CELL_SIZE),
                Vec2::splat(CELL_SIZE),
            );
            let to_screen = egui::emath::RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO, response.rect.size()),
                response.rect,
            );
            let to_pos2 = |pos: Pos2| to_screen.transform_pos(pos2(pos.x * SCALE, pos.y * SCALE));
            let scr_rect = Rect::from_min_max(to_pos2(rect.min), to_pos2(rect.max));
            painter.rect_stroke(scr_rect, 0., (1., Color32::from_rgb(255, 127, 127)));
        }
    }
}
