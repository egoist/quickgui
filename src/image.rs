use std::{
    fmt,
    hash::{Hash, Hasher},
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use thiserror::Error;

use crate::{
    Rect, Size,
    animated_image::{AnimatedImage, ImageAsset},
};

/// Largest accepted width or height for a decoded image.
pub const MAX_IMAGE_DIMENSION: u32 = 4096;
/// Largest decoded RGBA allocation owned by one [`Image`].
pub const MAX_DECODED_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
/// Largest encoded file accepted by [`Image::open`].
pub const MAX_ENCODED_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
/// Largest `data:` URL accepted by [`Image::from_data_url`], including its header.
///
/// Base64 expands payloads by four thirds, so this is the transport bound that corresponds to
/// [`MAX_ENCODED_IMAGE_BYTES`].
pub const MAX_IMAGE_DATA_URL_BYTES: usize = 88 * 1024 * 1024;
/// Largest number of additional scale representations retained by one [`Image`].
pub const MAX_IMAGE_REPRESENTATIONS: usize = 8;
/// Smallest accepted representation scale factor.
pub const MIN_IMAGE_REPRESENTATION_SCALE: f32 = 0.25;
/// Largest accepted representation scale factor.
pub const MAX_IMAGE_REPRESENTATION_SCALE: f32 = 16.0;
/// Default point size used by [`Image::named_system`].
pub const DEFAULT_SYSTEM_IMAGE_POINT_SIZE: f32 = 16.0;
/// Default backing scale used by [`Image::named_system`].
pub const DEFAULT_SYSTEM_IMAGE_SCALE: f32 = 2.0;

static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_CUSTOM_RESOURCE_ID: AtomicU64 = AtomicU64::new(1);

/// Whether `path` uses Apple's template-image filename convention.
///
/// A stem that ends in `Template`, optionally followed by a scale suffix such as `@2x`, is treated
/// as a monochrome mask. `trayTemplate.png` and `statusTemplate@2x.png` match; `emailTemplateIcon.png`
/// does not.
pub fn is_template_image_path(path: impl AsRef<Path>) -> bool {
    let Some(stem) = path.as_ref().file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    template_image_stem(stem).ends_with("Template")
}

fn template_image_stem(stem: &str) -> &str {
    let Some((base, suffix)) = stem.rsplit_once('@') else {
        return stem;
    };
    if suffix.len() >= 2
        && suffix.ends_with('x')
        && suffix[..suffix.len() - 1]
            .bytes()
            .all(|b| b.is_ascii_digit())
    {
        return base;
    }
    stem
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ImageId(u64);

/// Immutable, cheap-to-clone RGBA image data.
///
/// Clones share both identity and pixels. That identity lets every window reuse one GPU texture
/// until the renderer's bounded cache needs the space for a newer visible image.
#[derive(Clone)]
pub struct Image(Arc<ImageData>);

struct ImageData {
    id: ImageId,
    width: u32,
    height: u32,
    rgba: Arc<[u8]>,
    template: bool,
    representations: Arc<[(f32, Image)]>,
}

impl Image {
    /// Construct an image from tightly packed, straight-alpha RGBA8 pixels.
    pub fn from_rgba(
        width: u32,
        height: u32,
        rgba: impl Into<Arc<[u8]>>,
    ) -> Result<Self, ImageError> {
        validate_dimensions(width, height)?;
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(ImageError::TooLarge {
                bytes: u64::MAX,
                maximum: MAX_DECODED_IMAGE_BYTES,
            })?;
        if expected > MAX_DECODED_IMAGE_BYTES {
            return Err(ImageError::TooLarge {
                bytes: expected,
                maximum: MAX_DECODED_IMAGE_BYTES,
            });
        }
        let rgba = rgba.into();
        if rgba.len() as u64 != expected {
            return Err(ImageError::InvalidPixelLength {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self::from_parts(width, height, rgba, false, Arc::from([])))
    }

    fn from_parts(
        width: u32,
        height: u32,
        rgba: Arc<[u8]>,
        template: bool,
        representations: Arc<[(f32, Image)]>,
    ) -> Self {
        let id = ImageId(NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed));
        Self(Arc::new(ImageData {
            id,
            width,
            height,
            rgba,
            template,
            representations,
        }))
    }

    /// Decode PNG, JPEG, TIFF, WebP, or the first frame of a GIF from memory.
    ///
    /// Decoding is explicit and synchronous. Applications should perform it away from latency-
    /// sensitive input handling and then publish the resulting cheap-to-clone `Image`.
    pub fn decode(encoded: impl AsRef<[u8]>) -> Result<Self, ImageError> {
        let mut reader = image_codecs::ImageReader::new(Cursor::new(encoded.as_ref()))
            .with_guessed_format()
            .map_err(ImageError::Inspect)?;
        let mut limits = image_codecs::Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
        limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
        limits.max_alloc = Some(MAX_DECODED_IMAGE_BYTES);
        reader.limits(limits);
        let decoded = reader.decode().map_err(ImageError::Decode)?.into_rgba8();
        Self::from_rgba(decoded.width(), decoded.height(), decoded.into_raw())
    }

    /// Decode a supported image file synchronously with encoded and decoded size limits.
    ///
    /// Prefer passing a path to [`crate::img`] for event-driven background loading. This method is
    /// useful when an application already owns a background execution context.
    ///
    /// Paths whose stem ends in `Template`, optionally followed by a scale suffix such as `@2x`,
    /// are marked for macOS template rendering. Call [`Self::template`] to override that
    /// convention.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path).map_err(|source| ImageError::Open {
            path: path.to_path_buf(),
            source,
        })?;
        if metadata.len() > MAX_ENCODED_IMAGE_BYTES {
            return Err(ImageError::EncodedTooLarge {
                bytes: metadata.len(),
                maximum: MAX_ENCODED_IMAGE_BYTES,
            });
        }
        let mut reader =
            image_codecs::ImageReader::open(path).map_err(|source| ImageError::Open {
                path: path.to_path_buf(),
                source,
            })?;
        let mut limits = image_codecs::Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
        limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
        limits.max_alloc = Some(MAX_DECODED_IMAGE_BYTES);
        reader.limits(limits);
        let decoded = reader.decode().map_err(ImageError::Decode)?.into_rgba8();
        Ok(
            Self::from_rgba(decoded.width(), decoded.height(), decoded.into_raw())?
                .template(is_template_image_path(path)),
        )
    }

    pub fn width(&self) -> u32 {
        self.0.width
    }

    pub fn height(&self) -> u32 {
        self.0.height
    }

    pub fn size(&self) -> Size {
        Size::new(self.width() as f32, self.height() as f32)
    }

    pub fn byte_len(&self) -> usize {
        self.0.rgba.len()
    }

    pub(crate) fn id(&self) -> ImageId {
        self.0.id
    }

    /// Tightly packed, straight-alpha RGBA8 pixels.
    pub fn rgba(&self) -> &[u8] {
        &self.0.rgba
    }

    /// Decode a base64 `data:` URL carrying PNG, JPEG, GIF, or WebP bytes.
    ///
    /// Only base64 payloads are accepted; percent-encoded `data:` URLs are rejected because the
    /// binary formats QuickGUI decodes are never sent that way in practice. The URL text is
    /// bounded by [`MAX_IMAGE_DATA_URL_BYTES`] and the decoded payload by the same limits as
    /// [`Image::decode`].
    pub fn from_data_url(url: &str) -> Result<Self, ImageError> {
        if url.len() > MAX_IMAGE_DATA_URL_BYTES {
            return Err(ImageError::EncodedTooLarge {
                bytes: url.len() as u64,
                maximum: MAX_IMAGE_DATA_URL_BYTES as u64,
            });
        }
        let rest = url
            .strip_prefix("data:")
            .or_else(|| url.strip_prefix("DATA:"))
            .ok_or(ImageError::InvalidDataUrl)?;
        let (header, payload) = rest.split_once(',').ok_or(ImageError::InvalidDataUrl)?;
        if !header
            .rsplit(';')
            .next()
            .is_some_and(|encoding| encoding.eq_ignore_ascii_case("base64"))
        {
            return Err(ImageError::InvalidDataUrl);
        }
        let decoded = decode_base64(payload)?;
        if decoded.len() as u64 > MAX_ENCODED_IMAGE_BYTES {
            return Err(ImageError::EncodedTooLarge {
                bytes: decoded.len() as u64,
                maximum: MAX_ENCODED_IMAGE_BYTES,
            });
        }
        Self::decode(decoded)
    }

    /// Rasterize a system-provided image at [`DEFAULT_SYSTEM_IMAGE_POINT_SIZE`] and
    /// [`DEFAULT_SYSTEM_IMAGE_SCALE`].
    ///
    /// On macOS this resolves an `NSImage` name first and then an SF Symbol name. Other platforms
    /// report [`ImageError::SystemImageUnavailable`].
    pub fn named_system(name: &str) -> Result<Self, ImageError> {
        Self::named_system_sized(
            name,
            DEFAULT_SYSTEM_IMAGE_POINT_SIZE,
            DEFAULT_SYSTEM_IMAGE_SCALE,
        )
    }

    /// Rasterize a system-provided image at a requested point size and backing scale.
    ///
    /// The resulting bitmap is `round(point_size * scale)` pixels on its longest axis.
    pub fn named_system_sized(name: &str, point_size: f32, scale: f32) -> Result<Self, ImageError> {
        if name.is_empty() || name.contains('\0') {
            return Err(ImageError::SystemImageUnavailable {
                name: name.to_owned(),
            });
        }
        if !point_size.is_finite() || point_size <= 0.0 || !scale.is_finite() || scale <= 0.0 {
            return Err(ImageError::EmptyDimensions {
                width: point_size as u32,
                height: scale as u32,
            });
        }
        let pixels = (f64::from(point_size) * f64::from(scale)).round();
        if !(1.0..=f64::from(MAX_IMAGE_DIMENSION)).contains(&pixels) {
            return Err(ImageError::DimensionsTooLarge {
                width: pixels as u32,
                height: pixels as u32,
                maximum: MAX_IMAGE_DIMENSION,
            });
        }
        #[cfg(target_os = "macos")]
        {
            crate::macos::system_image(name, f64::from(point_size), f64::from(scale))
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(ImageError::SystemImageUnavailable {
                name: name.to_owned(),
            })
        }
    }

    /// Return a copy marked as a macOS template image.
    ///
    /// A template image is recolored by AppKit for the menu bar's light, dark, and highlighted
    /// appearances using only its alpha channel. The returned value has a fresh identity so a
    /// renderer cache never confuses it with the untemplated original; the pixels are shared.
    pub fn template(&self, template: bool) -> Self {
        if self.0.template == template {
            return self.clone();
        }
        Self::from_parts(
            self.0.width,
            self.0.height,
            self.0.rgba.clone(),
            template,
            self.0.representations.clone(),
        )
    }

    /// Whether this image is marked for macOS template rendering.
    pub fn is_template(&self) -> bool {
        self.0.template
    }

    /// Attach additional backing-scale representations used when building a native image.
    ///
    /// The receiver is the 1x representation. At most [`MAX_IMAGE_REPRESENTATIONS`] variants are
    /// retained and each scale must lie between [`MIN_IMAGE_REPRESENTATION_SCALE`] and
    /// [`MAX_IMAGE_REPRESENTATION_SCALE`].
    pub fn with_representations(
        &self,
        representations: impl IntoIterator<Item = (f32, Self)>,
    ) -> Result<Self, ImageError> {
        let representations = representations.into_iter().collect::<Vec<_>>();
        if representations.len() > MAX_IMAGE_REPRESENTATIONS {
            return Err(ImageError::TooManyRepresentations {
                representations: representations.len(),
                maximum: MAX_IMAGE_REPRESENTATIONS,
            });
        }
        if representations.iter().any(|(scale, _)| {
            !scale.is_finite()
                || !(MIN_IMAGE_REPRESENTATION_SCALE..=MAX_IMAGE_REPRESENTATION_SCALE)
                    .contains(scale)
        }) {
            return Err(ImageError::InvalidRepresentationScale);
        }
        Ok(Self::from_parts(
            self.0.width,
            self.0.height,
            self.0.rgba.clone(),
            self.0.template,
            representations.into(),
        ))
    }

    /// Additional backing-scale representations attached to this image.
    pub fn representations(&self) -> &[(f32, Self)] {
        &self.0.representations
    }

    /// Resample this image to exact pixel dimensions with a bilinear filter.
    pub fn resize(&self, width: u32, height: u32) -> Result<Self, ImageError> {
        validate_dimensions(width, height)?;
        if width == self.0.width && height == self.0.height {
            return Ok(self.clone());
        }
        let source = self.rgba_buffer()?;
        let resized = image_codecs::imageops::resize(
            &source,
            width,
            height,
            image_codecs::imageops::FilterType::Triangle,
        );
        Self::from_rgba(width, height, resized.into_raw())
    }

    /// Copy a pixel rectangle out of this image.
    ///
    /// The rectangle is rounded to whole pixels and must lie inside the image with a non-empty
    /// area.
    pub fn crop(&self, bounds: Rect) -> Result<Self, ImageError> {
        if !bounds.x.is_finite()
            || !bounds.y.is_finite()
            || !bounds.width.is_finite()
            || !bounds.height.is_finite()
            || bounds.x < 0.0
            || bounds.y < 0.0
        {
            return Err(ImageError::InvalidCrop);
        }
        let x = bounds.x.round() as u32;
        let y = bounds.y.round() as u32;
        let width = bounds.width.round() as u32;
        let height = bounds.height.round() as u32;
        if width == 0
            || height == 0
            || x.saturating_add(width) > self.0.width
            || y.saturating_add(height) > self.0.height
        {
            return Err(ImageError::InvalidCrop);
        }
        let mut pixels = Vec::with_capacity((width as usize) * (height as usize) * 4);
        let stride = self.0.width as usize * 4;
        for row in 0..height as usize {
            let start = (y as usize + row) * stride + x as usize * 4;
            pixels.extend_from_slice(&self.0.rgba[start..start + width as usize * 4]);
        }
        Self::from_rgba(width, height, pixels)
    }

    /// Encode this image as PNG with its alpha channel intact.
    pub fn to_png(&self) -> Result<Vec<u8>, ImageError> {
        use image_codecs::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(
                self.rgba(),
                self.0.width,
                self.0.height,
                ExtendedColorType::Rgba8,
            )
            .map_err(ImageError::Encode)?;
        Ok(encoded)
    }

    /// Encode this image as JPEG at `quality` (1-100), compositing away the alpha channel.
    ///
    /// JPEG has no alpha, so translucent pixels are composited over opaque white.
    pub fn to_jpeg(&self, quality: u8) -> Result<Vec<u8>, ImageError> {
        use image_codecs::{ExtendedColorType, ImageEncoder, codecs::jpeg::JpegEncoder};

        if quality == 0 || quality > 100 {
            return Err(ImageError::InvalidJpegQuality { quality });
        }
        let mut rgb = Vec::with_capacity((self.0.rgba.len() / 4) * 3);
        for pixel in self.0.rgba.as_chunks::<4>().0 {
            let alpha = f32::from(pixel[3]) / 255.0;
            for channel in &pixel[..3] {
                let value = f32::from(*channel) * alpha + 255.0 * (1.0 - alpha);
                rgb.push(value.round().clamp(0.0, 255.0) as u8);
            }
        }
        let mut encoded = Vec::new();
        JpegEncoder::new_with_quality(&mut encoded, quality)
            .write_image(&rgb, self.0.width, self.0.height, ExtendedColorType::Rgb8)
            .map_err(ImageError::Encode)?;
        Ok(encoded)
    }

    fn rgba_buffer(&self) -> Result<image_codecs::RgbaImage, ImageError> {
        image_codecs::RgbaImage::from_raw(self.0.width, self.0.height, self.0.rgba.to_vec()).ok_or(
            ImageError::InvalidPixelLength {
                expected: u64::from(self.0.width) * u64::from(self.0.height) * 4,
                actual: self.0.rgba.len(),
            },
        )
    }
}

type CustomImageLoader = dyn Fn() -> Result<Image, Arc<str>> + Send + Sync;
type CustomAnimatedImageLoader = dyn Fn() -> Result<AnimatedImage, Arc<str>> + Send + Sync;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ImageResourceKey {
    Path(Arc<Path>),
    Asset { source: u64, path: Arc<str> },
    Custom(u64),
}

#[derive(Clone)]
enum ImageResourceLoader {
    Path(Arc<Path>),
    Asset {
        assets: crate::Assets,
        path: Arc<str>,
    },
    Custom(Arc<CustomImageLoader>),
    Animated(Arc<CustomAnimatedImageLoader>),
}

/// A cheap, stable handle to image content loaded away from the UI thread.
///
/// Path resources deduplicate by path. Custom resources deduplicate by handle identity, so retain
/// and clone a custom handle instead of constructing it inside every `View::render` call.
#[derive(Clone)]
pub struct ImageResource {
    key: ImageResourceKey,
    loader: ImageResourceLoader,
}

impl ImageResource {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path: Arc<Path> = Arc::from(path.into().into_boxed_path());
        Self {
            key: ImageResourceKey::Path(path.clone()),
            loader: ImageResourceLoader::Path(path),
        }
    }

    pub(crate) fn from_asset(assets: crate::Assets, path: Arc<str>) -> Self {
        Self {
            key: ImageResourceKey::Asset {
                source: assets.id(),
                path: path.clone(),
            },
            loader: ImageResourceLoader::Asset { assets, path },
        }
    }

    /// Create a retained custom background loader.
    pub fn custom<F, E>(loader: F) -> Self
    where
        F: Fn() -> Result<Image, E> + Send + Sync + 'static,
        E: fmt::Display,
    {
        let id = NEXT_CUSTOM_RESOURCE_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            key: ImageResourceKey::Custom(id),
            loader: ImageResourceLoader::Custom(Arc::new(move || {
                loader().map_err(|error| Arc::from(error.to_string()))
            })),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.loader {
            ImageResourceLoader::Path(path) => Some(path),
            ImageResourceLoader::Asset { .. }
            | ImageResourceLoader::Custom(_)
            | ImageResourceLoader::Animated(_) => None,
        }
    }

    pub(crate) fn key(&self) -> &ImageResourceKey {
        &self.key
    }

    pub(crate) fn load(&self) -> Result<ImageAsset, Arc<str>> {
        match &self.loader {
            ImageResourceLoader::Path(path) => {
                ImageAsset::open(path).map_err(|error| Arc::from(error.to_string()))
            }
            ImageResourceLoader::Asset { assets, path } => {
                let bytes = assets
                    .load_required(path)
                    .map_err(|error| Arc::from(error.to_string()))?;
                ImageAsset::decode(bytes.as_ref()).map_err(|error| Arc::from(error.to_string()))
            }
            ImageResourceLoader::Custom(loader) => loader().map(ImageAsset::Static),
            ImageResourceLoader::Animated(loader) => loader().map(ImageAsset::Animated),
        }
    }

    /// Create a retained custom loader for a decoded animation.
    pub fn custom_animated<F, E>(loader: F) -> Self
    where
        F: Fn() -> Result<AnimatedImage, E> + Send + Sync + 'static,
        E: fmt::Display,
    {
        let id = NEXT_CUSTOM_RESOURCE_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            key: ImageResourceKey::Custom(id),
            loader: ImageResourceLoader::Animated(Arc::new(move || {
                loader().map_err(|error| Arc::from(error.to_string()))
            })),
        }
    }
}

impl fmt::Debug for ImageResource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.key {
            ImageResourceKey::Path(path) => formatter
                .debug_tuple("ImageResource::Path")
                .field(path)
                .finish(),
            ImageResourceKey::Asset { source, path } => formatter
                .debug_struct("ImageResource::Asset")
                .field("source", source)
                .field("path", path)
                .finish(),
            ImageResourceKey::Custom(id) => formatter
                .debug_tuple("ImageResource::Custom")
                .field(id)
                .finish(),
        }
    }
}

impl PartialEq for ImageResource {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for ImageResource {}

impl Hash for ImageResource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl fmt::Debug for Image {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Image")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("bytes", &self.byte_len())
            .field("template", &self.0.template)
            .field("representations", &self.0.representations.len())
            .finish_non_exhaustive()
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for Image {}

/// A source accepted by [`crate::img`]. New resource kinds can be added without changing views.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ImageSource {
    Image(Image),
    Animated(AnimatedImage),
    Resource(ImageResource),
}

impl ImageSource {
    pub(crate) fn image(&self) -> Option<&Image> {
        match self {
            Self::Image(image) => Some(image),
            Self::Animated(_) | Self::Resource(_) => None,
        }
    }

    pub(crate) fn animated(&self) -> Option<&AnimatedImage> {
        match self {
            Self::Animated(animation) => Some(animation),
            Self::Image(_) | Self::Resource(_) => None,
        }
    }

    pub(crate) fn resource(&self) -> Option<&ImageResource> {
        match self {
            Self::Image(_) | Self::Animated(_) => None,
            Self::Resource(resource) => Some(resource),
        }
    }
}

impl From<Image> for ImageSource {
    fn from(image: Image) -> Self {
        Self::Image(image)
    }
}

impl From<&Image> for ImageSource {
    fn from(image: &Image) -> Self {
        Self::Image(image.clone())
    }
}

impl From<AnimatedImage> for ImageSource {
    fn from(animation: AnimatedImage) -> Self {
        Self::Animated(animation)
    }
}

impl From<&AnimatedImage> for ImageSource {
    fn from(animation: &AnimatedImage) -> Self {
        Self::Animated(animation.clone())
    }
}

impl From<ImageResource> for ImageSource {
    fn from(resource: ImageResource) -> Self {
        Self::Resource(resource)
    }
}

impl From<&ImageResource> for ImageSource {
    fn from(resource: &ImageResource) -> Self {
        Self::Resource(resource.clone())
    }
}

impl From<PathBuf> for ImageSource {
    fn from(path: PathBuf) -> Self {
        Self::Resource(ImageResource::from_path(path))
    }
}

impl From<&Path> for ImageSource {
    fn from(path: &Path) -> Self {
        Self::Resource(ImageResource::from_path(path))
    }
}

impl From<String> for ImageSource {
    fn from(path: String) -> Self {
        Self::Resource(ImageResource::from_path(path))
    }
}

impl From<&str> for ImageSource {
    fn from(path: &str) -> Self {
        Self::Resource(ImageResource::from_path(path))
    }
}

/// How image content is sized inside its layout box.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ObjectFit {
    Fill,
    #[default]
    Contain,
    Cover,
    ScaleDown,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ImageFit {
    pub destination: Rect,
    pub source_uv: Rect,
}

pub(crate) fn fit_image(bounds: Rect, intrinsic: Size, fit: ObjectFit) -> ImageFit {
    debug_assert!(!bounds.is_empty());
    debug_assert!(!intrinsic.is_empty());
    match fit {
        ObjectFit::Fill => ImageFit {
            destination: bounds,
            source_uv: Rect::new(0.0, 0.0, 1.0, 1.0),
        },
        ObjectFit::Contain => contain(bounds, intrinsic),
        ObjectFit::Cover => cover(bounds, intrinsic),
        ObjectFit::ScaleDown => {
            if intrinsic.width <= bounds.width && intrinsic.height <= bounds.height {
                unscaled(bounds, intrinsic)
            } else {
                contain(bounds, intrinsic)
            }
        }
        ObjectFit::None => unscaled(bounds, intrinsic),
    }
}

fn contain(bounds: Rect, intrinsic: Size) -> ImageFit {
    let scale = (bounds.width / intrinsic.width).min(bounds.height / intrinsic.height);
    let size = Size::new(intrinsic.width * scale, intrinsic.height * scale);
    ImageFit {
        destination: centered(bounds, size),
        source_uv: Rect::new(0.0, 0.0, 1.0, 1.0),
    }
}

fn cover(bounds: Rect, intrinsic: Size) -> ImageFit {
    let scale = (bounds.width / intrinsic.width).max(bounds.height / intrinsic.height);
    let visible_width = bounds.width / scale;
    let visible_height = bounds.height / scale;
    ImageFit {
        destination: bounds,
        source_uv: Rect::new(
            (intrinsic.width - visible_width) * 0.5 / intrinsic.width,
            (intrinsic.height - visible_height) * 0.5 / intrinsic.height,
            visible_width / intrinsic.width,
            visible_height / intrinsic.height,
        ),
    }
}

fn unscaled(bounds: Rect, intrinsic: Size) -> ImageFit {
    ImageFit {
        destination: centered(bounds, intrinsic),
        source_uv: Rect::new(0.0, 0.0, 1.0, 1.0),
    }
}

fn centered(bounds: Rect, size: Size) -> Rect {
    Rect::new(
        bounds.x + (bounds.width - size.width) * 0.5,
        bounds.y + (bounds.height - size.height) * 0.5,
        size.width,
        size.height,
    )
}

/// Decode standard base64 without allocating an intermediate alphabet table per call.
///
/// Whitespace is skipped so wrapped `data:` URLs decode, and any other character is rejected.
fn decode_base64(value: &str) -> Result<Vec<u8>, ImageError> {
    const INVALID: u8 = 0xFF;
    const SKIP: u8 = 0xFE;
    const PAD: u8 = 0xFD;

    fn symbol(byte: u8) -> u8 {
        match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => PAD,
            b' ' | b'\t' | b'\r' | b'\n' => SKIP,
            _ => INVALID,
        }
    }

    let mut decoded = Vec::with_capacity(value.len() / 4 * 3);
    let mut accumulator = 0_u32;
    let mut symbols = 0_u32;
    let mut padding = 0_usize;
    for byte in value.bytes() {
        match symbol(byte) {
            SKIP => continue,
            PAD => {
                padding += 1;
                if padding > 2 {
                    return Err(ImageError::InvalidDataUrl);
                }
                continue;
            }
            INVALID => return Err(ImageError::InvalidDataUrl),
            value if padding > 0 => {
                let _ = value;
                return Err(ImageError::InvalidDataUrl);
            }
            value => {
                accumulator = (accumulator << 6) | u32::from(value);
                symbols += 1;
                if symbols == 4 {
                    decoded.extend_from_slice(&accumulator.to_be_bytes()[1..]);
                    accumulator = 0;
                    symbols = 0;
                }
            }
        }
    }
    match symbols {
        0 => {}
        2 => decoded.push((accumulator >> 4) as u8),
        3 => {
            decoded.push((accumulator >> 10) as u8);
            decoded.push((accumulator >> 2) as u8);
        }
        _ => return Err(ImageError::InvalidDataUrl),
    }
    if decoded.is_empty() {
        return Err(ImageError::InvalidDataUrl);
    }
    Ok(decoded)
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::EmptyDimensions { width, height });
    }
    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(ImageError::DimensionsTooLarge {
            width,
            height,
            maximum: MAX_IMAGE_DIMENSION,
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("image dimensions must be non-zero, got {width}x{height}")]
    EmptyDimensions { width: u32, height: u32 },
    #[error("image dimensions {width}x{height} exceed the {maximum}px per-axis limit")]
    DimensionsTooLarge {
        width: u32,
        height: u32,
        maximum: u32,
    },
    #[error("decoded image uses {bytes} bytes, exceeding the {maximum}-byte limit")]
    TooLarge { bytes: u64, maximum: u64 },
    #[error("encoded image uses {bytes} bytes, exceeding the {maximum}-byte limit")]
    EncodedTooLarge { bytes: u64, maximum: u64 },
    #[error("an animated image must contain at least one decodable frame")]
    EmptyAnimation,
    #[error("animation contains {frames} frames, exceeding the {maximum}-frame limit")]
    TooManyAnimationFrames { frames: usize, maximum: usize },
    #[error("decoded animation uses {bytes} bytes, exceeding the {maximum}-byte limit")]
    AnimationTooLarge { bytes: u64, maximum: u64 },
    #[error("the decoded image is static rather than animated")]
    NotAnimated,
    #[error("a finite animation must play at least one iteration")]
    ZeroAnimationIterations,
    #[error("RGBA data has {actual} bytes; exactly {expected} were required")]
    InvalidPixelLength { expected: u64, actual: usize },
    #[error("could not inspect encoded image data: {0}")]
    Inspect(#[source] std::io::Error),
    #[error("could not open image file {path}: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not decode image data: {0}")]
    Decode(#[source] image_codecs::ImageError),
    #[error("could not encode image data: {0}")]
    Encode(#[source] image_codecs::ImageError),
    #[error("could not rasterize the native image: {0}")]
    NativeRasterize(&'static str),
    #[error("a data URL must be a base64 `data:` URL carrying a supported image format")]
    InvalidDataUrl,
    #[error("a crop rectangle must be non-empty, whole-pixel, and inside the source image")]
    InvalidCrop,
    #[error("JPEG quality must be between 1 and 100, got {quality}")]
    InvalidJpegQuality { quality: u8 },
    #[error("an image carries {representations} representations, exceeding the {maximum} limit")]
    TooManyRepresentations {
        representations: usize,
        maximum: usize,
    },
    #[error("an image representation scale must be finite and between 0.25 and 16")]
    InvalidRepresentationScale,
    #[error("the operating system does not provide a system image named {name}")]
    SystemImageUnavailable { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnimatedImageFrame, Assets, BundledAssets};
    use image_codecs::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    fn temporary_path(label: &str) -> PathBuf {
        static NEXT_FILE_ID: AtomicU64 = AtomicU64::new(1);
        std::env::temp_dir().join(format!(
            "quickgui-{label}-{}-{}.png",
            std::process::id(),
            NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn rgba_length_is_validated() {
        let error = Image::from_rgba(2, 2, vec![0; 15]).unwrap_err();
        assert!(matches!(
            error,
            ImageError::InvalidPixelLength {
                expected: 16,
                actual: 15
            }
        ));
    }

    #[test]
    fn cloned_images_keep_identity_without_copying_pixels() {
        let image = Image::from_rgba(1, 1, vec![255; 4]).unwrap();
        assert_eq!(image, image.clone());
    }

    #[test]
    fn open_decodes_a_supported_file_with_the_same_limits() {
        let path = temporary_path("open");
        let pixels = [7, 11, 13, 255];
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&pixels, 1, 1, ExtendedColorType::Rgba8)
            .unwrap();
        std::fs::write(&path, encoded).unwrap();

        let image = Image::open(&path).unwrap();
        let _ = std::fs::remove_file(path);
        assert_eq!(image.size(), Size::new(1.0, 1.0));
        assert_eq!(image.rgba(), pixels);
        assert!(!image.is_template());
    }

    #[test]
    fn open_marks_template_filenames() {
        let path = std::env::temp_dir().join(format!(
            "quickgui-{}-statusTemplate.png",
            std::process::id()
        ));
        let pixels = [7, 11, 13, 255];
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&pixels, 1, 1, ExtendedColorType::Rgba8)
            .unwrap();
        std::fs::write(&path, encoded).unwrap();

        let image = Image::open(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(image.is_template());
        assert!(!image.template(false).is_template());
    }

    #[test]
    fn resource_clones_keep_custom_loader_identity_and_normalize_errors() {
        let resource = ImageResource::custom(|| Err::<Image, _>("not available"));
        assert_eq!(resource, resource.clone());
        assert_eq!(resource.load().unwrap_err().as_ref(), "not available");
    }

    #[test]
    fn asset_resources_deduplicate_by_source_and_path_and_decode_on_demand() {
        let pixels = [7, 11, 13, 255];
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&pixels, 1, 1, ExtendedColorType::Rgba8)
            .unwrap();

        let mut bundle = BundledAssets::new();
        bundle.insert("images/pixel.png", encoded).unwrap();
        let assets = Assets::new(bundle.clone());
        let first = assets.image("images/pixel.png").unwrap();
        let second = assets.image("images/pixel.png").unwrap();
        assert_eq!(first, second);

        let ImageAsset::Static(decoded) = first.load().unwrap() else {
            panic!("PNG asset decoded as an animation");
        };
        assert_eq!(decoded.size(), Size::new(1.0, 1.0));
        assert_eq!(decoded.rgba(), pixels);

        let other_source = Assets::new(bundle);
        assert_ne!(
            first,
            other_source.image("images/pixel.png").unwrap(),
            "distinct application asset sources must not alias renderer cache entries"
        );
    }

    #[test]
    fn custom_animated_resources_preserve_the_decoded_asset() {
        let animation = AnimatedImage::new([
            AnimatedImageFrame::new(
                Image::from_rgba(1, 1, vec![1, 2, 3, 255]).unwrap(),
                web_time::Duration::from_millis(40),
            ),
            AnimatedImageFrame::new(
                Image::from_rgba(1, 1, vec![4, 5, 6, 255]).unwrap(),
                web_time::Duration::from_millis(60),
            ),
        ])
        .unwrap();
        let expected = animation.clone();
        let resource =
            ImageResource::custom_animated(move || Ok::<_, &'static str>(animation.clone()));
        assert_eq!(resource.load().unwrap(), ImageAsset::Animated(expected));
    }

    fn gradient(width: u32, height: u32) -> Image {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&[(x * 8) as u8, (y * 8) as u8, (x * y) as u8, 255]);
            }
        }
        Image::from_rgba(width, height, pixels).unwrap()
    }

    #[test]
    fn png_and_jpeg_round_trips_preserve_geometry_and_opaque_pixels() {
        let source = gradient(4, 3);
        let png = source.to_png().unwrap();
        let decoded = Image::decode(&png).unwrap();
        assert_eq!(decoded.size(), source.size());
        assert_eq!(decoded.rgba(), source.rgba());

        let jpeg = source.to_jpeg(95).unwrap();
        let decoded = Image::decode(&jpeg).unwrap();
        assert_eq!(decoded.size(), source.size());
        assert!(
            decoded
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[3] == 255),
            "JPEG has no alpha channel"
        );
        assert!(matches!(
            source.to_jpeg(0).unwrap_err(),
            ImageError::InvalidJpegQuality { quality: 0 }
        ));
        assert!(matches!(
            source.to_jpeg(101).unwrap_err(),
            ImageError::InvalidJpegQuality { quality: 101 }
        ));
    }

    #[test]
    fn jpeg_encoding_composites_translucent_pixels_over_white() {
        let image = Image::from_rgba(1, 1, vec![0, 0, 0, 0]).unwrap();
        let decoded = Image::decode(image.to_jpeg(100).unwrap()).unwrap();
        let pixel = decoded.rgba();
        assert!(
            pixel[0] > 240 && pixel[1] > 240 && pixel[2] > 240,
            "a fully transparent pixel composites to white, got {pixel:?}"
        );
    }

    #[test]
    fn resize_is_bilinear_and_crop_copies_an_inside_rectangle() {
        let source = gradient(4, 4);
        let resized = source.resize(2, 2).unwrap();
        assert_eq!(resized.size(), Size::new(2.0, 2.0));
        assert_eq!(resized.byte_len(), 16);
        assert_eq!(
            source.resize(4, 4).unwrap(),
            source,
            "a no-op resize aliases"
        );
        assert!(source.resize(0, 4).is_err());
        assert!(source.resize(MAX_IMAGE_DIMENSION + 1, 4).is_err());

        let cropped = source.crop(Rect::new(1.0, 1.0, 2.0, 2.0)).unwrap();
        assert_eq!(cropped.size(), Size::new(2.0, 2.0));
        let stride = 4 * 4;
        assert_eq!(
            &cropped.rgba()[..8],
            &source.rgba()[stride + 4..stride + 12]
        );

        for invalid in [
            Rect::new(-1.0, 0.0, 2.0, 2.0),
            Rect::new(0.0, 0.0, 0.0, 2.0),
            Rect::new(3.0, 3.0, 2.0, 2.0),
            Rect::new(f32::NAN, 0.0, 2.0, 2.0),
        ] {
            assert!(matches!(
                source.crop(invalid).unwrap_err(),
                ImageError::InvalidCrop
            ));
        }
    }

    #[test]
    fn data_urls_decode_base64_payloads_and_reject_malformed_input() {
        let source = gradient(2, 2);
        let png = source.to_png().unwrap();
        let encoded = encode_base64_for_test(&png);
        let decoded = Image::from_data_url(&format!("data:image/png;base64,{encoded}")).unwrap();
        assert_eq!(decoded.rgba(), source.rgba());
        // The media type is optional and the encoding token is case-insensitive.
        assert!(Image::from_data_url(&format!("data:;BASE64,{encoded}")).is_ok());

        for invalid in [
            "https://example.com/a.png".to_owned(),
            "data:image/png,notbase64".to_owned(),
            "data:image/png;base64".to_owned(),
            "data:image/png;base64,%%%%".to_owned(),
            "data:image/png;base64,".to_owned(),
            format!("data:image/png;base64,{encoded}=A"),
        ] {
            assert!(
                matches!(
                    Image::from_data_url(&invalid),
                    Err(ImageError::InvalidDataUrl)
                ),
                "{invalid} should be rejected"
            );
        }
        assert!(matches!(
            Image::from_data_url(&format!(
                "data:image/png;base64,{}",
                "A".repeat(MAX_IMAGE_DATA_URL_BYTES)
            )),
            Err(ImageError::EncodedTooLarge { .. })
        ));
    }

    #[test]
    fn template_and_representation_metadata_keep_pixels_and_gain_identity() {
        let base = gradient(2, 2);
        assert!(!base.is_template());
        assert!(base.representations().is_empty());

        let template = base.template(true);
        assert!(template.is_template());
        assert_ne!(
            template, base,
            "native metadata gets a fresh cache identity"
        );
        assert_eq!(template.rgba(), base.rgba());
        assert_eq!(base.template(false), base, "an unchanged flag aliases");

        let retina = gradient(4, 4);
        let multi = base.with_representations([(2.0, retina.clone())]).unwrap();
        assert_eq!(multi.representations().len(), 1);
        assert_eq!(multi.representations()[0].0, 2.0);
        assert_eq!(multi.representations()[0].1, retina);
        assert_eq!(multi.size(), base.size());

        assert!(matches!(
            base.with_representations([(0.0, retina.clone())]),
            Err(ImageError::InvalidRepresentationScale)
        ));
        assert!(matches!(
            base.with_representations([(f32::INFINITY, retina.clone())]),
            Err(ImageError::InvalidRepresentationScale)
        ));
        let too_many =
            std::iter::repeat_n((2.0_f32, retina.clone()), MAX_IMAGE_REPRESENTATIONS + 1);
        assert!(matches!(
            base.with_representations(too_many),
            Err(ImageError::TooManyRepresentations { .. })
        ));
    }

    #[test]
    fn system_images_validate_their_name_and_requested_size() {
        assert!(matches!(
            Image::named_system(""),
            Err(ImageError::SystemImageUnavailable { .. })
        ));
        assert!(matches!(
            Image::named_system("a\0b"),
            Err(ImageError::SystemImageUnavailable { .. })
        ));
        assert!(Image::named_system_sized("NSAddTemplate", 0.0, 2.0).is_err());
        assert!(Image::named_system_sized("NSAddTemplate", 16.0, f32::NAN).is_err());
        assert!(matches!(
            Image::named_system_sized("NSAddTemplate", 4_096.0, 4.0),
            Err(ImageError::DimensionsTooLarge { .. })
        ));
        #[cfg(not(target_os = "macos"))]
        assert!(matches!(
            Image::named_system("NSAddTemplate"),
            Err(ImageError::SystemImageUnavailable { .. })
        ));
    }

    /// Minimal standard-base64 encoder used only to build deterministic test fixtures.
    fn encode_base64_for_test(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let mut buffer = [0_u8; 3];
            buffer[..chunk.len()].copy_from_slice(chunk);
            let value = u32::from_be_bytes([0, buffer[0], buffer[1], buffer[2]]);
            for index in 0..4 {
                if index <= chunk.len() {
                    let symbol = (value >> (18 - index * 6)) & 0x3F;
                    encoded.push(ALPHABET[symbol as usize] as char);
                } else {
                    encoded.push('=');
                }
            }
        }
        encoded
    }

    #[test]
    fn contain_centers_the_complete_source() {
        let fitted = fit_image(
            Rect::new(10.0, 20.0, 200.0, 200.0),
            Size::new(400.0, 200.0),
            ObjectFit::Contain,
        );
        assert_eq!(fitted.destination, Rect::new(10.0, 70.0, 200.0, 100.0));
        assert_eq!(fitted.source_uv, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn cover_centers_a_normalized_source_crop() {
        let fitted = fit_image(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Size::new(200.0, 100.0),
            ObjectFit::Cover,
        );
        assert_eq!(fitted.destination, Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(fitted.source_uv, Rect::new(0.25, 0.0, 0.5, 1.0));
    }

    #[test]
    fn scale_down_never_enlarges_small_images() {
        let fitted = fit_image(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Size::new(20.0, 40.0),
            ObjectFit::ScaleDown,
        );
        assert_eq!(fitted.destination, Rect::new(40.0, 30.0, 20.0, 40.0));
    }

    #[test]
    fn template_image_paths_follow_the_macos_filename_convention() {
        for path in [
            "trayTemplate.png",
            "statusTemplate@2x.png",
            "icons/statusTemplate@3x.png",
            "Template.png",
        ] {
            assert!(is_template_image_path(path), "{path}");
        }
        for path in [
            "icon.png",
            "emailTemplateIcon.png",
            "template.png",
            "tray-template.png",
            "status@2x.png",
        ] {
            assert!(!is_template_image_path(path), "{path}");
        }
    }
}
