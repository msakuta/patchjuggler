use eframe::epaint::Color32;

use crate::{object::AsObject, Object};

#[derive(Clone, Copy)]
pub struct ObjectWrap {
    obj: Object,
    updated: std::time::Instant,
}

impl ObjectWrap {
    pub fn new(obj: Object) -> Self {
        Self {
            obj,
            updated: std::time::Instant::now(),
        }
    }

    pub fn updated(&self) -> std::time::Instant {
        self.updated
    }
}

impl AsRef<Object> for ObjectWrap {
    fn as_ref(&self) -> &Object {
        &self.obj
    }
}

impl AsMut<Object> for ObjectWrap {
    fn as_mut(&mut self) -> &mut Object {
        &mut self.obj
    }
}

impl Default for ObjectWrap {
    fn default() -> Self {
        Self {
            obj: Object::default(),
            updated: std::time::Instant::now(),
        }
    }
}

impl AsObject for ObjectWrap {
    fn get_color(&self) -> Color32 {
        let obj = &self.obj;
        // let age = std::time::Instant::now() - self.updated();
        // let modulation = age.as_secs_f64();
        obj.get_color()
        // Color32::from_rgb(
        //     (obj.color[0] as f64 * (1. - modulation)).clamp(0., 255.) as u8,
        //     (obj.color[1] as f64 * (1. - modulation)).clamp(0., 255.) as u8,
        //     (obj.color[2] as f64 * (1. - modulation)).clamp(0., 255.) as u8,
        // )
    }

    fn render_circle(&self) -> Option<Color32> {
        let age = (std::time::Instant::now() - self.updated()).as_secs_f64();
        if age < 1. {
            Some(Color32::from_rgba_premultiplied(
                255,
                0,
                255,
                ((1. - age) * 63.).clamp(0., 255.) as u8,
            ))
        } else {
            None
        }
    }
}
