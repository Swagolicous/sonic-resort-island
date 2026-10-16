//! Collision data for the original Havok triangle contact-welding path.
//! Each edge stores a five-bit adjacent-face angle: 15 is coplanar and 31
//! leaves an open edge unwelded. The original game performs the contact response.

use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug)]
pub struct WeldedTriangle {
    pub vertices: [[f32; 3]; 3],
    pub welding_info: u16,
}

type Point = [i32; 3];
type Edge = [Point; 2];

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { std::array::from_fn(|i| a[i] - b[i]) }
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { (0..3).map(|i| a[i] * b[i]).sum() }
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
}
fn unit(a: [f32; 3]) -> [f32; 3] {
    let length = dot(a, a).sqrt().max(1e-12);
    a.map(|x| x / length)
}
fn point(p: [f32; 3]) -> Point { p.map(|x| (x * 1000.0).round() as i32) }

pub fn weld_mesh(input: &[[[f32; 3]; 3]]) -> Vec<WeldedTriangle> {
    let mut seen = HashSet::new();
    let mut triangles = Vec::with_capacity(input.len());
    for &vertices in input {
        let mut key = vertices.map(point);
        key.sort();
        if seen.insert(key) {
            triangles.push(WeldedTriangle { vertices, welding_info: 0x7fff });
        }
    }
    let normals: Vec<_> = triangles.iter().map(|t|
        unit(cross(sub(t.vertices[1], t.vertices[0]), sub(t.vertices[2], t.vertices[0])))).collect();
    let mut edges: HashMap<Edge, Vec<(usize, usize)>> = HashMap::new();
    for (i, t) in triangles.iter().enumerate() {
        for edge in 0..3 {
            let mut key = [point(t.vertices[edge]), point(t.vertices[(edge + 1) % 3])];
            key.sort();
            edges.entry(key).or_default().push((i, edge));
        }
    }
    for adjoining in edges.values() {
        for &(i, edge) in adjoining {
            let direction = unit(sub(triangles[i].vertices[(edge + 1) % 3], triangles[i].vertices[edge]));
            let angle = adjoining.iter().filter(|&&(j, _)| j != i && dot(normals[i], normals[j]) > -0.99)
                .map(|&(j, _)| dot(direction, cross(normals[i], normals[j])).atan2(dot(normals[i], normals[j])))
                .min_by(|a, b| a.abs().total_cmp(&b.abs()));
            if let Some(angle) = angle {
                let code = ((angle / std::f32::consts::PI + 1.0) * 15.0).round().clamp(0.0, 30.0) as u16;
                let shift = edge * 5;
                triangles[i].welding_info = (triangles[i].welding_info & !(31 << shift)) | (code << shift);
            }
        }
    }
    triangles
}
