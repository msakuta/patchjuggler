use eframe::{
    egui::{self, Painter, Response, Ui},
    epaint::{pos2, vec2, Color32, PathShape, Pos2, Rect},
};

use crate::{Object, SCALE};

pub fn render_objects(objs: &[Object], ui: &mut Ui) -> (Response, Painter) {
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

    for obj in objs.iter() {
        let color = Color32::from_rgb(obj.color[0], obj.color[1], obj.color[2]);
        // painter.circle(
        //     to_screen.transform_pos(pos2(obj.pos[0] as f32 * SCALE, obj.pos[1] as f32 * SCALE)),
        //     3.,
        //     color,
        //     (1., Color32::BLACK),
        // );
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