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
// One row per sector, written by sector_lighting.gd. See its layout comment.
global uniform sampler2D source_sector_lights : filter_nearest, repeat_disable;
uniform int source_sector_row = 0;

varying vec3 source_lighting;

// Per-vertex source lighting: GLI_vSendObjectToLights (00492af0) with the
// omni at 00491f70 and the spot at 004922d0. Distance fades on its square
// between near and far; there is no inverse-square term.
void vertex() {
	vec3 position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec3 normal = normalize(mat3(MODEL_MATRIX) * NORMAL);
	// 00491f70 dots in object space and scales by the largest axis, so a part
	// squashed along one axis takes more light on the faces along it.
	mat4 to_object = inverse(MODEL_MATRIX);
	float max_scale = max(max(length(MODEL_MATRIX[0].xyz), length(MODEL_MATRIX[1].xyz)), length(MODEL_MATRIX[2].xyz));
	vec4 head = texelFetch(source_sector_lights, ivec2(0, source_sector_row), 0);
	vec3 sum = head.rgb;
	int count = int(head.a);
	for (int i = 0; i < count; i++) {
		int x = 1 + 4 * i;
		vec4 place = texelFetch(source_sector_lights, ivec2(x, source_sector_row), 0);
		vec4 color = texelFetch(source_sector_lights, ivec2(x + 1, source_sector_row), 0);
		vec4 range = texelFetch(source_sector_lights, ivec2(x + 2, source_sector_row), 0);
		vec4 axis = texelFetch(source_sector_lights, ivec2(x + 3, source_sector_row), 0);
		float near2 = range.x * range.x;
		float far2 = range.y * range.y;
		vec3 to_light = place.xyz - position;
		float d2 = dot(to_light, to_light);
		float fade = far2 > near2 ? (far2 - d2) / (far2 - near2) : 1.0;
		float f = 0.0;
		if (place.w < 1.5) {
			f = max(dot(normal, axis.xyz), 0.0);
		} else if (place.w < 2.5) {
			if (d2 < far2) {
				vec3 object_to_light = (to_object * vec4(place.xyz, 1.0)).xyz - VERTEX;
				f = d2 > 0.0 ? dot(object_to_light, NORMAL) * max_scale / sqrt(d2) : 1.0;
				if (d2 > near2) f *= fade;
			}
		} else {
			float along = dot(to_light, axis.xyz);
			float cone = along != 0.0 ? (d2 - along * along) / (along * along) : range.w;
			if (along > 0.0 && d2 <= far2 && cone < range.w) {
				f = d2 >= near2 ? fade : 1.0;
				if (cone > range.z) f *= (range.w - cone) / (range.w - range.z);
				f *= (dot(normal, axis.xyz) + 1.0) * 0.5;
			}
		}
		if (f > 0.0) sum += color.rgb * f;
	}
	source_lighting = clamp(2.0 * (source_ambient.rgb + source_diffuse.rgb * sum), vec3(0.0), vec3(1.0));
}

void fragment() {
	vec2 source_uv = UV * source_uv_scale + source_uv_offset;
	vec4 source_texel = source_has_albedo_texture ? texture(source_albedo_texture, source_uv) : vec4(1.0);
	ALBEDO = source_texel.rgb * source_albedo_color.rgb * source_lighting;
	ALPHA = source_texel.a * source_albedo_color.a * source_opacity;
	ALPHA_SCISSOR_THRESHOLD = source_alpha_scissor_threshold;
}
