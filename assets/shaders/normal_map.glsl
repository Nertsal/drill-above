uniform mat3 u_model_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;

varying vec2 v_normal;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_normal;

void main() {
    v_normal = (u_model_matrix * vec3(a_normal, 1.0)).xy;
    vec3 pos = u_projection_matrix * u_view_matrix * u_model_matrix * vec3(a_pos, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
uniform float u_normal_influence;

void main() {
    // Encode the normal into the Red and Green channels
    // The Blue channel is used for the influence coefficient
    gl_FragColor = vec4(v_normal * 0.5 + 0.5, u_normal_influence, 1.0);
}
#endif
