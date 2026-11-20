struct Params {
    red_hue: f32,
    red_sat: f32,
    red_lum: f32,
    orange_hue: f32,
    orange_sat: f32,
    orange_lum: f32,
    yellow_hue: f32,
    yellow_sat: f32,
    yellow_lum: f32,
    green_hue: f32,
    green_sat: f32,
    green_lum: f32,
    cyan_hue: f32,
    cyan_sat: f32,
    cyan_lum: f32,
    blue_hue: f32,
    blue_sat: f32,
    blue_lum: f32,
    purple_hue: f32,
    purple_sat: f32,
    purple_lum: f32,
    magenta_hue: f32,
    magenta_sat: f32,
    magenta_lum: f32,
}

// The bands are worked here, not through the library's hsl_band: a turn of
// nothing must be no turn at all (the YIQ rotation does not undo itself
// exactly, and a panel at zero is the picture as it came, not one drifted by
// a hundredth), and the helpers it leans on are only stitched in when a
// package names them.

/// Hue in degrees, 0..360; 0 for a grey.
fn hsl_hue(c: vec3<f32>) -> f32 {
    let mx = max(max(c.r, c.g), c.b);
    let mn = min(min(c.r, c.g), c.b);
    let d = mx - mn;
    if (d < 0.0001) {
        return 0.0;
    }
    var h: f32;
    if (mx == c.r) {
        h = (c.g - c.b) / d;
    } else if (mx == c.g) {
        h = 2.0 + (c.b - c.r) / d;
    } else {
        h = 4.0 + (c.r - c.g) / d;
    }
    return fract(h / 6.0) * 360.0;
}

/// How much a pixel belongs to the hues within `width` degrees of `centre`,
/// weighted by chroma so a grey belongs to no band.
fn hsl_mask(c: vec3<f32>, centre: f32, width: f32) -> f32 {
    let d = abs(fract((hsl_hue(c) - centre) / 360.0 + 0.5) * 360.0 - 180.0);
    let chroma = max(max(c.r, c.g), c.b) - min(min(c.r, c.g), c.b);
    return (1.0 - smoothstep(width * 0.5, width, d)) * smoothstep(0.0, 0.25, chroma);
}

/// Every hue turned by `degrees`, brightness held: a rotation in the YIQ plane.
fn hsl_turn(c: vec3<f32>, degrees: f32) -> vec3<f32> {
    let a = radians(degrees);
    let y = dot(c, vec3<f32>(0.299, 0.587, 0.114));
    let i = dot(c, vec3<f32>(0.596, -0.274, -0.322));
    let q = dot(c, vec3<f32>(0.211, -0.523, 0.312));
    let i2 = i * cos(a) - q * sin(a);
    let q2 = i * sin(a) + q * cos(a);
    return vec3<f32>(
        y + 0.956 * i2 + 0.621 * q2,
        y - 0.272 * i2 - 0.647 * q2,
        y - 1.106 * i2 + 1.703 * q2,
    );
}

/// One band: its hue turned, its saturation scaled about luminance and its
/// lightness scaled, weighted by `hsl_mask` so the edges of the band blend.
fn hsl_pick(c: vec3<f32>, centre: f32, width: f32, turn: f32, sat: f32, lum: f32) -> vec3<f32> {
    let w = hsl_mask(c, centre, width);
    var out = c;
    if (turn != 0.0) {
        out = hsl_turn(out, turn);
    }
    let l = dot(out, vec3<f32>(0.2126, 0.7152, 0.0722));
    out = mix(vec3<f32>(l), out, sat) * lum;
    return mix(c, out, w);
}

// What a grading panel's HSL tab does: eight bands of colour, each with its
// hue turned (a hundred is thirty degrees), its saturation scaled (minus a
// hundred is none, a hundred twice) and its lightness scaled (a hundred is
// half as bright again). A band is weighted by how near a pixel's hue is to
// its centre and by how much colour the pixel has, so a grey belongs to no
// band and a sky change stays off a face. Every knob at zero is the picture
// as it came, and light past white is carried on past it.
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let src = sample(uv);
    var c = src.rgb;
    c = hsl_pick(c, 0.0, 60.0, params.red_hue * 0.3, 1.0 + params.red_sat / 100.0, 1.0 + params.red_lum / 200.0);
    c = hsl_pick(c, 30.0, 60.0, params.orange_hue * 0.3, 1.0 + params.orange_sat / 100.0, 1.0 + params.orange_lum / 200.0);
    c = hsl_pick(c, 60.0, 60.0, params.yellow_hue * 0.3, 1.0 + params.yellow_sat / 100.0, 1.0 + params.yellow_lum / 200.0);
    c = hsl_pick(c, 120.0, 60.0, params.green_hue * 0.3, 1.0 + params.green_sat / 100.0, 1.0 + params.green_lum / 200.0);
    c = hsl_pick(c, 180.0, 60.0, params.cyan_hue * 0.3, 1.0 + params.cyan_sat / 100.0, 1.0 + params.cyan_lum / 200.0);
    c = hsl_pick(c, 240.0, 60.0, params.blue_hue * 0.3, 1.0 + params.blue_sat / 100.0, 1.0 + params.blue_lum / 200.0);
    c = hsl_pick(c, 275.0, 60.0, params.purple_hue * 0.3, 1.0 + params.purple_sat / 100.0, 1.0 + params.purple_lum / 200.0);
    c = hsl_pick(c, 320.0, 60.0, params.magenta_hue * 0.3, 1.0 + params.magenta_sat / 100.0, 1.0 + params.magenta_lum / 200.0);
    return vec4<f32>(c, src.a);
}
