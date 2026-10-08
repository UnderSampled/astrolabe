shader_type spatial;
render_mode unshaded, depth_draw_opaque, cull_back;

uniform sampler2D source_albedo_texture : source_color, filter_linear_mipmap;
uniform bool source_has_albedo_texture = false;
uniform vec4 source_albedo_color : source_color = vec4(1.0);
uniform vec4 source_ambient = vec4(0.0);
uniform vec4 source_diffuse = vec4(0.7, 0.7, 0.7, 1.0);
uniform vec2 source_uv_scale = vec2(1.0);
uniform vec2 source_uv_offset = vec2(0.0);
uniform float source_alpha_scissor_threshold = 0.0;
uniform float source_opacity = 1.0;

varying vec4 source_rli;

void vertex() {
	source_rli = CUSTOM0;
}

void fragment() {
	vec2 source_uv = UV * source_uv_scale + source_uv_offset;
	vec4 source_texel = source_has_albedo_texture ? texture(source_albedo_texture, source_uv) : vec4(1.0);
	vec3 source_lighting = clamp(2.0 * (source_ambient.rgb + source_diffuse.rgb * source_rli.rgb), vec3(0.0), vec3(1.0));
	ALBEDO = source_texel.rgb * source_albedo_color.rgb * source_lighting;
	ALPHA = source_texel.a * source_albedo_color.a * source_opacity;
	ALPHA_SCISSOR_THRESHOLD = source_alpha_scissor_threshold;
}
