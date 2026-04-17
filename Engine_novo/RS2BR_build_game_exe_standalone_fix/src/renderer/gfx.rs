use egui::{self, Painter, TextureId, Pos2, Vec2, Color32, Stroke};

pub fn rotate_vec2(vec: Vec2, rotation_deg: f32) -> Vec2 {
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    egui::vec2(
        vec.x * cos - vec.y * sin,
        vec.x * sin + vec.y * cos,
    )
}

pub fn rotated_rect_points(center: Pos2, size: Vec2, rotation_deg: f32) -> [Pos2; 4] {
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    let half = size * 0.5;
    let corners = [
        egui::vec2(-half.x, -half.y),
        egui::vec2(half.x, -half.y),
        egui::vec2(half.x, half.y),
        egui::vec2(-half.x, half.y),
    ];
    corners.map(|corner| {
        let rotated = egui::vec2(
            corner.x * cos - corner.y * sin,
            corner.x * sin + corner.y * cos,
        );
        center + rotated
    })
}

pub fn rect_from_points(points: &[Pos2; 4]) -> egui::Rect {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y))
}

pub fn paint_rotated_image(
    painter: &Painter,
    texture_id: TextureId,
    center: Pos2,
    size: Vec2,
    rotation_deg: f32,
    tint: Color32,
    flip_x: bool,
    flip_y: bool,
) -> egui::Rect {
    let points = rotated_rect_points(center, size, rotation_deg);
    let (u0, u1) = if flip_x { (1.0, 0.0) } else { (0.0, 1.0) };
    let (v0, v1) = if flip_y { (1.0, 0.0) } else { (0.0, 1.0) };
    let mut mesh = egui::epaint::Mesh::with_texture(texture_id);
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex { pos: points[0], uv: egui::pos2(u0, v0), color: tint });
    mesh.vertices.push(egui::epaint::Vertex { pos: points[1], uv: egui::pos2(u1, v0), color: tint });
    mesh.vertices.push(egui::epaint::Vertex { pos: points[2], uv: egui::pos2(u1, v1), color: tint });
    mesh.vertices.push(egui::epaint::Vertex { pos: points[3], uv: egui::pos2(u0, v1), color: tint });
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    painter.add(egui::Shape::mesh(mesh));
    rect_from_points(&points)
}

pub fn paint_rotated_placeholder(
    painter: &Painter,
    center: Pos2,
    size: Vec2,
    rotation_deg: f32,
    fill: Color32,
    stroke: Stroke,
) -> egui::Rect {
    let points = rotated_rect_points(center, size, rotation_deg);
    painter.add(egui::Shape::convex_polygon(points.to_vec(), fill, stroke));
    painter.line_segment([points[0], points[2]], stroke);
    painter.line_segment([points[1], points[3]], stroke);
    rect_from_points(&points)
}

pub fn paint_rotated_outline(
    painter: &Painter,
    center: Pos2,
    size: Vec2,
    rotation_deg: f32,
    stroke: Stroke,
) -> egui::Rect {
    let points = rotated_rect_points(center, size, rotation_deg);
    for index in 0..4 {
        painter.line_segment([points[index], points[(index + 1) % 4]], stroke);
    }
    rect_from_points(&points)
}
