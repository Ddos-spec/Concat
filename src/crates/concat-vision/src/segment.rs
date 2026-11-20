// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer Abdullah and Concat contributors

//! Finding the subject in a picture.
//!
//! Three models, one interface: a frame in, a probability per pixel out.
//!
//! - **Selfie**: MediaPipe's selfie segmentation, compiled into the binary
//!   (see `models/NOTICE.md`). A person, at 256 × 256, in a few
//!   milliseconds; the answer when nothing has been downloaded.
//! - **Person**: Robust Video Matting on MobileNetV3. A person with a true
//!   alpha edge - hair, motion blur - at the picture's own aspect, carrying
//!   a recurrent state from one frame to the next so an outline does not
//!   flicker. Frames must therefore arrive in order: [`Segmenter::begin`]
//!   starts a run.
//! - **Object**: IS-Net. The one thing the picture is of, whatever it is,
//!   at 1024 × 1024; the answer for a car, a product, a cartoon.
//!
//! A source of any shape is resampled to the model's size on the way in
//! and the mask is read as covering the whole source on the way out, which
//! is the convention every other part of this crate keeps.

use std::path::Path;
use std::sync::Mutex;

use concat_core::frame::{BYTES_PER_PIXEL, Frame};

use crate::models::ModelId;
use crate::runtime::{Input, Model, Output};
use crate::{MODEL_ID, MODEL_SIZE, Mask};

/// The compiled-in model.
const SELFIE: &[u8] = include_bytes!("../models/selfie-segmentation.onnx");

/// The person model's long edge: its stage one works at up to 512, and a
/// larger frame would only be shrunk inside it.
const PERSON_EDGE: u32 = 512;
/// The object model's fixed input.
const OBJECT_SIZE: u32 = 1024;
/// The object model's answer is stored at half its size: a quarter of the
/// bytes, and still twice the person model's.
const OBJECT_SHRINK: u32 = 2;

/// Which model a segmenter runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The compiled-in person model.
    Selfie,
    /// The downloaded person matting model.
    Person,
    /// The downloaded object model.
    Object,
}

/// A loaded model, ready to run. `Send + Sync`; the runtime's own threads
/// take a frame across the cores, so one per process is enough.
pub struct Segmenter {
    kind: Kind,
    model: Mutex<Model>,
    /// The person model's recurrent state from the frame before, within
    /// one run.
    state: Mutex<Option<Vec<Output>>>,
}

impl Segmenter {
    /// The compiled-in person model. A few tens of milliseconds, once.
    pub fn selfie() -> Result<Segmenter, String> {
        Ok(Segmenter {
            kind: Kind::Selfie,
            model: Mutex::new(Model::from_bytes(SELFIE)?),
            state: Mutex::new(None),
        })
    }

    /// The compiled-in model; see [`Segmenter::selfie`].
    pub fn load() -> Result<Segmenter, String> {
        Segmenter::selfie()
    }

    /// The person matting model, from its downloaded file.
    pub fn person(file: &Path) -> Result<Segmenter, String> {
        Ok(Segmenter {
            kind: Kind::Person,
            model: Mutex::new(Model::from_file(file)?),
            state: Mutex::new(None),
        })
    }

    /// The object model, from its downloaded file.
    pub fn object(file: &Path) -> Result<Segmenter, String> {
        Ok(Segmenter {
            kind: Kind::Object,
            model: Mutex::new(Model::from_file(file)?),
            state: Mutex::new(None),
        })
    }

    /// Which model this is.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The name a mask store records for this model's masks.
    pub fn name(&self) -> &'static str {
        match self.kind {
            Kind::Selfie => MODEL_ID,
            Kind::Person => ModelId::Person.name(),
            Kind::Object => ModelId::Object.name(),
        }
    }

    /// The size to decode a source of `width` × `height` to for this
    /// model: its fixed square, or the source's shape at the person model's
    /// working edge.
    pub fn input_size(&self, width: u32, height: u32) -> (u32, u32) {
        match self.kind {
            Kind::Selfie => (MODEL_SIZE, MODEL_SIZE),
            Kind::Object => (OBJECT_SIZE, OBJECT_SIZE),
            Kind::Person => {
                let (width, height) = (width.max(1) as f64, height.max(1) as f64);
                let scale = f64::from(PERSON_EDGE) / width.max(height);
                // Multiples of sixteen, so every stride inside the network
                // divides evenly and nothing is padded off the edge.
                let round = |edge: f64| (((edge * scale) / 16.0).round() as u32 * 16).max(64);
                (round(width), round(height))
            }
        }
    }

    /// Starts a run of frames in source order. The person model forgets
    /// what it carried from the frame before; the others carry nothing.
    pub fn begin(&self) {
        if let Ok(mut state) = self.state.lock() {
            *state = None;
        }
    }

    /// The subject mask of `frame`. A frame of any size: it is resampled
    /// to the model's size first, so the decoder is best asked for
    /// [`Segmenter::input_size`] and the resample is a copy.
    pub fn mask(&self, frame: &Frame) -> Result<Mask, String> {
        if frame.width() == 0 || frame.height() == 0 {
            return Err("cutout: an empty frame".to_owned());
        }
        let (width, height) = self.input_size(frame.width(), frame.height());
        let planes = planes(frame, width, height);
        match self.kind {
            Kind::Selfie => self.selfie_mask(planes, width, height),
            Kind::Object => self.object_mask(planes, width, height),
            Kind::Person => self.person_mask(planes, width, height),
        }
    }

    fn run(&self, inputs: Vec<Input<'_>>, outputs: &[&str]) -> Result<Vec<Output>, String> {
        self.model
            .lock()
            .map_err(|_| "cutout model poisoned".to_owned())?
            .run(inputs, outputs)
    }

    fn selfie_mask(&self, planes: Vec<f32>, width: u32, height: u32) -> Result<Mask, String> {
        let size = (width * height) as usize;
        let input = Input {
            name: "pixel_values",
            dims: vec![1, 3, height as usize, width as usize],
            data: planes.into(),
        };
        let outputs = self.run(vec![input], &["alphas"])?;
        probabilities(&outputs[0].data, size, width, height)
    }

    fn object_mask(&self, mut planes: Vec<f32>, width: u32, height: u32) -> Result<Mask, String> {
        // As the model was trained: the picture scaled by its brightest
        // value, then centred on a half.
        let brightest = planes.iter().copied().fold(0.0f32, f32::max).max(1e-6);
        for value in &mut planes {
            *value = *value / brightest - 0.5;
        }
        let size = (width * height) as usize;
        let input = Input {
            name: "input_image",
            dims: vec![1, 3, height as usize, width as usize],
            data: planes.into(),
        };
        let outputs = self.run(vec![input], &["output_image"])?;
        // The model answers in its own range; the span of the answer is
        // the mask, as the reference implementation reads it.
        let data = &outputs[0].data[..size.min(outputs[0].data.len())];
        let low = data.iter().copied().fold(f32::INFINITY, f32::min);
        let high = data.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let span = (high - low).max(1e-6);
        let bytes: Vec<u8> = data
            .iter()
            .map(|&p| (((p - low) / span).clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
            .collect();
        let mask = Mask::from_bytes(width, height, bytes)
            .ok_or_else(|| "cutout: the model's answer is the wrong size".to_owned())?;
        Ok(mask.shrunk(OBJECT_SHRINK))
    }

    fn person_mask(&self, planes: Vec<f32>, width: u32, height: u32) -> Result<Mask, String> {
        let size = (width * height) as usize;
        let mut inputs = vec![Input {
            name: "src",
            dims: vec![1, 3, height as usize, width as usize],
            data: planes.into(),
        }];
        let carried = self
            .state
            .lock()
            .map_err(|_| "cutout state poisoned".to_owned())?
            .take();
        const STATES: [&str; 4] = ["r1i", "r2i", "r3i", "r4i"];
        match carried {
            Some(state) if state.len() == 4 => {
                for (name, output) in STATES.iter().zip(state) {
                    inputs.push(Input {
                        name,
                        dims: output.dims,
                        data: output.data.into(),
                    });
                }
            }
            _ => {
                for name in STATES {
                    inputs.push(Input {
                        name,
                        dims: vec![1, 1, 1, 1],
                        data: vec![0.0f32].into(),
                    });
                }
            }
        }
        // The frame is already at the working size; the model's own
        // downsample stays at one.
        inputs.push(Input {
            name: "downsample_ratio",
            dims: vec![1],
            data: vec![1.0f32].into(),
        });
        let mut outputs = self.run(inputs, &["pha", "r1o", "r2o", "r3o", "r4o"])?;
        let alpha = outputs.remove(0);
        if let Ok(mut state) = self.state.lock() {
            *state = Some(outputs);
        }
        probabilities(&alpha.data, size, width, height)
    }
}

/// The frame as the models read it: three planes of `0..=1`, red then
/// green then blue, at `width` × `height`, nearest-sampled from the frame.
fn planes(frame: &Frame, width: u32, height: u32) -> Vec<f32> {
    let (fw, fh) = (frame.width() as usize, frame.height() as usize);
    let (width, height) = (width as usize, height as usize);
    let pixels = frame.pixels();
    let mut data = vec![0f32; 3 * width * height];
    for c in 0..3 {
        for y in 0..height {
            let sy = (y * fh / height).min(fh - 1);
            for x in 0..width {
                let sx = (x * fw / width).min(fw - 1);
                data[(c * height + y) * width + x] =
                    f32::from(pixels[(sy * fw + sx) * BYTES_PER_PIXEL + c]) / 255.0;
            }
        }
    }
    data
}

/// A model's `0..=1` answer as a mask.
fn probabilities(data: &[f32], size: usize, width: u32, height: u32) -> Result<Mask, String> {
    let bytes: Vec<u8> = data
        .iter()
        .take(size)
        .map(|&p| (p.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
        .collect();
    Mask::from_bytes(width, height, bytes)
        .ok_or_else(|| "cutout: the model's answer is the wrong size".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_model_loads_and_answers_a_mask_of_its_size() {
        let segmenter = Segmenter::selfie().expect("loads");
        // A flat grey picture is nobody: the mask should lean background.
        let mut frame = Frame::black(64, 48);
        for pixel in frame.pixels_mut().chunks_exact_mut(4) {
            pixel[..3].copy_from_slice(&[128, 128, 128]);
        }
        let mask = segmenter.mask(&frame).expect("runs");
        assert_eq!((mask.width(), mask.height()), (MODEL_SIZE, MODEL_SIZE));
        let mean = mask.bytes().iter().map(|&v| u32::from(v)).sum::<u32>() / (256 * 256);
        assert!(mean < 128, "a blank picture read as mostly person: {mean}");
    }

    #[test]
    fn the_person_model_works_at_the_pictures_shape() {
        let segmenter = Segmenter {
            kind: Kind::Person,
            model: Mutex::new(Model::from_bytes(SELFIE).expect("loads")),
            state: Mutex::new(None),
        };
        assert_eq!(segmenter.input_size(1920, 1080), (512, 288));
        assert_eq!(segmenter.input_size(1080, 1920), (288, 512));
        assert_eq!(segmenter.input_size(1000, 1000), (512, 512));
    }
}


#[cfg(test)]
mod real_person {
    use super::*;
    use concat_core::frame::Frame;

    fn frame_from(path: &str, size: &str) -> Frame {
        let (w, h) = size.split_once('x').expect("WIDTHxHEIGHT");
        let (w, h): (u32, u32) = (w.parse().expect("width"), h.parse().expect("height"));
        let rgba = std::fs::read(path).expect("the raw RGBA file");
        Frame::from_rgba(w, h, rgba).expect("a frame of that size")
    }

    fn covered(mask: &Mask) -> f64 {
        let lit = mask.bytes().iter().filter(|v| **v >= 128).count();
        lit as f64 / mask.bytes().len().max(1) as f64
    }

    /// The person model that ships inside the program, run on a real
    /// photograph of a person (NASA's portrait of an astronaut, public
    /// domain) and on a flat grey picture with nobody in it. Nothing is
    /// downloaded and nothing leaves the machine. The mask is written out
    /// so it can be looked at. Ignored: it wants the picture.
    ///
    ///   CONCAT_PERSON_RGBA=astro.rgba CONCAT_PERSON_SIZE=512x512 CONCAT_MASK_OUT=mask.png \
    ///     cargo test -p concat-vision --features infer --lib -- --ignored the_built_in_model_finds_a_person --nocapture
    #[test]
    #[ignore = "needs CONCAT_PERSON_RGBA and CONCAT_PERSON_SIZE"]
    fn the_built_in_model_finds_a_person() {
        let (Ok(path), Ok(size)) = (
            std::env::var("CONCAT_PERSON_RGBA"),
            std::env::var("CONCAT_PERSON_SIZE"),
        ) else {
            panic!("CONCAT_PERSON_RGBA and CONCAT_PERSON_SIZE name the picture");
        };
        let photo = frame_from(&path, &size);
        let segmenter = Segmenter::selfie().expect("the compiled-in model loads");
        segmenter.begin();
        let mask = segmenter.mask(&photo).expect("the photo is segmented");
        let share = covered(&mask);
        // Where the lit part of the mask sits, as a share of the width and height.
        let (mut sx, mut sy, mut n) = (0.0f64, 0.0f64, 0.0f64);
        for y in 0..mask.height() {
            for x in 0..mask.width() {
                if mask.get(x, y) >= 128 {
                    sx += f64::from(x);
                    sy += f64::from(y);
                    n += 1.0;
                }
            }
        }
        let centre = (sx / n.max(1.0) / f64::from(mask.width()), sy / n.max(1.0) / f64::from(mask.height()));
        eprintln!(
            "photo {}x{}: mask {}x{}, person covers {:.1}% of it, centred at ({:.2}, {:.2})",
            photo.width(), photo.height(), mask.width(), mask.height(), share * 100.0, centre.0, centre.1
        );
        if let Ok(out) = std::env::var("CONCAT_MASK_OUT") {
            std::fs::write(&out, mask.to_png()).expect("the mask is written");
        }
        assert!((0.08..=0.75).contains(&share), "a person fills some of the picture, not {share}");
        assert!((0.2..=0.8).contains(&centre.0) && (0.2..=0.8).contains(&centre.1), "centred on the subject: {centre:?}");

        // Nobody in a flat grey picture.
        let empty = Frame::from_rgba(photo.width(), photo.height(), [128u8, 128, 128, 255].repeat((photo.width() * photo.height()) as usize))
            .expect("a grey frame");
        segmenter.begin();
        let none = covered(&segmenter.mask(&empty).expect("segments"));
        eprintln!("flat grey picture: {:.1}% covered", none * 100.0);
        assert!(none < 0.05, "no person in a grey picture, but {none} was found");
    }
}
