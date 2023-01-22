uniform mat3 u_model_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;

varying vec2 v_uv;
varying vec2 v_mask_uv;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_uv;
attribute vec2 a_mask_uv;

void main() {
    v_uv = a_uv;
    v_mask_uv = a_mask_uv;
    vec3 pos = u_projection_matrix * u_view_matrix * u_model_matrix * vec3(a_pos, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_texture;
uniform sampler2D u_mask;

void main() {
    vec4 texture_color = texture2D(u_texture, v_uv);
    vec4 mask_color = texture2D(u_mask, v_mask_uv);
    vec4 color = texture_color * mask_color;
    gl_FragColor = color;
}
#endif
