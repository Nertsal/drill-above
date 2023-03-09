uniform mat3 u_model_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_projection_matrix;

varying vec2 v_quad_pos;

float smooth_step(float min, float max, float value) {
    float t = clamp((value - min) / (max - min), 0.0, 1.0);
    return 3.0 * t * t - 2.0 * t * t * t;
}

#ifdef VERTEX_SHADER
attribute vec2 a_pos;

void main() {
    v_quad_pos = a_pos;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
#endif

#ifdef FRAGMENT_SHADER
uniform vec2 u_light_pos;
uniform float u_light_angle;
uniform float u_light_angle_range;
uniform float u_light_angle_gradient;
uniform vec4 u_light_color;
uniform float u_light_intensity;
uniform float u_light_max_distance;
uniform float u_light_distance_gradient;
uniform float u_light_volume;
uniform sampler2D u_normal_texture;
uniform sampler2D u_source_texture;
uniform ivec2 u_framebuffer_size;

void main() {
    // Calculate position relative to the light in world space
    mat3 transform = u_projection_matrix * u_view_matrix * u_model_matrix;
    mat3 inv = inverse(transform);
    vec2 position = (inv * vec3(v_quad_pos, 1.0)).xy - u_light_pos;

    // Convert to polar coordinates
    float distance = length(position);
    float angle = atan(position.y, position.x) - u_light_angle;
    if (angle >= PI) {
        angle -= PI * 2.0;
    }
    if (angle < -PI) {
        angle += PI * 2.0;
    }

    // Radial falloff
    float distance_t = min(distance / u_light_max_distance, 1.0);
    distance_t = pow(distance_t, u_light_distance_gradient);
    float radial_falloff = (1.0 - distance_t) * (1.0 - distance_t);

    // Angular falloff
    float angle_t = abs(angle / u_light_angle_range);
    angle_t = pow(angle_t, u_light_angle_gradient);
    float angular_falloff;
    if (u_light_angle_range >= 2.0 * PI) {
        angular_falloff = 1.0;
    } else {
        angular_falloff = smooth_step(1.0, 0.0, angle_t);
    }

    // Normal falloff
    vec2 light_dir = -position / distance;
    vec2 texture_pos = gl_FragCoord.xy / vec2(u_framebuffer_size);
    vec4 normal_value = texture2D(u_normal_texture, texture_pos);
    vec2 normal = normal_value.rg * 2.0 - 1.0;
    float normal_influence = normal_value.b;
    float normal_falloff = clamp(dot(normal, light_dir), 0.0, 1.0) * normal_influence;
    if (normal_value.a == 0.0) {
        normal_falloff = 1.0;
    }

    // Adjust light intensity based on radial and angular falloff
    float intensity = u_light_intensity * radial_falloff * angular_falloff * normal_falloff;

    // Get the base color of the world
    vec3 base_color = texture2D(u_source_texture, texture_pos).rgb;

    // Adjust light color based on the new intensity
    vec3 light_color = u_light_color.xyz * intensity;
    // Shade the world with the normal falloff
    vec3 shaded_color = base_color * light_color;
    // Add volumetric lighting
    shaded_color += light_color * u_light_volume;

    gl_FragColor = vec4(shaded_color, 1.0);
    // gl_FragColor = vec4(base_color, 1.0);
    // gl_FragColor = vec4(normal_falloff, 0.0, 0.0, 1.0);
    // gl_FragColor = vec4(light_dir * 0.5 + 0.5, 0.0, 1.0);
    // gl_FragColor = vec4(normal * 0.5 + 0.5, 0.0, 1.0);
}
#endif
