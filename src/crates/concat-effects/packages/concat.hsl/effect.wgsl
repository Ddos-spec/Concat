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
    c = hsl_band(c, 0.0, 60.0, params.red_hue * 0.3, 1.0 + params.red_sat / 100.0, 1.0 + params.red_lum / 200.0);
    c = hsl_band(c, 30.0, 60.0, params.orange_hue * 0.3, 1.0 + params.orange_sat / 100.0, 1.0 + params.orange_lum / 200.0);
    c = hsl_band(c, 60.0, 60.0, params.yellow_hue * 0.3, 1.0 + params.yellow_sat / 100.0, 1.0 + params.yellow_lum / 200.0);
    c = hsl_band(c, 120.0, 60.0, params.green_hue * 0.3, 1.0 + params.green_sat / 100.0, 1.0 + params.green_lum / 200.0);
    c = hsl_band(c, 180.0, 60.0, params.cyan_hue * 0.3, 1.0 + params.cyan_sat / 100.0, 1.0 + params.cyan_lum / 200.0);
    c = hsl_band(c, 240.0, 60.0, params.blue_hue * 0.3, 1.0 + params.blue_sat / 100.0, 1.0 + params.blue_lum / 200.0);
    c = hsl_band(c, 275.0, 60.0, params.purple_hue * 0.3, 1.0 + params.purple_sat / 100.0, 1.0 + params.purple_lum / 200.0);
    c = hsl_band(c, 320.0, 60.0, params.magenta_hue * 0.3, 1.0 + params.magenta_sat / 100.0, 1.0 + params.magenta_lum / 200.0);
    return vec4<f32>(c, src.a);
}
