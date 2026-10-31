struct Params { punch: f32, bpm: f32 }

// Rhythmic beat-synced radial zoom and RGB split: pulses with the beat phase
// derived from `frame.time` and `params.bpm`. At `frame.time == 0` the pulse
// envelope starts cleanly and on a flat field returns the exact input colour.
fn effect(uv: vec2<f32>) -> vec4<f32> {
    let base = sample(uv);
    let strength = params.punch * 0.01;
    if (strength <= 0.0) {
        return base;
    }

    let beats = frame.time * (params.bpm / 60.0);
    let phase = fract(beats);
    let kick = exp(-phase * 6.0) * sin(phase * 3.14159265);
    let zoom = 1.0 + strength * 0.22 * kick;

    let centered = (uv - vec2<f32>(0.5)) / zoom;
    let split = strength * 0.012 * kick;

    let r = sample(vec2<f32>(0.5) + centered * (1.0 + split)).r;
    let g = sample(vec2<f32>(0.5) + centered).g;
    let b = sample(vec2<f32>(0.5) + centered * (1.0 - split)).b;

    return vec4<f32>(r, g, b, base.a);
}
