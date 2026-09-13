#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::ClassType;
    use objc2_app_kit::{
        NSApplication, NSBitmapFormat, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage,
    };
    use objc2_foundation::{MainThreadMarker, NSData, NSSize, NSString};
    use quickgui::Image;

    let mtm = MainThreadMarker::new().expect("native image checks require the main thread");
    let _application = NSApplication::sharedApplication(mtm);

    // An asymmetric fixture detects vertical flips, channel swaps, and premultiplied-alpha leaks.
    let expected = [255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 0, 0, 0, 0];
    let png = Image::from_rgba(2, 2, expected.to_vec())
        .unwrap()
        .to_png()
        .unwrap();
    let native = NSImage::initWithData(mtm.alloc(), &NSData::with_bytes(&png)).unwrap();
    let name = NSString::from_str("QuickGUI.Rasterization.RGBA8");
    assert!(unsafe { native.setName(Some(&name)) });
    let raster = Image::named_system_sized("QuickGUI.Rasterization.RGBA8", 2.0, 1.0).unwrap();
    assert_eq!((raster.width(), raster.height()), (2, 2));
    for (actual, expected) in raster.rgba().iter().zip(expected) {
        assert!(
            actual.abs_diff(expected) <= 1,
            "RGBA8 orientation or alpha mismatch: {:?}",
            raster.rgba()
        );
    }

    // Recent macOS icons can return floating-point TIFF samples. The ordinary TIFF decoder
    // rejects half-float samples; native images must still rasterize without going through it.
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            2,
            2,
            16,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            NSBitmapFormat::FloatingPointSamples | NSBitmapFormat::SixteenBitLittleEndian,
            16,
            64,
        )
    }.expect("AppKit supports RGBA16F image representations");
    let samples: [u16; 16] = [
        0x3c00, 0, 0, 0x3c00, // opaque red
        0, 0x3800, 0, 0x3800, // half-alpha green, premultiplied
        0, 0, 0x3c00, 0x3c00, // opaque blue
        0, 0, 0, 0, // transparent
    ];
    let bytes: Vec<u8> = samples.into_iter().flat_map(u16::to_le_bytes).collect();
    unsafe {
        assert_eq!(bitmap.bytesPerRow(), 16);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), bitmap.bitmapData(), bytes.len());
    }
    let native = unsafe { NSImage::initWithSize(mtm.alloc(), NSSize::new(2.0, 2.0)) };
    unsafe { native.addRepresentation(&bitmap) };
    assert!(unsafe { native.setName(Some(&NSString::from_str("QuickGUI.Rasterization.RGBA16F"))) });
    let raster = Image::named_system_sized("QuickGUI.Rasterization.RGBA16F", 2.0, 1.0).unwrap();
    for (actual, expected) in raster.rgba().iter().zip(expected) {
        assert!(
            actual.abs_diff(expected) <= 1,
            "RGBA16F orientation or alpha mismatch: {:?}",
            raster.rgba()
        );
    }

    // Native vector symbols select a representation at the requested size and retain their ratio.
    let symbol = Image::named_system_sized("arrow.triangle.2.circlepath", 20.0, 2.0).unwrap();
    assert_eq!(symbol.width().max(symbol.height()), 40);
    assert!(
        symbol
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] != 0)
    );
    println!("native images: RGBA8, RGBA16F, alpha, orientation, and symbol sizing passed");
}
