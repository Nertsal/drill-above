uniform mat3 u_model_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;

varying vec2 v_uv;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_uv;

void main() {
    v_uv = a_uv;
    vec3 pos = u_projection_matrix * u_view_matrix * u_model_matrix * vec3(a_pos, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
uniform float u_normal_influence;
uniform sampler2D u_normal_texture;

void main() {
    vec4 normal = texture2D(u_normal_texture, v_uv);

    // Encode the normal into the Red and Green channels
    // The Blue channel is used for the influence coefficient
    gl_FragColor = vec4(normal.xy, u_normal_influence, 1.0);
}
#endif
