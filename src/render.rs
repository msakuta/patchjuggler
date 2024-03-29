use eframe::{
    egui::{self, Painter, Response, Ui},
    epaint::{pos2, vec2, Color32, PathShape, Pos2, Rect},
};

use crate::{object::AsObject, SCALE};

pub fn render_objects(
    objs: &[impl AsObject],
    selected: Option<usize>,
    ui: &mut Ui,
    show_updates: bool,
) -> (Response, Painter) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());

    let rotate = |pos: &Pos2, angle: f32| {
        let c = angle.cos();
        let s = angle.sin();
        vec2(pos.x * c - pos.y * s, pos.x * s + pos.y * c)
    };

    let to_screen = egui::emath::RectTransform::from_to(
        Rect::from_min_size(Pos2::ZERO, response.rect.size()),
        response.rect,
    );

    let convert_to_poly = |vertices: &[Pos2], pos: Pos2, angle: f32, color: Color32| {
        PathShape::convex_polygon(
            vertices
                .into_iter()
                .map(|ofs| to_screen.transform_pos(pos + rotate(ofs, angle)))
                .collect(),
            color,
            (1., Color32::BLACK),
        )
    };

    if show_updates {
        for as_obj in objs.iter() {
            let obj = as_obj.as_ref();
            if let Some(fcolor) = as_obj.render_circle() {
                painter.circle_filled(
                    to_screen
                        .transform_pos(pos2(obj.pos[0] as f32 * SCALE, obj.pos[1] as f32 * SCALE)),
                    15.,
                    fcolor,
                );
            }
        }
    }

    for (i, as_obj) in objs.iter().enumerate() {
        let obj = as_obj.as_ref();
        let color = if Some(i) == selected {
            Color32::WHITE
        } else {
            as_obj.get_color()
        };
        let heading = obj.velo[1].atan2(obj.velo[0]) as f32;
        painter.add(convert_to_poly(
            &[pos2(10., 0.), pos2(-5., 5.), pos2(-5., -5.)],
            pos2(obj.pos[0] as f32, obj.pos[1] as f32) * SCALE,
            heading,
            color,
        ));
    }

    (response, painter)
}
