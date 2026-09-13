use objc2::runtime::AnyClass;
use objc2_app_kit::{
    NSBitmapFormat, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace,
    NSGraphicsContext, NSImageInterpolation,
};
use objc2_foundation::NSData;

use super::*;
use crate::{Image, ImageError};

/// `NSImageSymbolScaleLarge`.
const SYMBOL_SCALE_LARGE: isize = 3;
/// `NSFontWeightRegular`.
const SYMBOL_WEIGHT_REGULAR: f64 = 0.0;

/// Rasterize a named `NSImage` or SF Symbol at a requested point size and backing scale.
///
/// AppKit resolves and rasterizes the native representation directly into bounded RGBA8 storage.
pub(crate) fn system_image(name: &str, point_size: f64, scale: f64) -> Result<Image, ImageError> {
    let unavailable = || ImageError::SystemImageUnavailable {
        name: name.to_owned(),
    };
    let mtm = MainThreadMarker::new().ok_or_else(unavailable)?;
    let native = NSString::from_str(name);
    let pixels = (point_size * scale).round().max(1.0);

    let image = unsafe { NSImage::imageNamed(&native) }
        .or_else(|| symbol_image(&native, pixels))
        .ok_or_else(unavailable)?;
    let size = unsafe { image.size() };
    if !size.width.is_finite()
        || !size.height.is_finite()
        || size.width <= 0.0
        || size.height <= 0.0
    {
        return Err(unavailable());
    }
    // Preserve the aspect ratio while fitting the longest axis to the requested pixel size.
    let longest = size.width.max(size.height);
    let fit = |value: f64| (value * pixels / longest).round().max(1.0) as u32;
    rasterize_native_image(mtm, &image, fit(size.width), fit(size.height))
}

/// Resolve vector, wide-gamut, and floating-point native representations through AppKit, rather
/// than encoding TIFF and relying on a codec to support whichever representation it chooses.
pub(crate) fn rasterize_native_image(
    _mtm: MainThreadMarker,
    image: &NSImage,
    width: u32,
    height: u32,
) -> Result<Image, ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::EmptyDimensions { width, height });
    }
    if width > crate::MAX_IMAGE_DIMENSION || height > crate::MAX_IMAGE_DIMENSION {
        return Err(ImageError::DimensionsTooLarge {
            width,
            height,
            maximum: crate::MAX_IMAGE_DIMENSION,
        });
    }
    // AppKit owns its bitmap storage; the dimension limit bounds it to MAX_DECODED_IMAGE_BYTES.
    let stride = width as usize * 4;
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            null_mut(),
            width as isize,
            height as isize,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            NSBitmapFormat::empty(),
            stride as isize,
            32,
        )
    }
    .ok_or(ImageError::NativeRasterize("AppKit could not allocate an RGBA8 bitmap"))?;
    let data = unsafe { bitmap.bitmapData() };
    if data.is_null() || unsafe { bitmap.bytesPerRow() } != stride as isize {
        return Err(ImageError::NativeRasterize(
            "AppKit returned an invalid RGBA8 bitmap",
        ));
    }
    let byte_len = stride * height as usize;
    // SAFETY: the allocated bitmap owns byte_len bytes. Clear transparent pixels before drawing.
    unsafe { data.write_bytes(0, byte_len) };
    let context = unsafe { NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap) }.ok_or(
        ImageError::NativeRasterize("AppKit could not create a bitmap graphics context"),
    )?;
    // SAFETY: main-thread AppKit drawing into its bounded pixel buffer.
    // Saving/restoring the thread's context prevents changing the surrounding window's drawing.
    unsafe {
        NSGraphicsContext::saveGraphicsState_class();
        NSGraphicsContext::setCurrentContext(Some(&context));
        context.setImageInterpolation(NSImageInterpolation::High);
        image.drawInRect_fromRect_operation_fraction(
            NSRect::new(
                NSPoint::ZERO,
                NSSize::new(f64::from(width), f64::from(height)),
            ),
            NSRect::ZERO,
            NSCompositingOperation::Copy,
            1.0,
        );
        NSGraphicsContext::restoreGraphicsState_class();
    }
    drop(context);
    // SAFETY: copy the tightly packed RGBA8 bitmap while its owning representation is alive.
    let mut rgba = unsafe { std::slice::from_raw_parts(data, byte_len) }.to_vec();
    drop(bitmap);
    unpremultiply_rgba(&mut rgba);
    Image::from_rgba(width, height, rgba)
}

fn unpremultiply_rgba(pixels: &mut [u8]) {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = u32::from(*channel)
                .saturating_mul(255)
                .saturating_add(alpha / 2)
                .checked_div(alpha)
                .unwrap_or(0)
                .min(255) as u8;
        }
    }
}

/// Resolve an SF Symbol and apply a point-size configuration so the rasterization is large enough.
fn symbol_image(name: &NSString, pixels: f64) -> Option<Retained<NSImage>> {
    let image = unsafe { NSImage::imageWithSystemSymbolName_accessibilityDescription(name, None) }?;
    let Some(class) = AnyClass::get("NSImageSymbolConfiguration") else {
        return Some(image);
    };
    let configuration: Option<Retained<AnyObject>> = unsafe {
        msg_send_id![
            class,
            configurationWithPointSize: pixels,
            weight: SYMBOL_WEIGHT_REGULAR,
            scale: SYMBOL_SCALE_LARGE,
        ]
    };
    let Some(configuration) = configuration else {
        return Some(image);
    };
    let configured: Option<Retained<NSImage>> =
        unsafe { msg_send_id![&image, imageWithSymbolConfiguration: &*configuration] };
    configured.or(Some(image))
}

/// Build an `NSImage` honoring QuickGUI's template flag and additional scale representations.
pub(crate) fn native_image_with_metadata(
    mtm: MainThreadMarker,
    image: &Image,
) -> Result<Retained<NSImage>, PlatformError> {
    let encoded = image
        .to_png()
        .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
    let native = NSImage::initWithData(mtm.alloc(), &NSData::with_bytes(&encoded))
        .ok_or_else(|| PlatformError::Platform("AppKit could not decode the image".into()))?;
    for (_, representation) in image.representations() {
        let encoded = representation
            .to_png()
            .map_err(|error| PlatformError::Platform(error.to_string().into()))?;
        let Some(variant) = NSImage::initWithData(mtm.alloc(), &NSData::with_bytes(&encoded))
        else {
            continue;
        };
        unsafe {
            let reps: Option<Retained<AnyObject>> = msg_send_id![&variant, representations];
            let Some(reps) = reps else { continue };
            let count: usize = msg_send![&reps, count];
            for index in 0..count {
                let rep: Option<Retained<AnyObject>> = msg_send_id![&reps, objectAtIndex: index];
                if let Some(rep) = rep {
                    let _: () = msg_send![&native, addRepresentation: &*rep];
                }
            }
        }
    }
    if !image.representations().is_empty() {
        // The receiver is the 1x representation, so its pixel size is also its point size.
        unsafe {
            native.setSize(NSSize::new(
                f64::from(image.width()),
                f64::from(image.height()),
            ));
        }
    }
    unsafe { native.setTemplate(image.is_template()) };
    Ok(native)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_pixels_preserve_color_at_translucent_edges() {
        let mut pixels = [12, 34, 56, 255, 128, 64, 0, 128, 1, 0, 0, 1, 99, 42, 17, 0];
        unpremultiply_rgba(&mut pixels);
        assert_eq!(
            pixels,
            [12, 34, 56, 255, 255, 128, 0, 128, 255, 0, 0, 1, 0, 0, 0, 0]
        );
    }
}
