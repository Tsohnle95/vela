//! Browser-compatible image URI conversion, without a Referer header.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use gpui::{Image, ImageFormat, ImageSource};
use std::{path::PathBuf, sync::Arc};

pub(super) fn source(uri: &str) -> Result<ImageSource, ()> {
    if let Some(data) = uri.strip_prefix("data:") {
        let (metadata, payload) = data.split_once(',').ok_or(())?;
        let mime = metadata.split(';').next().unwrap_or_default();
        let format = ImageFormat::from_mime_type(mime).ok_or(())?;
        let bytes = if metadata
            .split(';')
            .any(|value| value.eq_ignore_ascii_case("base64"))
        {
            let decoded = percent_encoding::percent_decode_str(payload).collect::<Vec<_>>();
            STANDARD.decode(decoded).map_err(|_| ())?
        } else {
            percent_encoding::percent_decode_str(payload).collect()
        };
        return Ok(ImageSource::from(Arc::new(Image::from_bytes(
            format, bytes,
        ))));
    }
    if let Ok(url) = url::Url::parse(uri) {
        return match url.scheme() {
            // Match the reference renderer CSP: img-src self data:.
            "http" | "https" => Err(()),
            "file" => url.to_file_path().map(ImageSource::from).map_err(|_| ()),
            _ => Err(()),
        };
    }
    if uri.starts_with('/') {
        Ok(ImageSource::from(PathBuf::from(
            percent_encoding::percent_decode_str(uri)
                .decode_utf8_lossy()
                .into_owned(),
        )))
    } else {
        Ok(ImageSource::from(uri.to_owned()))
    }
}

struct DataImageDecoder;
impl gpui::Asset for DataImageDecoder {
    type Source = Arc<Image>;
    type Output = Result<Arc<gpui::RenderImage>, ()>;
    fn load(
        image: Self::Source,
        cx: &mut gpui::App,
    ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
        let renderer = cx.svg_renderer();
        async move { image.to_image_data(renderer).map_err(|_| ()) }
    }
}
pub(super) fn resolve(
    source: &ImageSource,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> Option<Result<Arc<gpui::RenderImage>, ()>> {
    match source {
        ImageSource::Image(image) => window.use_asset::<DataImageDecoder>(image, cx),
        ImageSource::Resource(resource) => window
            .use_asset::<gpui::ImageAssetLoader>(resource, cx)
            .map(|image| image.map_err(|_| ())),
        ImageSource::Render(image) => Some(Ok(image.clone())),
        ImageSource::Custom(load) => load(window, cx).map(|image| image.map_err(|_| ())),
    }
}
pub(super) fn fit(width: f32, height: f32) -> gpui::Size<gpui::Pixels> {
    if width <= 0. || height <= 0. {
        return gpui::size(gpui::px(0.), gpui::px(0.));
    }
    // CSS max-width/max-height constrain the border box. The replaced
    // image's aspect ratio belongs to its content, inside the two 1px edges.
    let scale = (198. / width).min(198. / height).min(1.);
    gpui::size(gpui::px(width * scale + 2.), gpui::px(height * scale + 2.))
}

pub(super) fn broken(window: &gpui::Window) -> Arc<Image> {
    static LOW: std::sync::OnceLock<Arc<Image>> = std::sync::OnceLock::new();
    static HIGH: std::sync::OnceLock<Arc<Image>> = std::sync::OnceLock::new();
    if window.scale_factor() >= 2. {
        HIGH.get_or_init(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../assets/chromium/broken-image@2x.png").to_vec(),
            ))
        })
        .clone()
    } else {
        LOW.get_or_init(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../assets/chromium/broken-image@1x.png").to_vec(),
            ))
        })
        .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intrinsic_images_fit_both_axes_without_upscaling() {
        assert_eq!(
            super::fit(400., 100.),
            gpui::size(gpui::px(200.), gpui::px(51.5))
        );
        assert_eq!(
            super::fit(100., 400.),
            gpui::size(gpui::px(51.5), gpui::px(200.))
        );
        assert_eq!(
            super::fit(16., 12.),
            gpui::size(gpui::px(18.), gpui::px(14.))
        );
    }
    #[test]
    fn images_preserve_encoded_files_svg_data_and_reference_csp() {
        let file = source("file:///private/tmp/image%20%F0%9F%8C%8D.png").unwrap();
        assert!(
            matches!(file,ImageSource::Resource(gpui::Resource::Path(path)) if path.as_ref()==std::path::Path::new("/private/tmp/image 🌍.png"))
        );
        assert!(source("https://example.com/a%20b.png?q=%23").is_err());
        for uri in [
            "data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%2F%3E",
            "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciLz4=",
        ] {
            let ImageSource::Image(image) = source(uri).unwrap() else {
                panic!("expected decoded embedded image");
            };
            assert_eq!(image.format, ImageFormat::Svg);
            assert_eq!(image.bytes, b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
        }
        assert!(source("javascript:alert(1)").is_err());
        assert!(source("data:image/png;base64,not-base64").is_err());
    }
}
