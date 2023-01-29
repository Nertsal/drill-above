uniform mat3 u_model_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;
uniform vec2 u_light_pos;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 a_normal;

void main() {
    vec2 light_dir = u_light_pos - a_pos;
    vec2 vertex = a_pos;
    if (dot(a_normal, light_dir) < 0.0) {
        // The vertex is facing away from the light
        // so we extend that vertex away from the light to make a shadow
        vertex -= light_dir * 1000.0;
    }
    vec3 pos = u_projection_matrix * u_view_matrix * u_model_matrix * vec3(vertex, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
void main() { }
#endif
