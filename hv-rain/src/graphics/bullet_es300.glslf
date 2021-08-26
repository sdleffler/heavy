#version 300 es

uniform mediump sampler2D t_Texture;
in mediump vec2 v_Uv;
in mediump vec4 v_Color;
out mediump vec4 Target0;

uniform mediump mat4 u_MVP;

void main() {
    mediump vec4 texel = texture(t_Texture, v_Uv);
    Target0 = vec4(
        vec3(1) - ((vec3(1) - texel.rgb) * (vec3(1) - v_Color.rgb)),
        texel.a * v_Color.a
    );
}