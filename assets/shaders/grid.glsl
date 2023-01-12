// Transforms the unit grid centered at the origin
// (the (0, 0) cell's bottom left corner is in the origin)
// to the required grid.
uniform mat3 u_grid_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;

varying vec2 v_quad_pos;

#ifdef VERTEX_SHADER
// Position in screen space
attribute vec2 a_pos;

void main() {
    v_quad_pos = a_pos;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform vec4 u_grid_color;
uniform vec2 u_grid_width;

void main() {
    mat3 camera_matrix = u_projection_matrix * u_view_matrix;
    mat3 inv_camera = inverse(camera_matrix);
    vec3 world_pos = u_grid_matrix * inv_camera * vec3(v_quad_pos, 1.0);
    vec2 pos = world_pos.xy / world_pos.z;

    ivec2 cell_pos = ivec2(0);
    while (pos.x < 0.0) {
        pos.x += 1.0;
        cell_pos.x -= 1;
    }
    while (pos.x >= 1.0) {
        pos.x -= 1.0;
        cell_pos.x += 1;
    }
    while (pos.y < 0.0) {
        pos.y += 1.0;
        cell_pos.y -= 1;
    }
    while (pos.y >= 1.0) {
        pos.y -= 1.0;
        cell_pos.y += 1;
    }

    if (0.5 - abs(pos.x - 0.5) > u_grid_width.x && 0.5 - abs(pos.y - 0.5) > u_grid_width.y) {
        discard;
    }
    
    gl_FragColor = u_grid_color;
}
#endif
