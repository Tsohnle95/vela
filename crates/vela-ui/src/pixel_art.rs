//! Match CSS image-rendering:pixelated at the actual device-pixel size. The
//! atlas then paints 1:1, so its linear sampler cannot blur the source pixels.
use gpui::{prelude::*, *};
use std::{
    cell::RefCell,
    sync::{Arc, OnceLock},
};

type SizedImage = (u32, u32, Arc<RenderImage>);
thread_local! {static STATION: RefCell<Option<SizedImage>> = const {RefCell::new(None)};}

pub(crate) fn station(width: f32) -> AnyElement {
    let height = width * 161. / 208.;
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            let scale = window.scale_factor();
            let width = (f32::from(bounds.size.width) * scale).round().max(1.) as u32;
            let height = (f32::from(bounds.size.height) * scale).round().max(1.) as u32;
            let image = STATION.with(|cache| {
                let mut cache = cache.borrow_mut();
                if cache
                    .as_ref()
                    .is_some_and(|(w, h, _)| (*w, *h) == (width, height))
                {
                    return cache.as_ref().unwrap().2.clone();
                }
                static SOURCE: OnceLock<image::RgbaImage> = OnceLock::new();
                let source = SOURCE.get_or_init(|| {
                    image::load_from_memory(include_bytes!("../assets/agent-station.png"))
                        .expect("bundled station image")
                        .to_rgba8()
                });
                let mut pixels = image::imageops::resize(
                    source,
                    width,
                    height,
                    image::imageops::FilterType::Nearest,
                );
                // RenderImage's atlas format is BGRA; the PNG decoder returns RGBA.
                for pixel in pixels.pixels_mut() {
                    pixel.0.swap(0, 2);
                }
                let image = Arc::new(RenderImage::new([image::Frame::new(pixels)]));
                if let Some((_, _, old)) = cache.replace((width, height, image.clone())) {
                    cx.drop_image(old, Some(window));
                }
                image
            });
            if let Err(error) = window.paint_image(bounds, Corners::default(), image, 0, false) {
                tracing::warn!(%error,"could not paint station artwork");
            }
        },
    )
    .w(px(width))
    .h(px(height))
    .flex_shrink_0()
    .into_any_element()
}
