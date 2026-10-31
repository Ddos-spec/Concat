struct Params { smooth: f32, threshold: f32 }

// Edge-preserving bilateral skin smoothing: averages neighbouring pixels
// weighted by photometric distance in display space, softening pores and
// blemishes while keeping eyes, eyebrows and lips crisp. On a flat field
// every neighbour equals the centre pixel, preserving exact linearity.
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let center = sample(uv);
    let strength = params.smooth * 0.01;
    if (strength <= 0.0) {
        return center;
    }

    let t = texel() * 1.5;
    let center_disp = to_display(center.rgb);
    let sigma_color = max(params.threshold * 0.01, 0.02);
    let inv_two_sigma2 = 1.0 / (2.0 * sigma_color * sigma_color);

    var sum = center.rgb;
    var total_w = 1.0;

    for (var y: i32 = -2; y <= 2; y++) {
        for (var x: i32 = -2; x <= 2; x++) {
            if (x == 0 && y == 0) {
                continue;
            }
            let neighbor = sample(uv + vec2<f32>(f32(x), f32(y)) * t).rgb;
            let diff = to_display(neighbor) - center_disp;
            let dist2 = dot(diff, diff);
            let spatial = exp(-f32(x * x + y * y) / 8.0);
            let w = spatial * exp(-dist2 * inv_two_sigma2);
            sum += neighbor * w;
            total_w += w;
        }
    }

    let smoothed = sum / total_w;
    let blended = mix(center.rgb, smoothed, strength);
    return vec4<f32>(blended, center.a);
}
