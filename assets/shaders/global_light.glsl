#ifdef VERTEX_SHADER
attribute vec2 a_pos;

void main() {
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_source_texture;
uniform ivec2 u_framebuffer_size;
uniform vec4 u_light_color;
uniform float u_light_intensity;

void main() {
    // Get the base color of the world
    vec2 texture_pos = gl_FragCoord.xy / vec2(u_framebuffer_size);
    vec4 base_color = texture2D(u_source_texture, texture_pos);

    // Adjust light color with the intensity
    vec4 light_color = vec4(u_light_color.rgb * u_light_intensity, 1.0);
    // Shade the world with the light
    vec4 shaded_color = base_color * light_color;
    gl_FragColor = shaded_color;
}
#endif
