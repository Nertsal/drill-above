use super::*;

pub struct StaticPolygon {
    pub vertices: ugli::VertexBuffer<ShadowVertex>,
    pub doubled: ugli::VertexBuffer<ShadowVertex>,
}

#[derive(ugli::Vertex, Debug, Clone, Copy)]
pub struct ShadowVertex {
    pub a_pos: vec2<f32>,
    pub a_normal: vec2<f32>,
}

impl StaticPolygon {
    pub fn new(geng: &Geng, vertices: &[vec2<f32>]) -> Self {
        let normals: Vec<vec2<f32>> = vertices
            .iter()
            .zip(vertices.iter().cycle().skip(1))
            .map(|(&left, &right)| (left - right).normalize_or_zero().rotate_90())
            .collect();
        Self {
            doubled: ugli::VertexBuffer::new_dynamic(
                geng.ugli(),
                vertices
                    .iter()
                    .zip(
                        normals
                            .iter()
                            .cycle()
                            .skip(normals.len() - 1)
                            .zip(normals.iter()),
                    )
                    .flat_map(|(&pos, (&left_normal, &right_normal))| {
                        [
                            ShadowVertex {
                                a_pos: pos,
                                a_normal: left_normal,
                            },
                            ShadowVertex {
                                a_pos: pos,
                                a_normal: right_normal,
                            },
                        ]
                    })
                    .collect(),
            ),
            vertices: ugli::VertexBuffer::new_dynamic(
                geng.ugli(),
                vertices
                    .iter()
                    .enumerate()
                    .map(|(i, &a_pos)| ShadowVertex {
                        a_pos,
                        a_normal: (normals[i] + normals[(i + normals.len() - 1) % normals.len()])
                            .normalize_or_zero(),
                    })
                    .collect(),
            ),
        }
    }
}
