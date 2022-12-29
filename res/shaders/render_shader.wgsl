struct Particle {
    @location(0) pos: vec2<f32>,
    @location(1) vel: vec2<f32>,
}

@group(0) @binding(0) var out_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;

@compute @workgroup_size(1, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let dimensions = textureDimensions(out_texture);
    let pixel_coords = vec2<i32>(global_id.xy);

    if (pixel_coords.x >= dimensions.x || pixel_coords.y >= dimensions.y) {
        return;
    }

    let color = vec3(
        vec2<f32>(global_id.xy) / vec2<f32>(dimensions / 15),
        0f
    );

    let pos = vec2<f32>(color.xy);
    let particles_len = i32(arrayLength(&particles));
    var closest: f32 = distance(particles[0].pos, pos);

    for (var i: i32 = 0; i < particles_len; i++ ) {
        let dist = distance(particles[i].pos, pos);

        if (dist < closest) {
            closest = dist;
        }
    }

    let dst = 1f - step(0.5f, closest);
    let color = vec3(dst * 0.4f, 0f, dst);

    textureStore(out_texture, pixel_coords, vec4(color, 1f));
}