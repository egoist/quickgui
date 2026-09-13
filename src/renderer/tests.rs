use super::*;
use crate::{
    BoxShadow, Color, FontFeatureTag, HighlightStyle, Hyphens, MAX_TEXT_SHADOW_SAMPLES,
    OverflowWrap, StyledText, TextDirection, TextRun, TextShadow, TextTransform, WordBreak,
};
#[cfg(target_os = "macos")]
use crate::{
    CustomShader, CustomShaderPrimitive, Image, ImagePrimitive, PathBuilder, PathPrimitive, Svg,
    SvgPrimitive,
};

fn fixture_font_system() -> FontSystem {
    let mut database = glyphon::cosmic_text::fontdb::Database::new();
    database
        .load_font_data(include_bytes!("../../tests/fixtures/fonts/Inter-Regular.ttf").to_vec());
    database
        .load_font_data(include_bytes!("../../tests/fixtures/fonts/NotoSansHebrew.ttf").to_vec());
    FontSystem::new_with_locale_and_db("en-US".to_owned(), database)
}

fn fixture_terminal_font_system() -> FontSystem {
    let mut database = glyphon::cosmic_text::fontdb::Database::new();
    database.load_font_data(
        include_bytes!("../../examples/herdr-gui/assets/JetBrainsMonoNerdFontMono-Regular.ttf")
            .to_vec(),
    );
    FontSystem::new_with_locale_and_db("en-US".to_owned(), database)
}

#[test]
fn application_font_system_handle_is_shared_without_a_lock() {
    let fonts = create_shared_font_system(&Assets::default(), &[]).unwrap();
    assert!(Rc::ptr_eq(&fonts, &fonts.clone()));
}

#[cfg(target_os = "macos")]
#[test]
fn retained_uploads_update_the_correct_physical_buffer() {
    let fonts = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let renderer =
        pollster::block_on(OffscreenRenderer::new(PerformanceProfile::Balanced, fonts)).unwrap();
    let (device, queue) = renderer.gpu();
    let make_buffer = || {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("retained upload test"),
            size: 4096,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    };
    let buffers = [make_buffer(), make_buffer()];
    let mut uploads = super::upload::BufferUploads::default();
    let mut data = vec![1u8; 4096];
    for (slot, buffer) in buffers.iter().enumerate() {
        uploads.write(slot, queue, buffer, &data);
    }
    uploads.begin_frame();
    uploads.write(0, queue, &buffers[0], &data);
    assert_eq!(uploads.stats().bytes, 0);
    data[512] = 2;
    uploads.write(1, queue, &buffers[1], &data);
    assert_eq!(uploads.stats().bytes, 256);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4096,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let read = |buffer: &wgpu::Buffer| {
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(buffer, 0, &readback, 0, 4096);
        queue.submit(Some(encoder.finish()));
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();
        let bytes = readback.slice(..).get_mapped_range().unwrap().to_vec();
        readback.unmap();
        bytes
    };
    assert_eq!(read(&buffers[0]), vec![1; 4096]);
    assert_eq!(read(&buffers[1]), data);
    let replacement = make_buffer();
    uploads.reset(1);
    uploads.begin_frame();
    uploads.write(1, queue, &replacement, &data);
    assert_eq!(uploads.stats().bytes, 4096);
    assert_eq!(read(&replacement), data);
}

#[cfg(target_os = "macos")]
#[test]
fn compositing_planes_keep_distinct_uniforms_in_one_submission() {
    use crate::scene::PaintLayerKey;
    let fonts = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        fonts.clone(),
    ))
    .unwrap();
    let (device, queue) = renderer.gpu();
    let format = TextureFormat::Rgba8Unorm;
    let size = Size::new(64.0, 64.0);
    let mut scene = Scene::new();
    for (plane, angle, color, blur) in [
        (ScenePlane::Base, 15.0, Color::WHITE, 1.0),
        (ScenePlane::Overlay, -35.0, Color::rgb8(255, 0, 0), 3.0),
    ] {
        let key = PaintLayerKey {
            plane,
            ..Default::default()
        };
        let bounds = Rect::new(15.0, 20.0, 25.0, 12.0);
        let group = scene
            .begin_group(
                key,
                bounds,
                Rect::from_size(size),
                crate::LayerEffects {
                    transform: crate::Transform2D::rotate_degrees(angle)
                        .around(Point::new(30.0, 30.0)),
                    blur,
                    ..Default::default()
                },
            )
            .unwrap();
        scene.push_quad_in(group.content_key(), Quad::new(bounds, color));
        scene.end_group(group);
    }
    scene.finish();
    let mut shapes = ShapeRenderer::new(device, format, None);
    shapes.prepare(device, queue, &scene, Rect::from_size(size), 64, 64, 1.0);
    let text = TextSystem::new(device, queue, format, fonts, None);
    let renderers = SceneRenderers {
        shapes: &shapes,
        text: &text,
        path: None,
        image: None,
        svg: None,
        custom_shader: None,
    };
    let targets: Vec<_> = (0..4)
        .map(|_| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            (texture, view)
        })
        .collect();
    let mut shared = Compositor::default();
    let mut independent = [Compositor::default(), Compositor::default()];
    let mut encoder = device.create_command_encoder(&Default::default());
    for (index, plane) in [ScenePlane::Base, ScenePlane::Overlay]
        .into_iter()
        .enumerate()
    {
        let frame = CompositeFrame {
            width: 64,
            height: 64,
            scale: 1.0,
            format,
            target_copyable: true,
            plane: Some(plane),
        };
        shared
            .render_scene(
                device,
                queue,
                &mut encoder,
                &scene,
                &renderers,
                &targets[index].1,
                Some(&targets[index].0),
                Some(Color::BLACK),
                frame,
            )
            .unwrap();
        independent[index]
            .render_scene(
                device,
                queue,
                &mut encoder,
                &scene,
                &renderers,
                &targets[index + 2].1,
                Some(&targets[index + 2].0),
                Some(Color::BLACK),
                frame,
            )
            .unwrap();
    }
    let readbacks: Vec<_> = targets
        .iter()
        .map(|(texture, _)| {
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 64 * 256,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(64),
                    },
                },
                wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
            );
            readback
        })
        .collect();
    queue.submit(Some(encoder.finish()));
    let pixels: Vec<_> = readbacks
        .iter()
        .map(|buffer| {
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    sender.send(result).unwrap();
                });
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            receiver.recv().unwrap().unwrap();
            let pixels = buffer.slice(..).get_mapped_range().unwrap().to_vec();
            buffer.unmap();
            pixels
        })
        .collect();
    assert_eq!(
        pixels[0], pixels[2],
        "overlay uniforms overwrote the base plane"
    );
    assert_eq!(pixels[1], pixels[3]);
    assert_ne!(pixels[0], pixels[1]);
}

#[test]
fn advanced_shaping_uses_ordered_custom_fallbacks_before_platform_fonts() {
    fn shaped_families(buffer: &Buffer, font_system: &FontSystem) -> Vec<(usize, String)> {
        buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.iter())
            .map(|glyph| {
                (
                    glyph.start,
                    font_system.db().face(glyph.font_id).unwrap().families[0]
                        .0
                        .clone(),
                )
            })
            .collect()
    }

    let mut font_system = fixture_font_system();
    let missing_then_noto = TextStyle::new(16.0, Color::WHITE)
        .family(FontFamily::named("Missing QuickGUI Test Primary"))
        .font_fallbacks(FontFallbacks::from_fonts([
            "Missing QuickGUI Test Fallback",
            "Noto Sans Hebrew",
            "Inter",
        ]));
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 22.0));
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        "A",
        &missing_then_noto,
        None,
        None,
        1.0,
    );
    assert_eq!(
        shaped_families(&buffer, &font_system),
        vec![(0, "Noto Sans Hebrew".to_owned())]
    );

    let inter_then_noto = TextStyle::new(16.0, Color::WHITE)
        .family(FontFamily::named("Missing QuickGUI Test Primary"))
        .font_fallbacks(FontFallbacks::from_fonts(["Inter", "Noto Sans Hebrew"]));
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        "Aא",
        &inter_then_noto,
        None,
        None,
        1.0,
    );
    let families = shaped_families(&buffer, &font_system);
    assert!(families.contains(&(0, "Inter".to_owned())));
    assert!(families.contains(&(1, "Noto Sans Hebrew".to_owned())));
}

#[test]
fn basic_shaping_preserves_independent_glyphs_and_uses_declared_fallbacks() {
    let mut font_system = fixture_font_system();
    let style = TextStyle::new(16.0, Color::WHITE)
        .family(FontFamily::named("Inter"))
        .font_fallbacks(FontFallbacks::from_fonts(["Noto Sans Hebrew"]))
        .shaping(TextShaping::Basic);
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 22.0));
    configure_text_buffer(&mut buffer, &mut font_system, "Aא", &style, None, None, 1.0);

    let families = buffer
        .layout_runs()
        .flat_map(|run| run.glyphs.iter())
        .map(|glyph| {
            (
                glyph.start,
                glyph.glyph_id,
                font_system.db().face(glyph.font_id).unwrap().families[0]
                    .0
                    .clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(families.len(), 2);
    assert_eq!((families[0].0, families[0].2.as_str()), (0, "Inter"));
    assert_eq!(
        (families[1].0, families[1].2.as_str()),
        (1, "Noto Sans Hebrew")
    );
    assert!(families.iter().all(|(_, glyph_id, _)| *glyph_id != 0));
}

#[cfg(target_os = "macos")]
#[test]
fn basic_shaping_uses_the_platform_emoji_fallback() {
    let mut font_system = create_font_system();
    font_system
        .db_mut()
        .load_font_data(include_bytes!("../../tests/fixtures/fonts/Inter-Regular.ttf").to_vec());
    let style = TextStyle::new(16.0, Color::WHITE)
        .family(FontFamily::named("Inter"))
        .shaping(TextShaping::Basic);
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 22.0));
    configure_text_buffer(&mut buffer, &mut font_system, "🐳", &style, None, None, 1.0);

    let glyph = buffer
        .layout_runs()
        .flat_map(|run| run.glyphs.iter())
        .next()
        .expect("emoji glyph");
    let face = font_system.db().face(glyph.font_id).expect("emoji face");
    assert_ne!(glyph.glyph_id, 0);
    assert!(
        face.families
            .iter()
            .any(|(family, _)| family == "Apple Color Emoji")
    );
    assert!(
        SwashCache::new()
            .get_image_uncached(&mut font_system, glyph.physical((0.0, 0.0), 1.0).cache_key,)
            .is_some()
    );
}

#[test]
fn opentype_features_affect_plain_and_rich_text_shaping() {
    fn glyph_ids(
        font_system: &mut FontSystem,
        style: &TextStyle,
        highlights: Option<&[TextHighlight]>,
        content: &str,
    ) -> Vec<(usize, u16)> {
        let mut buffer = Buffer::new(font_system, Metrics::new(16.0, 22.0));
        configure_text_buffer(
            &mut buffer,
            font_system,
            content,
            style,
            highlights,
            None,
            1.0,
        );
        buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.iter())
            .map(|glyph| (glyph.metadata, glyph.glyph_id))
            .collect()
    }

    let mut font_system = fixture_font_system();
    let enabled = TextStyle::new(16.0, Color::WHITE).family(FontFamily::named("Inter"));
    let disabled = enabled
        .clone()
        .font_features(FontFeatures::new().disable(FontFeatureTag::CONTEXTUAL_ALTERNATES));
    let enabled_ids = glyph_ids(&mut font_system, &enabled, None, "|>")
        .into_iter()
        .map(|(_, glyph_id)| glyph_id)
        .collect::<Vec<_>>();
    let disabled_ids = glyph_ids(&mut font_system, &disabled, None, "|>")
        .into_iter()
        .map(|(_, glyph_id)| glyph_id)
        .collect::<Vec<_>>();
    assert_ne!(enabled_ids, disabled_ids);

    let highlighted = StyledText::new("|>|>").with_highlights([(
        2..4,
        HighlightStyle::default()
            .font_features(FontFeatures::new().disable(FontFeatureTag::CONTEXTUAL_ALTERNATES)),
    )]);
    let rich_ids = glyph_ids(
        &mut font_system,
        &enabled,
        Some(highlighted.highlights()),
        highlighted.content(),
    );
    assert_eq!(
        rich_ids
            .iter()
            .filter_map(|(metadata, glyph_id)| (*metadata == 0).then_some(*glyph_id))
            .collect::<Vec<_>>(),
        enabled_ids
    );
    assert_eq!(
        rich_ids
            .iter()
            .filter_map(|(metadata, glyph_id)| (*metadata == 1).then_some(*glyph_id))
            .collect::<Vec<_>>(),
        disabled_ids
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_opentype_bytes_register_in_the_application_font_database() {
    let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNSMono.ttf") else {
        return;
    };
    create_shared_font_system(&Assets::default(), &[FontSource::from(bytes)]).unwrap();
}

#[test]
fn surface_format_uses_encoded_ui_blending() {
    assert_eq!(
        preferred_surface_format(&[TextureFormat::Rgba8Unorm, TextureFormat::Bgra8UnormSrgb]),
        Some(TextureFormat::Rgba8Unorm)
    );
}

#[test]
fn surface_alpha_modes_keep_opaque_and_transparent_paths_explicit() {
    let modes = [
        CompositeAlphaMode::Opaque,
        CompositeAlphaMode::PostMultiplied,
        CompositeAlphaMode::PreMultiplied,
    ];
    assert_eq!(
        opaque_surface_alpha_mode(&modes),
        Some(CompositeAlphaMode::Opaque)
    );
    assert_eq!(
        transparent_surface_alpha_mode(&modes),
        Some(CompositeAlphaMode::PreMultiplied)
    );
    assert_eq!(
        transparent_surface_alpha_mode(&[
            CompositeAlphaMode::Opaque,
            CompositeAlphaMode::PostMultiplied,
        ]),
        Some(CompositeAlphaMode::PostMultiplied)
    );
    assert_eq!(
        transparent_surface_alpha_mode(&[CompositeAlphaMode::Opaque]),
        None
    );
}

#[cfg(target_os = "macos")]
#[test]
fn offscreen_gpu_pipelines_apply_one_shared_subtree_opacity() {
    let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        font_system,
    ))
    .unwrap();
    let mut scene = Scene::new();
    scene.clear(Color::BLACK);
    let previous = scene.multiply_opacity(0.5);

    scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 16.0, 16.0), Color::WHITE));
    let image = Image::from_rgba(1, 1, Arc::<[u8]>::from([255, 255, 255, 255])).unwrap();
    scene.push_image(ImagePrimitive::new(image, Rect::new(16.0, 0.0, 16.0, 16.0)));
    let svg = Svg::from_svg(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="white"/></svg>"#,
    )
    .unwrap();
    scene.push_svg(SvgPrimitive::new(
        svg,
        Rect::new(32.0, 0.0, 16.0, 16.0),
        Color::WHITE,
    ));
    let mut path = PathBuilder::fill();
    path.move_to(Point::new(0.0, 0.0));
    path.line_to(Point::new(16.0, 0.0));
    path.line_to(Point::new(16.0, 16.0));
    path.line_to(Point::new(0.0, 16.0));
    path.close();
    scene.push_path(PathPrimitive::new(path.build().unwrap(), Color::WHITE).translate(48.0, 0.0));
    let shader = CustomShader::new(
        "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(1.0, 1.0, 1.0, 1.0 + input.uv.x * 0.0); }",
    )
    .unwrap();
    scene.push_custom_shader(CustomShaderPrimitive::new(
        shader,
        Rect::new(64.0, 0.0, 16.0, 16.0),
    ));
    scene.restore_opacity(previous);
    scene.finish();

    let snapshot = renderer
        .render_to_snapshot(&scene, Size::new(80.0, 16.0), 1.0)
        .unwrap();
    let samples = [8, 24, 40, 56, 72].map(|x| snapshot.pixel(x, 8).unwrap());
    for sample in samples {
        // UI source-over is evaluated in encoded sRGB: white at 50% over black is 128.
        assert!(sample[0].abs_diff(128) <= 2, "{sample:?}");
        assert_eq!(sample[0], sample[1]);
        assert_eq!(sample[1], sample[2]);
        assert_eq!(sample[3], 255);
    }
    for sample in samples.windows(2) {
        assert!(sample[0][0].abs_diff(sample[1][0]) <= 2, "{samples:?}");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn adjacent_square_quads_do_not_show_internal_seams() {
    let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        font_system,
    ))
    .unwrap();
    let mut scene = Scene::new();
    scene.clear(Color::BLACK);

    // Terminal cell backgrounds commonly land between physical pixels. They are still one
    // continuous opaque surface, so rasterizing each square cell must not expose its edges.
    // JetBrains Mono's declared advance is 0.6em, which is also the terminal grid metric.
    let cell_width = 14.0 * 0.6;
    let cell_height = 20.5;
    let fill = Color::rgb8(225, 226, 231);
    for row in 0..2 {
        for column in 0..5 {
            scene.push_quad(Quad::new(
                Rect::new(
                    column as f32 * cell_width,
                    row as f32 * cell_height,
                    cell_width,
                    cell_height,
                ),
                fill,
            ));
        }
    }
    scene.finish();

    let snapshot = renderer
        .render_to_snapshot(&scene, Size::new(cell_width * 5.0, cell_height * 2.0), 2.0)
        .unwrap();
    let interior = snapshot.pixel(4, 4).unwrap();
    for column in 1..5 {
        let boundary_x = (column as f32 * cell_width * 2.0).floor() as u32;
        assert_eq!(
            snapshot.pixel(boundary_x, 10).unwrap(),
            interior,
            "vertical cell edge at x={boundary_x} must be invisible"
        );
    }
    let boundary_y = (cell_height * 2.0) as u32;
    assert_eq!(
        snapshot.pixel(10, boundary_y).unwrap(),
        interior,
        "horizontal row edge at y={boundary_y} must be invisible"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn edge_quad_paints_each_border_width_independently() {
    use crate::Insets;
    use crate::scene::{EdgeQuad, PaintLayerKey};

    let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        font_system,
    ))
    .unwrap();
    let mut scene = Scene::new();
    scene.clear(Color::BLACK);
    scene.push_edge_quad_in(
        PaintLayerKey::default(),
        EdgeQuad::new(Rect::new(4.0, 4.0, 24.0, 24.0), Color::rgb8(16, 64, 160)).border(
            Insets {
                top: 2.0,
                right: 0.0,
                bottom: 4.0,
                left: 3.0,
            },
            Color::rgb8(240, 32, 24),
        ),
    );
    scene.finish();

    let snapshot = renderer
        .render_to_snapshot(&scene, Size::new(32.0, 32.0), 1.0)
        .unwrap();
    let top = snapshot.pixel(16, 5).unwrap();
    let left = snapshot.pixel(5, 16).unwrap();
    let bottom = snapshot.pixel(16, 26).unwrap();
    for sample in [top, left, bottom] {
        assert!(
            sample[0] > 200
                && u16::from(sample[0]) > u16::from(sample[1]) * 3
                && u16::from(sample[0]) > u16::from(sample[2]) * 2,
            "{sample:?}"
        );
    }

    let center = snapshot.pixel(16, 16).unwrap();
    let borderless_right = snapshot.pixel(26, 16).unwrap();
    for sample in [center, borderless_right] {
        assert!(
            sample[2] > 130 && u16::from(sample[2]) > u16::from(sample[0]) * 4,
            "{sample:?}"
        );
    }
    assert_eq!(snapshot.pixel(1, 1).unwrap(), [0, 0, 0, 255]);
}

#[cfg(target_os = "macos")]
#[test]
fn square_edge_border_has_no_partially_transparent_inner_seam() {
    use crate::Insets;
    use crate::scene::{EdgeQuad, PaintLayerKey};

    let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        font_system,
    ))
    .unwrap();
    let mut scene = Scene::new();
    scene.clear(Color::TRANSPARENT);
    scene.push_edge_quad_in(
        PaintLayerKey::default(),
        EdgeQuad::new(Rect::new(0.0, 0.0, 254.0, 8.0), Color::TRANSPARENT).border(
            Insets {
                top: 0.0,
                right: 1.0,
                bottom: 0.0,
                left: 0.0,
            },
            Color::rgb8(204, 204, 204),
        ),
    );
    scene.finish();

    let snapshot = renderer
        .render_to_snapshot(&scene, Size::new(256.0, 8.0), 2.0)
        .unwrap();
    assert_eq!(snapshot.pixel(505, 8).unwrap(), [0, 0, 0, 0]);
    assert_eq!(snapshot.pixel(506, 8).unwrap(), [204, 204, 204, 255]);
    assert_eq!(snapshot.pixel(507, 8).unwrap(), [204, 204, 204, 255]);
    assert_eq!(snapshot.pixel(508, 8).unwrap(), [0, 0, 0, 0]);
}

#[cfg(target_os = "macos")]
#[test]
fn oversized_target_keeps_primitive_clips_in_logical_viewport_space() {
    let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        font_system,
    ))
    .unwrap();
    let mut scene = Scene::new();
    scene.clear(Color::BLACK);

    // Model the live-resize path: layout and projection use a 64px current viewport while
    // Metal retains a 128px render target. Drawable-pixel clipping would incorrectly discard
    // every primitive below y=32 logical after projection doubles its target position.
    scene.push_quad(Quad::new(Rect::new(0.0, 40.0, 16.0, 16.0), Color::WHITE));
    let image = Image::from_rgba(1, 1, Arc::<[u8]>::from([255, 255, 255, 255])).unwrap();
    scene.push_image(ImagePrimitive::new(
        image,
        Rect::new(16.0, 40.0, 16.0, 16.0),
    ));
    let svg = Svg::from_svg(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="white"/></svg>"#,
    )
    .unwrap();
    scene.push_svg(SvgPrimitive::new(
        svg,
        Rect::new(32.0, 40.0, 16.0, 16.0),
        Color::WHITE,
    ));
    let shader = CustomShader::new(
        "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(1.0, 1.0, 1.0, 1.0 + input.uv.x * 0.0); }",
    )
    .unwrap();
    scene.push_custom_shader(CustomShaderPrimitive::new(
        shader,
        Rect::new(48.0, 40.0, 16.0, 16.0),
    ));
    scene.finish();

    let snapshot = renderer
        .render_to_snapshot_with_target(&scene, Size::new(64.0, 64.0), 1.0, (128, 128))
        .unwrap();
    let samples = [16, 48, 80, 112].map(|x| snapshot.pixel(x, 96).unwrap());
    for sample in samples {
        assert!(sample[0] > 240, "{sample:?}");
        assert!(sample[1] > 240, "{sample:?}");
        assert!(sample[2] > 240, "{sample:?}");
        assert_eq!(sample[3], 255);
    }
}

#[test]
fn surface_capacity_grows_geometrically_and_never_shrinks() {
    assert_eq!(
        grow_surface_capacity((900, 640), (760, 520), 16_384),
        (900, 640)
    );
    assert_eq!(
        grow_surface_capacity((900, 640), (920, 650), 16_384),
        (1_350, 960)
    );
    assert_eq!(
        grow_surface_capacity((1_350, 960), (1_180, 720), 16_384),
        (1_350, 960)
    );
    assert_eq!(
        grow_surface_capacity((12_000, 12_000), (16_000, 15_000), 16_384),
        (16_384, 16_384)
    );
}

#[test]
fn text_clip_rounds_outward() {
    let bounds = physical_text_bounds(Rect::new(1.1, 2.2, 3.3, 4.4), 2.0);
    assert_eq!(
        (bounds.left, bounds.top, bounds.right, bounds.bottom),
        (2, 4, 9, 14)
    );
}

#[test]
fn text_layout_width_is_retained_only_when_wrapping_or_alignment_needs_it() {
    assert_eq!(
        canonical_text_width(TextWrap::None, TextAlign::Left, false, Some(640.0)),
        None
    );
    assert_eq!(
        canonical_text_width(TextWrap::None, TextAlign::Center, false, Some(640.0)),
        Some(640.0)
    );
    assert_eq!(
        canonical_text_width(TextWrap::None, TextAlign::Right, false, Some(640.0)),
        Some(640.0)
    );
    assert_eq!(
        canonical_text_width(TextWrap::None, TextAlign::Justify, false, Some(640.0)),
        Some(640.0)
    );
    assert_eq!(
        canonical_text_width(TextWrap::Word, TextAlign::Left, false, Some(640.0)),
        Some(640.0)
    );
    assert_eq!(
        canonical_text_width(TextWrap::Glyph, TextAlign::Left, false, Some(640.0)),
        Some(640.0)
    );
    assert_eq!(
        canonical_text_width(TextWrap::None, TextAlign::Left, true, Some(640.0)),
        Some(640.0)
    );
}

#[test]
fn text_layout_keys_allow_reflow_only_for_width_changes() {
    let content: Arc<str> = Arc::from("stable wrapped text");
    let key = TextLayoutKey {
        content,
        highlights: None,
        width: Some(320.0),
        font_size: 14.0,
        line_height: 20.0,
        monospace_width: None,
        family: FontFamily::SansSerif,
        features: FontFeatures::new(),
        fallbacks: None,
        weight: glyphon::Weight::NORMAL,
        font_style: GlyphStyle::Normal,
        font_thicken: false,
        underline: TextUnderline::None,
        underline_color: None,
        underline_wavy: false,
        underline_thickness: 1.0,
        strikethrough: false,
        strikethrough_color: None,
        align: TextAlign::Left,
        wrap: TextWrap::Word,
        text_overflow: None,
        line_clamp: None,
        shaping: TextShaping::Advanced,
        extras: TextShapingExtras::from_style(&TextStyle::default()),
        scale: 2.0,
    };
    let mut narrower = key.clone();
    narrower.width = Some(180.0);
    assert!(key.same_except_width(&narrower));

    let mut different_style = narrower.clone();
    different_style.font_size = 15.0;
    assert!(!key.same_except_width(&different_style));

    let mut optically_thick = key.clone();
    optically_thick.font_thicken = true;
    assert!(!key.same_except_width(&optically_thick));

    let mut fixed_cells = key.clone();
    fixed_cells.monospace_width = Some(8.54);
    assert!(!key.same_except_width(&fixed_cells));

    let mut italic = key.clone();
    italic.font_style = GlyphStyle::Italic;
    assert!(!key.same_except_width(&italic));

    let mut different_features = key.clone();
    different_features.features =
        FontFeatures::new().disable(FontFeatureTag::CONTEXTUAL_ALTERNATES);
    assert!(!key.same_except_width(&different_features));

    let mut different_fallbacks = key.clone();
    different_fallbacks.fallbacks = Some(FontFallbacks::from_fonts(["Noto Sans Hebrew"]));
    assert!(!key.same_except_width(&different_fallbacks));

    let mut underlined = key.clone();
    underlined.underline = TextUnderline::Single;
    underlined.underline_color = Some(Color::rgb8(56, 189, 248));
    assert!(!key.same_except_width(&underlined));

    let mut wavy = underlined.clone();
    wavy.underline_wavy = true;
    assert!(!underlined.same_except_width(&wavy));

    let mut thick = underlined.clone();
    thick.underline_thickness = 4.0;
    assert!(!underlined.same_except_width(&thick));

    let mut different_content = narrower;
    different_content.content = Arc::from("different wrapped text");
    assert!(!key.same_except_width(&different_content));

    let mut overflowing = key.clone();
    overflowing.text_overflow = Some(TextOverflow::ellipsis());
    let mut overflowing_narrower = overflowing.clone();
    overflowing_narrower.width = Some(180.0);
    assert!(overflowing.same_except_width(&overflowing_narrower));

    let mut custom = key.clone();
    custom.text_overflow = Some(TextOverflow::Truncate(Arc::from("...")));
    let mut custom_narrower = custom.clone();
    custom_narrower.width = Some(180.0);
    assert!(!custom.same_except_width(&custom_narrower));
}

#[test]
fn fixed_monospace_width_controls_glyph_and_background_cells() {
    let content: Arc<str> = Arc::from("ABCDE");
    // The terminal grid must use the bundled font's declared 0.6em advance.
    let cell_width = 14.0 * 0.6;
    let scale = 2.0;
    let styled =
        StyledText::new(content.clone()).with_highlights((0..content.len()).map(|index| {
            (
                index..index + 1,
                HighlightStyle::default().background(if index % 2 == 0 {
                    Color::rgb8(225, 226, 231)
                } else {
                    Color::rgb8(128, 128, 128)
                }),
            )
        }));
    let style = TextStyle::new(14.0, Color::BLACK)
        .family(FontFamily::named("JetBrainsMono Nerd Font Mono"))
        .line_height(20.5)
        .monospace_width(cell_width)
        .wrap(TextWrap::None)
        .shaping(TextShaping::Basic);
    let mut font_system = fixture_terminal_font_system();
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size * scale, style.line_height * scale),
    );

    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &style,
        Some(styled.highlights()),
        None,
        scale,
    );

    assert_eq!(buffer.monospace_width(), Some(cell_width * scale));
    let run = buffer.layout_runs().next().expect("one terminal row");
    let expected_width = content.len() as f32 * cell_width * scale;
    assert!(
        (run.line_w - expected_width).abs() < 0.01,
        "expected {expected_width}, got {}",
        run.line_w,
    );
    for cells in run.glyphs.windows(2) {
        assert!((cells[1].x - cells[0].x - cell_width * scale).abs() < 0.01);
    }

    let geometry = collect_styled_text_geometry(
        &buffer,
        styled.highlights(),
        &style,
        scale,
        0.0..1_000.0,
        None,
    );
    assert_eq!(geometry.backgrounds.len(), content.len());
    for cells in geometry.backgrounds.windows(2) {
        assert!((cells[0].rect.right() - cells[1].rect.x).abs() < 0.01);
    }
    assert!((geometry.backgrounds.last().unwrap().rect.right() - cell_width * 5.0).abs() < 0.01);
}

#[test]
fn font_thicken_preserves_weight_and_outline_geometry() {
    let scale = 2.0;
    let style = TextStyle::new(14.0, Color::BLACK)
        .family(FontFamily::named("JetBrainsMono Nerd Font Mono"))
        .font_thicken(true)
        .wrap(TextWrap::None)
        .shaping(TextShaping::Basic);
    let mut font_system = fixture_terminal_font_system();
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size * scale, style.line_height * scale),
    );
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        "M",
        &style,
        None,
        None,
        scale,
    );

    let glyph = buffer
        .layout_runs()
        .flat_map(|run| run.glyphs.iter())
        .next()
        .expect("one terminal glyph");
    assert_eq!(glyph.font_weight, glyphon::Weight::NORMAL);
    let thick_key = glyph.physical((0.0, 0.0), 1.0).cache_key;
    assert!(thick_key.flags.contains(CacheKeyFlags::FONT_THICKEN));

    let mut normal_key = thick_key;
    normal_key.flags.remove(CacheKeyFlags::FONT_THICKEN);
    let mut swash = SwashCache::new();
    let normal_outline = swash
        .get_outline_commands_uncached(&mut font_system, normal_key)
        .expect("regular glyph outline");
    let thick_outline = swash
        .get_outline_commands_uncached(&mut font_system, thick_key)
        .expect("optically thick glyph outline");
    assert_eq!(thick_outline, normal_outline);

    let normal = swash
        .get_image_uncached(&mut font_system, normal_key)
        .expect("regular glyph mask");
    let thick = swash
        .get_image_uncached(&mut font_system, thick_key)
        .expect("thickened glyph mask");
    assert_eq!(thick.content, normal.content);
    assert!(thick.placement.width <= normal.placement.width + 2);
    assert!(thick.placement.height <= normal.placement.height + 2);
    let normal_x = usize::try_from(normal.placement.left - thick.placement.left)
        .expect("thick mask contains the regular mask's left edge");
    let normal_y = usize::try_from(thick.placement.top - normal.placement.top)
        .expect("thick mask contains the regular mask's top edge");
    for row in 0..normal.placement.height as usize {
        for column in 0..normal.placement.width as usize {
            let normal_alpha = normal.data[row * normal.placement.width as usize + column];
            let thick_alpha =
                thick.data[(normal_y + row) * thick.placement.width as usize + normal_x + column];
            assert!(thick_alpha >= normal_alpha);
        }
    }
    let normal_coverage = normal
        .data
        .iter()
        .map(|value| u64::from(*value))
        .sum::<u64>();
    let thick_coverage = thick
        .data
        .iter()
        .map(|value| u64::from(*value))
        .sum::<u64>();
    assert!(
        thick_coverage > normal_coverage,
        "expected thickened coverage {thick_coverage} to exceed regular coverage {normal_coverage}",
    );
}

#[test]
fn fixed_monospace_width_keeps_fallback_symbols_on_the_terminal_grid() {
    let content: Arc<str> = Arc::from("■⬝■⬝");
    let cell_width = 14.0 * 0.6;
    let scale = 2.0;
    let styled =
        StyledText::new(content.clone()).with_highlights(content.char_indices().enumerate().map(
            |(index, (start, character))| {
                (
                    start..start + character.len_utf8(),
                    HighlightStyle::default().color(if index % 2 == 0 {
                        Color::rgb8(153, 102, 204)
                    } else {
                        Color::rgb8(174, 157, 197)
                    }),
                )
            },
        ));
    let style = TextStyle::new(14.0, Color::BLACK)
        .family(FontFamily::named("JetBrainsMono Nerd Font Mono"))
        .line_height(20.5)
        .monospace_width(cell_width)
        .wrap(TextWrap::None)
        .shaping(TextShaping::Basic);
    let mut font_system = create_font_system();
    font_system.db_mut().load_font_data(
        include_bytes!("../../examples/herdr-gui/assets/JetBrainsMonoNerdFontMono-Regular.ttf")
            .to_vec(),
    );
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size * scale, style.line_height * scale),
    );

    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &style,
        Some(styled.highlights()),
        None,
        scale,
    );

    let run = buffer.layout_runs().next().expect("one terminal row");
    assert_eq!(run.glyphs.len(), 4);
    let expected_width = 4.0 * cell_width * scale;
    assert!(
        (run.line_w - expected_width).abs() < 0.01,
        "terminal row should measure {expected_width}, got {}",
        run.line_w,
    );
    for (column, glyph) in run.glyphs.iter().enumerate() {
        let expected_x = column as f32 * cell_width * scale;
        assert!(
            (glyph.x - expected_x).abs() < 0.01,
            "terminal column {column} should start at {expected_x}, got {}",
            glyph.x,
        );
        assert!(
            (glyph.w - cell_width * scale).abs() < 0.01,
            "terminal column {column} should occupy one cell, got {}",
            glyph.w,
        );
    }
}

#[test]
fn unicode_text_projection_maps_retained_ranges_and_ellipsis_to_original_bytes() {
    let content: Arc<str> = Arc::from("alpha🙂beta界gamma");
    let prefix_end = content.find("beta").unwrap() + "beta".len();
    let suffix_start = content.find("gamma").unwrap();
    let projection = build_text_projection(
        &content,
        None,
        &Arc::from("…"),
        TruncationPlacement::Middle {
            prefix_end,
            suffix_start,
        },
    );

    assert_eq!(projection.content.as_ref(), "alpha🙂beta…gamma");
    let ellipsis_start = projection.content.find('…').unwrap();
    assert_eq!(projection.display_to_original(ellipsis_start), prefix_end);
    assert_eq!(
        projection.display_to_original(ellipsis_start + "…".len()),
        suffix_start
    );
    assert_eq!(
        projection.original_to_display(content.find('界').unwrap()),
        ellipsis_start
    );
    assert_eq!(
        projection.display_ranges_for_original(0..content.len()),
        vec![0..projection.content.len()]
    );
}

#[test]
fn custom_affix_projection_remaps_styled_ranges_without_splitting_unicode() {
    let content: Arc<str> = Arc::from("prefix🙂hidden界suffix");
    let suffix_start = content.find("suffix").unwrap();
    let styled = StyledText::new(content.clone()).with_highlights([
        (0.."prefix".len(), HighlightStyle::default().font_bold()),
        (
            suffix_start..content.len(),
            HighlightStyle::default().underline(),
        ),
    ]);
    let projection = build_text_projection(
        &content,
        Some(styled.shared_highlights()),
        &Arc::from("[...]"),
        TruncationPlacement::Middle {
            prefix_end: "prefix".len(),
            suffix_start,
        },
    );

    assert_eq!(projection.content.as_ref(), "prefix[...]suffix");
    let highlights = projection.highlights.as_deref().unwrap();
    assert_eq!(highlights.len(), 2);
    assert_eq!(highlights[0].range, 0.."prefix[...]".len());
    assert_eq!(&projection.content[highlights[1].range.clone()], "suffix");
    assert_eq!(
        projection.display_ranges_for_original(suffix_start..content.len()),
        vec!["prefix[...]".len()..projection.content.len()]
    );
}

#[test]
fn standard_ellipsis_uses_cosmic_texts_single_reflowable_original_buffer() {
    let content: Arc<str> =
        Arc::from("A very long filename with emoji 🙂 and an important-final-suffix.quickgui");
    let mut font_system = create_font_system();
    for (overflow, ellipsize) in [
        (
            TextOverflow::ellipsis(),
            Ellipsize::End(EllipsizeHeightLimit::Lines(1)),
        ),
        (
            TextOverflow::ellipsis_start(),
            Ellipsize::Start(EllipsizeHeightLimit::Lines(1)),
        ),
        (
            TextOverflow::ellipsis_middle(),
            Ellipsize::Middle(EllipsizeHeightLimit::Lines(1)),
        ),
    ] {
        let style = TextStyle::new(14.0, Color::WHITE)
            .wrap(TextWrap::None)
            .text_overflow(overflow);
        let (projection, buffer) =
            prepare_text_buffer(&mut font_system, &content, None, &style, Some(150.0), 1.0);
        assert!(Arc::ptr_eq(&projection.content, &content));
        assert!(matches!(projection.mapping, TextProjection::Identity));
        assert_eq!(buffer.ellipsize(), ellipsize);
        assert!(text_buffer_fits(&buffer, 150.0, 1, 1.0));
    }
}

#[test]
fn custom_overflow_affixes_use_the_unicode_safe_visible_projection() {
    let content: Arc<str> =
        Arc::from("A very long filename with emoji 🙂 and an important-final-suffix.quickgui");
    let mut font_system = create_font_system();
    for (overflow, expected_start, expected_end, expected_affix) in [
        (TextOverflow::Truncate(Arc::from("...")), "A", "...", "..."),
        (
            TextOverflow::TruncateStart(Arc::from("...")),
            "...",
            "quickgui",
            "...",
        ),
        (
            TextOverflow::TruncateMiddle(Arc::from("[...]")),
            "A",
            "gui",
            "[...]",
        ),
    ] {
        let style = TextStyle::new(14.0, Color::WHITE)
            .wrap(TextWrap::None)
            .text_overflow(overflow);
        let (projection, buffer) =
            prepare_text_buffer(&mut font_system, &content, None, &style, Some(150.0), 1.0);
        assert!(projection.content.contains(expected_affix));
        assert!(
            projection.content.starts_with(expected_start),
            "projected {:?} did not start with {expected_start:?}",
            projection.content
        );
        assert!(
            projection.content.ends_with(expected_end),
            "projected {:?} did not end with {expected_end:?}",
            projection.content
        );
        assert!(projection.content.len() < content.len());
        assert_eq!(buffer.ellipsize(), Ellipsize::None);
        assert!(text_buffer_fits(&buffer, 150.0, 1, 1.0));
    }
}

#[test]
fn multiline_ellipsis_fits_the_requested_line_clamp() {
    let content: Arc<str> = Arc::from(
        "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron",
    );
    let style = TextStyle::new(14.0, Color::WHITE)
        .line_height(20.0)
        .wrap(TextWrap::Word)
        .text_overflow(TextOverflow::ellipsis())
        .line_clamp(2);
    let mut font_system = create_font_system();
    let (projection, buffer) =
        prepare_text_buffer(&mut font_system, &content, None, &style, Some(110.0), 1.0);

    assert!(Arc::ptr_eq(&projection.content, &content));
    assert_eq!(
        buffer.ellipsize(),
        Ellipsize::End(EllipsizeHeightLimit::Lines(2))
    );
    assert!(buffer.layout_runs().count() <= 2);
    assert!(text_buffer_fits(&buffer, 110.0, 2, 1.0));
}

#[test]
fn fitting_overflow_text_keeps_the_original_shared_content() {
    let content: Arc<str> = Arc::from("Short label");
    let style = TextStyle::new(14.0, Color::WHITE)
        .wrap(TextWrap::None)
        .text_overflow(TextOverflow::ellipsis());
    let mut font_system = create_font_system();
    let (projection, buffer) =
        prepare_text_buffer(&mut font_system, &content, None, &style, Some(400.0), 1.0);

    assert!(Arc::ptr_eq(&projection.content, &content));
    assert!(matches!(projection.mapping, TextProjection::Identity));
    assert!(text_buffer_fits(&buffer, 400.0, 1, 1.0));
}

#[test]
fn width_only_reflow_preserves_the_shaped_glyph_cache() {
    let content = "Resize reuses shaped glyphs while recomputing wrapped line placement";
    let style = TextStyle::new(14.0, Color::WHITE)
        .line_height(20.0)
        .wrap(TextWrap::Word);
    let mut font_system = create_font_system();
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size, style.line_height),
    );
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        content,
        &style,
        None,
        Some(480.0),
        1.0,
    );
    let wide_lines = buffer.layout_runs().count();
    let shaped_line = buffer.lines[0]
        .shape_opt()
        .expect("configuration shapes the first paragraph") as *const _
        as usize;

    reflow_text_buffer(&mut buffer, &mut font_system, Some(90.0), 1.0);

    assert_eq!(buffer.size(), (Some(90.0), None));
    assert!(buffer.layout_runs().count() > wide_lines);
    assert_eq!(
        buffer.lines[0]
            .shape_opt()
            .expect("relayout retains the shaped paragraph") as *const _ as usize,
        shaped_line
    );
}

#[test]
fn text_alignment_changes_shaped_line_origins_with_one_bounded_buffer() {
    let mut font_system = create_font_system();
    let mut origin = |align| {
        let style = TextStyle::new(14.0, Color::WHITE)
            .wrap(TextWrap::None)
            .align(align);
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(style.font_size, style.line_height),
        );
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            "Align me",
            &style,
            None,
            Some(240.0),
            1.0,
        );
        buffer
            .cursor_position(&Cursor::new(0, 0))
            .expect("the line has a caret origin")
            .0
    };

    let left = origin(TextAlign::Left);
    let center = origin(TextAlign::Center);
    let right = origin(TextAlign::Right);
    assert!(left < center, "centered text must move right of left text");
    assert!(
        center < right,
        "right text must move right of centered text"
    );
    assert_eq!(glyph_alignment(TextAlign::Justify), GlyphAlign::Justified);
}

#[test]
fn multiline_layout_maps_carets_hits_and_selection_per_line() {
    let content = "first line\nsecond line";
    let style = TextStyle::new(14.0, Color::WHITE).line_height(20.0);
    let mut font_system = create_font_system();
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size, style.line_height),
    );
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        content,
        &style,
        None,
        Some(240.0),
        1.0,
    );

    let second_line = content.find("second").expect("second line exists");
    let caret = text_cursor_for_byte_index(content, second_line + 3);
    let (_, caret_y) = buffer.cursor_position(&caret).expect("caret is laid out");
    assert!(caret_y >= style.line_height);
    let hit = buffer
        .hit(0.0, caret_y + style.line_height * 0.5)
        .expect("second line is hittable");
    assert_eq!(hit.line, 1);
    assert_eq!(byte_index_for_text_cursor(content, hit), second_line);

    let selection_start = text_cursor_for_byte_index(content, 2);
    let selection_end = text_cursor_for_byte_index(content, content.len() - 2);
    let highlighted_lines = buffer
        .layout_runs()
        .filter(|run| !text_selection_spans(run, selection_start, selection_end).is_empty())
        .count();
    assert_eq!(highlighted_lines, 2);

    let trailing_newline = "first line\n";
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        trailing_newline,
        &style,
        None,
        Some(240.0),
        1.0,
    );
    let trailing_caret = text_cursor_for_byte_index(trailing_newline, trailing_newline.len());
    let (_, trailing_y) = buffer
        .cursor_position(&trailing_caret)
        .expect("a trailing empty line has caret geometry");
    assert!(trailing_y >= style.line_height);

    let wrapped = "alpha beta gamma delta epsilon";
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        wrapped,
        &style,
        None,
        Some(60.0),
        1.0,
    );
    assert!(buffer.layout_runs().count() >= 3);
    let gamma = wrapped.find("gamma").expect("gamma exists");
    let wrapped_caret = text_cursor_for_byte_index(wrapped, gamma + 2);
    let (wrapped_x, wrapped_y) = buffer
        .cursor_position(&wrapped_caret)
        .expect("a wrapped caret has visual-line geometry");
    assert!(wrapped_y >= style.line_height);
    let wrapped_hit = buffer
        .hit(wrapped_x, wrapped_y + style.line_height * 0.5)
        .expect("the wrapped visual line is hittable");
    let wrapped_index = byte_index_for_text_cursor(wrapped, wrapped_hit);
    assert!((gamma..=gamma + "gamma".len()).contains(&wrapped_index));
}

#[test]
fn styled_text_uses_one_wrapped_buffer_for_glyphs_backgrounds_and_decorations() {
    let content: Arc<str> = Arc::from("status ready deprecated");
    let ready = content.find("ready").unwrap();
    let deprecated = content.find("deprecated").unwrap();
    let styled = StyledText::new(content.clone()).with_highlights([
        (
            0..6,
            HighlightStyle::default()
                .background(Color::rgba8(30, 64, 175, 96))
                .font_semibold(),
        ),
        (
            ready..ready + "ready".len(),
            HighlightStyle::default()
                .color(Color::rgb8(56, 189, 248))
                .double_underline(),
        ),
        (
            deprecated..content.len(),
            HighlightStyle::default()
                .italic()
                .strikethrough_color(Color::rgb8(248, 113, 113)),
        ),
    ]);
    let style = TextStyle::new(14.0, Color::WHITE).line_height(20.0);
    let mut font_system = create_font_system();
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size, style.line_height),
    );

    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &style,
        Some(styled.highlights()),
        Some(90.0),
        1.0,
    );

    assert!(buffer.layout_runs().count() >= 2);
    let metadata: HashSet<_> = buffer
        .layout_runs()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.metadata))
        .collect();
    assert!(metadata.contains(&1));
    assert!(metadata.contains(&2));
    assert!(metadata.contains(&3));

    let geometry = collect_styled_text_geometry(
        &buffer,
        styled.highlights(),
        &style,
        1.0,
        0.0..1_000.0,
        None,
    );
    assert!(!geometry.backgrounds.is_empty());
    assert!(geometry.decorations.len() >= 3);
    assert_eq!(geometry.backgrounds[0].color, Color::rgba8(30, 64, 175, 96));

    let clipped = collect_styled_text_geometry(
        &buffer,
        styled.highlights(),
        &style,
        1.0,
        10_000.0..10_020.0,
        None,
    );
    assert!(clipped.backgrounds.is_empty());
    assert!(clipped.decorations.is_empty());
}

#[test]
fn inherited_base_text_decorations_use_the_same_bounded_buffer() {
    let content: Arc<str> = Arc::from("inherited decoration");
    let underline = Color::rgb8(56, 189, 248);
    let strike = Color::rgb8(248, 113, 113);
    let style = TextStyle::new(16.0, Color::WHITE)
        .line_height(24.0)
        .font_style(GlyphStyle::Italic)
        .underline_color(underline)
        .strikethrough_color(strike);
    let mut font_system = create_font_system();
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size, style.line_height),
    );

    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &style,
        None,
        Some(240.0),
        1.0,
    );
    let geometry = collect_styled_text_geometry(&buffer, &[], &style, 1.0, 0.0..1_000.0, None);

    assert!(geometry.backgrounds.is_empty());
    assert!(geometry.decorations.len() >= 2);
    assert!(
        geometry
            .decorations
            .iter()
            .any(|decoration| decoration.color == underline)
    );
    assert!(
        geometry
            .decorations
            .iter()
            .any(|decoration| decoration.color == strike)
    );
}

#[test]
fn custom_underline_geometry_preserves_waves_thickness_and_zero() {
    let content: Arc<str> = Arc::from("diagnostic underline");
    let accent = Color::rgb8(248, 113, 113);
    let mut font_system = create_font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 24.0));

    let wavy = TextStyle::new(16.0, Color::TRANSPARENT)
        .line_height(24.0)
        .underline_color(accent)
        .text_decoration_4()
        .text_decoration_wavy();
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &wavy,
        None,
        Some(240.0),
        2.0,
    );
    let geometry = collect_styled_text_geometry(&buffer, &[], &wavy, 2.0, 0.0..1_000.0, None);
    assert_eq!(geometry.decorations.len(), 1);
    let decoration = geometry.decorations[0];
    assert_eq!(decoration.color, accent);
    assert!(matches!(
        decoration.kind,
        TextPaintKind::WavyUnderline {
            thickness,
            amplitude,
            wavelength,
            ..
        } if thickness == 4.0 && amplitude == 4.0 && wavelength == 16.0
    ));

    let solid = wavy.clone().text_decoration_solid().text_decoration_2();
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &solid,
        None,
        Some(240.0),
        2.0,
    );
    let geometry = collect_styled_text_geometry(&buffer, &[], &solid, 2.0, 0.0..1_000.0, None);
    assert_eq!(geometry.decorations.len(), 1);
    assert_eq!(geometry.decorations[0].kind, TextPaintKind::Solid);
    assert_eq!(geometry.decorations[0].rect.height, 2.0);

    let zero = solid.text_decoration_0();
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &zero,
        None,
        Some(240.0),
        2.0,
    );
    let geometry = collect_styled_text_geometry(&buffer, &[], &zero, 2.0, 0.0..1_000.0, None);
    assert!(geometry.decorations.is_empty());
}

#[test]
fn highlighted_wavy_ranges_split_an_inherited_solid_decoration_span() {
    let content: Arc<str> = Arc::from("solid wavy");
    let wavy_start = content.find("wavy").unwrap();
    let styled = StyledText::new(content.clone()).with_highlights([(
        wavy_start..content.len(),
        HighlightStyle::default()
            .text_decoration_wavy()
            .text_decoration_2(),
    )]);
    let style = TextStyle::new(16.0, Color::WHITE).underline();
    let mut font_system = create_font_system();
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size, style.line_height),
    );
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &style,
        Some(styled.highlights()),
        Some(240.0),
        1.0,
    );
    let geometry = collect_styled_text_geometry(
        &buffer,
        styled.highlights(),
        &style,
        1.0,
        0.0..1_000.0,
        None,
    );
    assert!(
        geometry
            .decorations
            .iter()
            .any(|decoration| decoration.kind == TextPaintKind::Solid)
    );
    assert!(geometry.decorations.iter().any(|decoration| matches!(
        decoration.kind,
        TextPaintKind::WavyUnderline { thickness: 2.0, .. }
    )));
}

#[test]
fn styled_multiline_selection_respects_exact_byte_range() {
    let content: Arc<str> =
        Arc::from("fn render() {\n    let state = \"GPU cached\";\n    deprecated_api();\n}");
    let selected_start = content.find("GPU cached").unwrap();
    let selected_end = selected_start + "GPU cached".len();
    let deprecated = content.find("deprecated_api").unwrap();
    let styled = StyledText::new(content.clone()).with_highlights([
        (0..2, HighlightStyle::default().font_bold()),
        (
            selected_start..selected_end,
            HighlightStyle::default().background(Color::rgba8(14, 116, 144, 72)),
        ),
        (
            deprecated..deprecated + "deprecated_api".len(),
            HighlightStyle::default().strikethrough(),
        ),
    ]);
    let style = TextStyle::new(14.0, Color::WHITE)
        .family(FontFamily::Monospace)
        .wrap(TextWrap::None)
        .line_height(22.0);
    let mut font_system = create_font_system();
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size, style.line_height),
    );
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &style,
        Some(styled.highlights()),
        Some(640.0),
        1.0,
    );

    let start = text_cursor_for_byte_index(&content, selected_start);
    let end = text_cursor_for_byte_index(&content, selected_end);
    let selections = buffer
        .layout_runs()
        .flat_map(|run| {
            let line_width = run.line_w;
            text_selection_spans(&run, start, end)
                .into_iter()
                .map(move |(_, width)| (run.line_i, width, line_width))
        })
        .collect::<Vec<_>>();

    assert_eq!(selections.len(), 1);
    assert_eq!(selections[0].0, 1);
    assert!(selections[0].1 < selections[0].2);
}

#[test]
fn background_color_only_changes_reuse_the_shaped_highlight_key() {
    use std::collections::hash_map::DefaultHasher;

    let cool = StyledText::new("cached").with_highlights([(
        0..6,
        HighlightStyle::default().background(Color::rgb8(14, 116, 144)),
    )]);
    let warm = StyledText::new("cached").with_highlights([(
        0..6,
        HighlightStyle::default().background(Color::rgb8(190, 24, 93)),
    )]);
    let cool = Some(cool.shared_highlights().clone());
    let warm = Some(warm.shared_highlights().clone());

    assert!(highlights_equal(&cool, &warm));
    let mut cool_hash = DefaultHasher::new();
    hash_highlights(&cool, &mut cool_hash);
    let mut warm_hash = DefaultHasher::new();
    hash_highlights(&warm, &mut warm_hash);
    assert_eq!(cool_hash.finish(), warm_hash.finish());

    let foreground = StyledText::new("cached").with_highlights([(
        0..6,
        HighlightStyle::default().color(Color::rgb8(14, 116, 144)),
    )]);
    assert!(!highlights_equal(
        &cool,
        &Some(foreground.shared_highlights().clone())
    ));
}

#[cfg(target_os = "macos")]
#[test]
fn translated_text_settles_without_a_final_pixel_step() {
    let fonts = Rc::new(RefCell::new(fixture_font_system()));
    let mut renderer =
        pollster::block_on(OffscreenRenderer::new(PerformanceProfile::Balanced, fonts)).unwrap();
    for scale in [1.0, 1.5, 2.0] {
        let render = |renderer: &mut OffscreenRenderer, y: f32| {
            let mut scene = Scene::new();
            scene.clear(Color::BLACK);
            scene.push_text(TextRun::new(
                TextId::new(1),
                Arc::from("View"),
                Rect::new(8.0, y, 96.0, 28.0),
                TextStyle::new(13.0, Color::WHITE).family(FontFamily::named("Inter")),
            ));
            scene.finish();
            renderer
                .render_to_snapshot(&scene, Size::new(112.0, 64.0), scale)
                .unwrap()
        };
        let settled = render(&mut renderer, 20.0);
        assert!(settled.rgba().chunks_exact(4).any(|pixel| pixel[0] > 0));
        // An eased translation approaches its endpoint from either direction. A tiny
        // remaining fraction must not leave downward-moving glyphs one pixel behind.
        for offset in [-0.1, 0.1] {
            let approaching = render(&mut renderer, 20.0 + offset / scale);
            assert!(
                approaching.rgba() == settled.rgba(),
                "text jumped at its endpoint: scale={scale}, physical offset={offset}"
            );
        }
        let moving = render(&mut renderer, 20.0 - 0.75 / scale);
        assert!(
            moving.rgba() != settled.rgba(),
            "whole-pixel movement must remain visible"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn ui_text_edges_use_srgb_blending_and_preserve_opacity() {
    let fonts = Rc::new(RefCell::new(fixture_font_system()));
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        fonts.clone(),
    ))
    .unwrap();
    for scale in [1.0, 2.0] {
        for (ink, backdrop) in [(0u8, 255u8), (32, 255), (255, 0)] {
            let style =
                TextStyle::new(20.0, Color::rgb8(ink, ink, ink)).family(FontFamily::named("Inter"));
            let bounds = Rect::new(8.0, 8.0, 48.0, 48.0);
            let (mask, left, top) = {
                let mut fonts = fonts.borrow_mut();
                let mut buffer = Buffer::new(&mut fonts, Metrics::new(20.0 * scale, 27.0 * scale));
                configure_text_buffer(
                    &mut buffer,
                    &mut fonts,
                    "M",
                    &style,
                    None,
                    Some(48.0),
                    scale,
                );
                let run = buffer.layout_runs().next().unwrap();
                let physical = run.glyphs[0].physical((8.0 * scale, 8.0 * scale), 1.0);
                let mask = SwashCache::new()
                    .get_image_uncached(
                        &mut fonts,
                        physical
                            .cache_key
                            .with_color(glyphon::Color::rgb(ink, ink, ink)),
                    )
                    .unwrap();
                let left = physical.x + mask.placement.left;
                let top = run.line_y.round() as i32 + physical.y - mask.placement.top;
                (mask, left, top)
            };
            for opacity in [1.0, 0.5] {
                let mut scene = Scene::new();
                scene.clear(Color::rgb8(backdrop, backdrop, backdrop));
                scene.multiply_opacity(opacity);
                scene.push_text(TextRun::new(
                    TextId::new(1),
                    Arc::from("M"),
                    bounds,
                    style.clone(),
                ));
                scene.finish();
                let snapshot = renderer
                    .render_to_snapshot(&scene, Size::new(64.0, 64.0), scale)
                    .unwrap();
                let mut edges = 0;
                for (index, alpha) in mask.data.iter().copied().enumerate() {
                    if !(25..230).contains(&alpha) {
                        continue;
                    }
                    let x = left + (index % mask.placement.width as usize) as i32;
                    let y = top + (index / mask.placement.width as usize) as i32;
                    let pixel = snapshot.pixel(x as u32, y as u32).unwrap();
                    let coverage = f32::from(alpha) / 255.0;
                    let backdrop = f32::from(backdrop) / 255.0;
                    let desired = backdrop + (f32::from(ink) / 255.0 - backdrop) * coverage;
                    let expected =
                        ((desired * opacity + backdrop * (1.0 - opacity)) * 255.0).round() as u8;
                    assert!(
                        pixel[0].abs_diff(expected) <= 3,
                        "scale={scale}, ink={ink}, opacity={opacity}, coverage={coverage}, pixel={pixel:?}, expected={expected}"
                    );
                    assert_eq!(pixel[0], pixel[1]);
                    assert_eq!(pixel[1], pixel[2]);
                    assert_eq!(pixel[3], 255);
                    edges += 1;
                }
                assert!(edges > 10, "the test must cover antialiased glyph edges");
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_default_ui_font_uses_the_system_family() {
    let mut font_system = create_font_system();
    for weight in [
        glyphon::Weight::NORMAL,
        glyphon::Weight::MEDIUM,
        glyphon::Weight::BOLD,
    ] {
        let style = TextStyle::new(14.0, Color::BLACK).weight(weight);
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(28.0, 38.0));
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            "Keep offline changes",
            &style,
            None,
            None,
            2.0,
        );
        let glyph = &buffer.layout_runs().next().unwrap().glyphs[0];
        let face = font_system.db().face(glyph.font_id).unwrap();
        assert_eq!(glyph.font_weight, weight);
        assert!(face.families.iter().any(|(name, _)| name == ".SF NS"));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_default_ui_text_matches_system_font_metrics_at_each_logical_size() {
    use core_foundation::{
        attributed_string::CFMutableAttributedString,
        base::{CFRange, CFType, CFTypeRef, TCFType},
        dictionary::CFDictionary,
        number::CFNumber,
        string::{CFString, CFStringRef},
    };

    #[link(name = "CoreText", kind = "framework")]
    unsafe extern "C" {
        static kCTFontAttributeName: CFStringRef;
        static kCTFontVariationAttribute: CFStringRef;
        fn CTFontCreateUIFontForLanguage(kind: u32, size: f64, language: CFStringRef) -> CFTypeRef;
        fn CTFontDescriptorCreateWithAttributes(attributes: CFTypeRef) -> CFTypeRef;
        fn CTFontCreateCopyWithAttributes(
            font: CFTypeRef,
            size: f64,
            matrix: *const (),
            descriptor: CFTypeRef,
        ) -> CFTypeRef;
        fn CTLineCreateWithAttributedString(text: CFTypeRef) -> CFTypeRef;
        fn CTLineGetTypographicBounds(
            line: CFTypeRef,
            ascent: *mut f64,
            descent: *mut f64,
            leading: *mut f64,
        ) -> f64;
    }

    let mut font_system = create_font_system();
    for size in [10.0, 12.0, 14.0, 18.0, 21.0, 24.0] {
        for weight in [400, 500, 700] {
            // Chromium resolves system-ui through this CoreText font and sets the CSS weight
            // directly on wght. Compare real platform metrics, not a duplicate of our shaper.
            let font = unsafe {
                let base = CFType::wrap_under_create_rule(CTFontCreateUIFontForLanguage(
                    2,
                    size,
                    std::ptr::null(),
                ));
                let variations = CFDictionary::from_CFType_pairs(&[(
                    CFNumber::from(0x77676874_i32),
                    CFNumber::from(weight),
                )]);
                let attrs = CFDictionary::from_CFType_pairs(&[(
                    CFString::wrap_under_get_rule(kCTFontVariationAttribute),
                    variations.as_CFType(),
                )]);
                let descriptor = CFType::wrap_under_create_rule(
                    CTFontDescriptorCreateWithAttributes(attrs.as_CFTypeRef()),
                );
                CFType::wrap_under_create_rule(CTFontCreateCopyWithAttributes(
                    base.as_CFTypeRef(),
                    size,
                    std::ptr::null(),
                    descriptor.as_CFTypeRef(),
                ))
            };
            for text in [
                "Open",
                "Completed",
                "Keep offline changes when reconnecting",
                "office AV 0123456789",
            ] {
                let mut reference = CFMutableAttributedString::new();
                reference.replace_str(&CFString::new(text), CFRange::init(0, 0));
                reference.set_attribute(
                    CFRange::init(0, reference.char_len()),
                    unsafe { kCTFontAttributeName },
                    &font,
                );
                let expected = unsafe {
                    let line = CFType::wrap_under_create_rule(CTLineCreateWithAttributedString(
                        reference.as_CFTypeRef(),
                    ));
                    CTLineGetTypographicBounds(
                        line.as_CFTypeRef(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    ) as f32
                };
                for scale in [1.0, 2.0] {
                    let style = TextStyle::new(size as f32, Color::BLACK)
                        .weight(glyphon::Weight(weight as u16));
                    let mut buffer = Buffer::new(
                        &mut font_system,
                        Metrics::new(style.font_size, style.line_height),
                    );
                    configure_text_buffer(
                        &mut buffer,
                        &mut font_system,
                        text,
                        &style,
                        None,
                        None,
                        scale,
                    );
                    let run = buffer.layout_runs().next().unwrap();
                    let actual = run.line_w / scale;
                    assert!(
                        (actual - expected).abs() < 0.3,
                        "{text:?}, size={size}, weight={weight}, scale={scale}: QuickGUI={actual}, CoreText={expected}"
                    );
                    let key = run.glyphs[0].physical((0.0, 0.0), 1.0).cache_key;
                    assert_eq!(key.optical_size_bits, (size as f32).to_bits());
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "run bun scripts/compare-text-rendering.ts to generate actual Electron references"]
fn capture_text_rendering_comparison() {
    #[derive(serde::Deserialize)]
    struct Case {
        text: String,
        size: f32,
        weight: u16,
        ink: String,
        background: String,
        opacity: f32,
    }
    let directory = std::path::PathBuf::from(
        std::env::var_os("QUICKGUI_TEXT_REFERENCE_DIR").expect("reference directory"),
    );
    let cases: Vec<Case> =
        serde_json::from_slice(&std::fs::read(directory.join("cases.json")).unwrap()).unwrap();
    let fonts = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer =
        pollster::block_on(OffscreenRenderer::new(PerformanceProfile::Balanced, fonts)).unwrap();
    let color = |hex: &str| {
        Color::rgb8(
            u8::from_str_radix(&hex[1..3], 16).unwrap(),
            u8::from_str_radix(&hex[3..5], 16).unwrap(),
            u8::from_str_radix(&hex[5..7], 16).unwrap(),
        )
    };
    // Diagnostic: CoreText/GPUI and Chromium deliberately differ in colored-mask treatment.
    // Production regressions above validate native masks and exact UI blending independently.
    let mut comparisons = Vec::new();
    for scale in [2.0] {
        let mut scene = Scene::new();
        for (index, case) in cases.iter().enumerate() {
            scene.push_quad(Quad::new(
                Rect::new(0.0, index as f32 * 64.0, 640.0, 64.0),
                color(&case.background),
            ));
            let style = TextStyle::new(case.size, color(&case.ink))
                .weight(glyphon::Weight(case.weight))
                .line_height(32.0)
                .wrap(TextWrap::None);
            let opacity = scene.multiply_opacity(case.opacity);
            scene.push_text(TextRun::new(
                TextId::new(index as u64 + 1),
                Arc::from(case.text.as_str()),
                Rect::new(16.0, index as f32 * 64.0 + 16.0, 608.0, 32.0),
                style,
            ));
            scene.restore_opacity(opacity);
        }
        scene.finish();
        let actual = renderer
            .render_to_snapshot(&scene, Size::new(640.0, 640.0), scale)
            .unwrap();
        actual
            .write_png(directory.join(format!("quickgui-{scale}.png")))
            .unwrap();
        let expected =
            crate::VisualSnapshot::open_png(directory.join(format!("electron-{scale}.png")))
                .unwrap();
        assert_eq!(
            (actual.width(), actual.height()),
            (expected.width(), expected.height())
        );
        for (index, case) in cases.iter().enumerate() {
            let background = color(&case.background).to_srgba8();
            let ink = color(&case.ink).to_srgba8();
            let vector: [f64; 3] =
                std::array::from_fn(|i| f64::from(ink[i]) - f64::from(background[i]));
            let magnitude = vector.iter().map(|v| v * v).sum::<f64>();
            let coverage = |image: &crate::VisualSnapshot| {
                let mut sum = 0.0;
                for y in
                    (index as f32 * 64.0 * scale) as u32..((index + 1) as f32 * 64.0 * scale) as u32
                {
                    for x in 0..image.width() {
                        let pixel = image.pixel(x, y).unwrap();
                        let value = (0..3)
                            .map(|i| (f64::from(pixel[i]) - f64::from(background[i])) * vector[i])
                            .sum::<f64>()
                            / magnitude;
                        sum += value.clamp(0.0, 1.0);
                    }
                }
                sum
            };
            let native = coverage(&actual);
            let web = coverage(&expected);
            let difference = (native / web - 1.0) * 100.0;
            eprintln!(
                "scale={scale} case={index} size={} weight={} ink={}: stroke coverage {difference:+.2}% (QuickGUI={native:.1}, Electron={web:.1})",
                case.size, case.weight, case.ink
            );
            comparisons.push(serde_json::json!({"scale":scale,"case":index,"size":case.size,"weight":case.weight,"ink":case.ink,"coverageDifferencePercent":difference}));
        }
    }
    std::fs::write(
        directory.join("comparison.json"),
        serde_json::to_vec_pretty(&comparisons).unwrap(),
    )
    .unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn macos_font_fallback_excludes_the_unscalable_gb18030_bitmap_face() {
    let font_system = create_font_system();
    assert!(
        font_system
            .db()
            .faces()
            .all(|face| face.post_script_name != "GB18030Bitmap")
    );
}

#[test]
fn multiline_monospaced_styled_text_keeps_rasterizable_glyphs_on_every_line() {
    let content: Arc<str> = Arc::from("first red\nsecond blue\nthird 東京 مرحبا 🙂");
    let red = content.find("red").unwrap();
    let blue = content.find("blue").unwrap();
    let styled = StyledText::new(content.clone()).with_highlights([
        (
            red..red + 3,
            HighlightStyle::default()
                .color(Color::rgb8(248, 113, 113))
                .background(Color::rgba8(127, 29, 29, 96)),
        ),
        (
            blue..blue + 4,
            HighlightStyle::default()
                .color(Color::rgb8(96, 165, 250))
                .underline(),
        ),
    ]);
    let style = TextStyle::new(14.0, Color::WHITE)
        .family(FontFamily::Monospace)
        .line_height(22.0);
    let mut font_system = create_font_system();
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.font_size, style.line_height),
    );
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &style,
        Some(styled.highlights()),
        Some(400.0),
        1.0,
    );

    let runs: Vec<_> = buffer.layout_runs().collect();
    assert_eq!(runs.len(), 3);
    assert_eq!(
        runs.iter().map(|run| run.text).collect::<Vec<_>>(),
        ["first red", "second blue", "third 東京 مرحبا 🙂"]
    );
    assert!(runs.iter().all(|run| run.line_w.is_finite()));
    assert!(runs.iter().all(|run| !run.glyphs.is_empty()));

    let mut swash = SwashCache::new();
    for run in runs {
        for glyph in run.glyphs.iter().filter(|glyph| glyph.w > 0.0) {
            assert!(glyph.x.is_finite() && glyph.w.is_finite());
            assert!(
                swash
                    .get_image_uncached(
                        &mut font_system,
                        glyph.physical((0.0, 0.0), 1.0).cache_key,
                    )
                    .is_some(),
                "every fallback glyph in a monospaced rich-text buffer must rasterize"
            );
        }
    }
}

#[test]
fn basic_text_fragments_isolate_changing_numeric_fields() {
    let fragments: Vec<_> = BasicTextFragments::new("Row 014440    The quick brown fox").collect();
    assert_eq!(
        fragments,
        ["Row ", "01", "44", "40", "    The quick brown fox"]
    );
}

#[test]
fn basic_text_fragmentation_requires_explicit_safe_shaping() {
    let content = "Row 014440    The quick brown fox";
    let mut style = TextStyle::new(14.0, Color::WHITE).wrap(TextWrap::None);
    assert!(!should_fragment_basic_text(content, &style));
    style.shaping = TextShaping::Basic;
    assert!(should_fragment_basic_text(content, &style));
    assert!(!should_fragment_basic_text(
        content,
        &style.clone().align(TextAlign::Center)
    ));
    assert!(!should_fragment_basic_text("short 123", &style));
    assert!(!should_fragment_basic_text(
        "Row 014440\tThe quick brown fox",
        &style
    ));
}

#[test]
fn dilation_expands_and_collapses_around_the_original_center() {
    let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
    assert_eq!(dilate_rect(rect, 3.0), Rect::new(7.0, 17.0, 106.0, 56.0));
    assert_eq!(dilate_rect(rect, -60.0), Rect::new(60.0, 45.0, 0.0, 0.0));
}

#[test]
fn one_wavy_underline_span_is_one_analytic_shape_instance() {
    let underline = WavyUnderline::new(
        Rect::new(10.0, 20.0, 120.0, 6.0),
        23.0,
        1.5,
        2.0,
        6.0,
        Color::rgb8(248, 113, 113),
    );
    let instance = wavy_underline_instance(&underline, Rect::new(0.0, 0.0, 200.0, 100.0));
    assert_eq!(instance.geometry, [10.0, 20.0, 120.0, 6.0]);
    assert_eq!(instance.subject, [1.5, 0.0, 0.0, 0.0]);
    assert_eq!(instance.params, [SHAPE_MODE_WAVY_UNDERLINE, 23.0, 2.0, 6.0]);
}

#[test]
fn drop_shadow_geometry_includes_offset_spread_and_three_sigma_margin() {
    let shadow = Shadow::new(
        Rect::new(10.0, 20.0, 100.0, 50.0),
        BoxShadow::new(4.0, 6.0, Color::WHITE)
            .blur_radius(10.0)
            .spread_radius(3.0),
    )
    .radius(8.0);
    let instance = shadow_instance(&shadow, Rect::new(-100.0, -100.0, 500.0, 500.0)).unwrap();
    assert_eq!(instance.subject, [11.0, 23.0, 106.0, 56.0]);
    assert_eq!(instance.geometry, [-5.0, 7.0, 138.0, 88.0]);
    assert_eq!(instance.params, [SHAPE_MODE_DROP_SHADOW, 0.0, 0.0, 10.0]);
    assert_eq!(instance.corners, [8.0, 8.0, 8.0, 8.0]);
}

#[test]
fn inset_shadow_uses_a_spread_contracting_translated_hole() {
    let shadow = Shadow::new(
        Rect::new(10.0, 20.0, 100.0, 50.0),
        BoxShadow::new(2.0, 3.0, Color::WHITE)
            .blur_radius(4.0)
            .spread_radius(5.0)
            .inset(true),
    )
    .radius(8.0);
    let instance = shadow_instance(&shadow, Rect::new(0.0, 0.0, 500.0, 500.0)).unwrap();
    assert_eq!(instance.geometry, [10.0, 20.0, 100.0, 50.0]);
    assert_eq!(instance.subject, [17.0, 28.0, 90.0, 40.0]);
    // The hole's corner radii are derived in the shader from the element radii and the spread.
    assert_eq!(instance.params, [SHAPE_MODE_INSET_SHADOW, 5.0, 0.0, 4.0]);
    assert_eq!(instance.corners, [8.0, 8.0, 8.0, 8.0]);
}

fn shaped_line_width(font_system: &mut FontSystem, content: &str, style: &TextStyle) -> f32 {
    let mut buffer = Buffer::new(
        font_system,
        Metrics::new(style.font_size, style.line_height),
    );
    configure_text_buffer(&mut buffer, font_system, content, style, None, None, 1.0);
    buffer
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0_f32, f32::max)
}

#[test]
fn letter_and_word_spacing_add_exact_logical_advance() {
    let mut font_system = fixture_font_system();
    let content = "ab cd ef";
    let base = TextStyle::new(20.0, Color::WHITE)
        .family(FontFamily::named("Inter"))
        .wrap(TextWrap::None);
    let plain = shaped_line_width(&mut font_system, content, &base);

    // Eight clusters each gain 2 logical pixels of tracking.
    let tracked = shaped_line_width(&mut font_system, content, &base.clone().letter_spacing(2.0));
    assert!(
        (tracked - (plain + 16.0)).abs() < 0.5,
        "tracked {tracked} should be 16 wider than {plain}"
    );

    // Only the two spaces gain word spacing.
    let spaced = shaped_line_width(&mut font_system, content, &base.clone().word_spacing(5.0));
    assert!(
        (spaced - (plain + 10.0)).abs() < 0.5,
        "word spaced {spaced} should be 10 wider than {plain}"
    );

    // Both spacings are part of the retained shaping key.
    let extras = TextShapingExtras::from_style(&base.clone().letter_spacing(2.0));
    assert_ne!(extras, TextShapingExtras::from_style(&base));
    assert_ne!(
        extras,
        TextShapingExtras::from_style(&base.clone().word_spacing(2.0))
    );
}

#[test]
fn a_forced_base_direction_orders_a_neutral_run_against_the_content() {
    // Digits and punctuation only: the Unicode bidirectional algorithm has no strong character to
    // derive a paragraph direction from, so the declared base direction decides it.
    const NEUTRAL: &str = "12 - 34";
    // A digit run between Hebrew words: its visual placement depends on the paragraph level.
    const MIXED: &str = "\u{05d0}\u{05d1} 12 \u{05d2}\u{05d3}";

    fn shaped(
        font_system: &mut FontSystem,
        content: &str,
        style: &TextStyle,
    ) -> (bool, Vec<usize>) {
        let mut buffer = Buffer::new(
            font_system,
            Metrics::new(style.font_size, style.line_height),
        );
        configure_text_buffer(&mut buffer, font_system, content, style, None, None, 1.0);
        let run = buffer
            .layout_runs()
            .next()
            .expect("the run shapes at least one line");
        (
            run.rtl,
            run.glyphs.iter().map(|glyph| glyph.start).collect(),
        )
    }

    let mut font_system = fixture_font_system();
    let base = TextStyle::new(16.0, Color::WHITE)
        .family(FontFamily::named("Noto Sans Hebrew"))
        .wrap(TextWrap::None);

    // Neutral content follows whichever base direction was declared.
    assert!(!shaped(&mut font_system, NEUTRAL, &base).0);
    assert!(
        shaped(
            &mut font_system,
            NEUTRAL,
            &base.clone().direction(TextDirection::Rtl)
        )
        .0
    );
    assert!(
        !shaped(
            &mut font_system,
            NEUTRAL,
            &base.clone().direction(TextDirection::Ltr)
        )
        .0
    );

    // Forcing a direction on mixed content reorders its visual runs.
    let (rtl_flag, rtl_order) = shaped(
        &mut font_system,
        MIXED,
        &base.clone().direction(TextDirection::Rtl),
    );
    let (ltr_flag, ltr_order) = shaped(
        &mut font_system,
        MIXED,
        &base.clone().direction(TextDirection::Ltr),
    );
    assert!(rtl_flag);
    assert!(!ltr_flag);
    assert_ne!(rtl_order, ltr_order);

    assert_ne!(
        TextShapingExtras::from_style(&base.clone().direction(TextDirection::Rtl)),
        TextShapingExtras::from_style(&base)
    );
}

#[test]
fn text_transform_shapes_the_mapped_string_and_keeps_source_indices() {
    let content: Arc<str> = Arc::from("straße road");
    let style = TextStyle::new(16.0, Color::WHITE)
        .family(FontFamily::named("Inter"))
        .wrap(TextWrap::None)
        .text_transform(TextTransform::Uppercase);
    let projection = project_text_content(&content, None, &style);

    assert_eq!(projection.content.as_ref(), "STRASSE ROAD");
    // The one character whose case mapping changes length snaps to a real source boundary; every
    // other display index maps back exactly.
    assert_eq!(projection.display_to_original(0), 0);
    assert_eq!(projection.display_to_original(3), 3);
    assert_eq!(
        projection.display_to_original("STRASSE".len()),
        "straße".len()
    );
    assert_eq!(
        projection.display_to_original(projection.content.len()),
        content.len()
    );
    assert_eq!(
        projection.original_to_display(content.len()),
        projection.content.len()
    );
    assert_eq!(projection.display_ranges_for_original(0..3), vec![0..3]);

    // Capitalization only touches the first character of each word.
    let capitalized = project_text_content(
        &content,
        None,
        &style.clone().text_transform(TextTransform::Capitalize),
    );
    assert_eq!(capitalized.content.as_ref(), "Straße Road");
    assert_eq!(capitalized.display_to_original(7), 7);
}

#[test]
fn soft_hyphens_are_removed_unless_manual_hyphenation_is_requested() {
    let content: Arc<str> = Arc::from("Kraft\u{00ad}fahrzeug");
    let base = TextStyle::new(16.0, Color::WHITE)
        .family(FontFamily::named("Inter"))
        .wrap(TextWrap::Word);

    let stripped = project_text_content(&content, None, &base);
    assert_eq!(stripped.content.as_ref(), "Kraftfahrzeug");
    // Indices after the removed character still land on source boundaries.
    assert_eq!(stripped.display_to_original("Kraft".len()), "Kraft".len());
    assert_eq!(
        stripped.display_to_original(stripped.content.len()),
        content.len()
    );

    let manual = project_text_content(&content, None, &base.clone().hyphens(Hyphens::Manual));
    assert_eq!(manual.content.as_ref(), content.as_ref());
    assert_eq!(manual.display_to_original(9), 9);

    // Content without a soft hyphen keeps the identity projection and every truncation path.
    let plain: Arc<str> = Arc::from("Kraftfahrzeug");
    assert!(!rewrites_text_content(&plain, &base));
    assert!(rewrites_text_content(&content, &base));
}

#[test]
fn word_break_and_overflow_wrap_map_onto_cosmic_wrap_modes() {
    let base = TextStyle::new(16.0, Color::WHITE);
    assert_eq!(cosmic_wrap(&base), Wrap::Word);
    assert_eq!(cosmic_wrap(&base.clone().wrap(TextWrap::None)), Wrap::None);
    assert_eq!(
        cosmic_wrap(&base.clone().word_break(WordBreak::BreakAll)),
        Wrap::Glyph
    );
    assert_eq!(
        cosmic_wrap(&base.clone().word_break(WordBreak::KeepAll)),
        Wrap::Word
    );
    assert_eq!(
        cosmic_wrap(&base.clone().overflow_wrap(OverflowWrap::Anywhere)),
        Wrap::Glyph
    );
    assert_eq!(
        cosmic_wrap(&base.clone().overflow_wrap(OverflowWrap::BreakWord)),
        Wrap::WordOrGlyph
    );
    // `word-break` wins over `overflow-wrap`, and `wrap(None)` wins over both.
    assert_eq!(
        cosmic_wrap(
            &base
                .clone()
                .word_break(WordBreak::KeepAll)
                .overflow_wrap(OverflowWrap::Anywhere)
        ),
        Wrap::Word
    );
    assert_eq!(
        cosmic_wrap(
            &base
                .clone()
                .wrap(TextWrap::None)
                .overflow_wrap(OverflowWrap::Anywhere)
        ),
        Wrap::None
    );

    // A long word only breaks once the mode allows it.
    let mut font_system = fixture_font_system();
    let content = "Kraftfahrzeughaftpflichtversicherung";
    let narrow = TextStyle::new(14.0, Color::WHITE).family(FontFamily::named("Inter"));
    let mut normal = Buffer::new(&mut font_system, Metrics::new(14.0, 18.0));
    configure_text_buffer(
        &mut normal,
        &mut font_system,
        content,
        &narrow,
        None,
        Some(60.0),
        1.0,
    );
    let mut broken = Buffer::new(&mut font_system, Metrics::new(14.0, 18.0));
    configure_text_buffer(
        &mut broken,
        &mut font_system,
        content,
        &narrow.clone().word_break(WordBreak::BreakAll),
        None,
        Some(60.0),
        1.0,
    );
    assert_eq!(normal.layout_runs().count(), 1);
    assert!(broken.layout_runs().count() > 1);
}

#[test]
fn overline_geometry_sits_above_the_baseline_and_joins_the_shaping_key() {
    let mut font_system = fixture_font_system();
    let style = TextStyle::new(20.0, Color::WHITE)
        .family(FontFamily::named("Inter"))
        .wrap(TextWrap::None)
        .overline_color(Color::rgb8(255, 0, 0));
    assert!(style.has_decorations());

    let mut buffer = Buffer::new(&mut font_system, Metrics::new(20.0, 26.0));
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        "Overlined",
        &style,
        None,
        None,
        1.0,
    );
    let geometry = collect_styled_text_geometry(&buffer, &[], &style, 1.0, 0.0..26.0, None);
    let decoration = geometry
        .decorations
        .first()
        .copied()
        .expect("an overline is emitted");
    assert_eq!(decoration.color, Color::rgb8(255, 0, 0));
    let baseline = buffer
        .layout_runs()
        .next()
        .map(|run| run.line_y)
        .expect("one shaped line");
    assert!(
        decoration.rect.y < baseline,
        "the overline at {} must sit above the baseline at {baseline}",
        decoration.rect.y
    );

    assert_ne!(
        TextShapingExtras::from_style(&style),
        TextShapingExtras::from_style(&TextStyle::new(20.0, Color::WHITE))
    );
}

#[test]
fn a_text_shadow_paints_bounded_offset_copies_beneath_the_run() {
    let content: Arc<str> = Arc::from("shadowed");
    let bounds = Rect::new(10.0, 20.0, 200.0, 30.0);
    let sharp = TextStyle::new(16.0, Color::WHITE).text_shadow(TextShadow::new(
        3.0,
        4.0,
        0.0,
        Color::rgba8(0, 0, 0, 255),
    ));

    let mut scene = Scene::new();
    scene.push_text(TextRun::new(
        TextId::new(1),
        content.clone(),
        bounds,
        sharp.clone(),
    ));
    let runs = scene.text_runs();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].bounds, Rect::new(13.0, 24.0, 200.0, 30.0));
    assert_eq!(runs[0].style.color, Color::rgba8(0, 0, 0, 255));
    assert!(runs[0].style.shadow.is_none());
    assert_eq!(runs[1].bounds, bounds);
    assert_eq!(runs[1].style.color, Color::WHITE);

    // A blur radius is approximated with a bounded number of extra copies.
    let blurred = TextStyle::new(16.0, Color::WHITE).text_shadow(TextShadow::new(
        3.0,
        4.0,
        8.0,
        Color::rgba8(0, 0, 0, 255),
    ));
    let mut scene = Scene::new();
    scene.push_text(TextRun::new(TextId::new(1), content, bounds, blurred));
    assert_eq!(scene.text_runs().len(), MAX_TEXT_SHADOW_SAMPLES + 1);

    // Shadows are purely visual and never split the retained shaping cache.
    assert_eq!(
        TextShapingExtras::from_style(&sharp),
        TextShapingExtras::from_style(&TextStyle::new(16.0, Color::WHITE))
    );

    // Offsets and blur are clamped to their exported bounds.
    let clamped = TextShadow::new(f32::INFINITY, -1e9, f32::NAN, Color::BLACK);
    assert_eq!(clamped.offset_x, 0.0);
    assert_eq!(clamped.offset_y, -TextShadow::MAX_OFFSET);
    assert_eq!(clamped.blur, 0.0);
}

#[cfg(target_os = "macos")]
#[test]
fn a_scene_without_layer_effects_never_touches_the_compositor() {
    use crate::scene::PaintLayerKey;

    let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        font_system,
    ))
    .unwrap();
    let mut scene = Scene::new();
    scene.clear(Color::BLACK);
    scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 16.0, 16.0), Color::WHITE));
    scene.finish();
    renderer
        .render_to_snapshot(&scene, Size::new(32.0, 32.0), 1.0)
        .unwrap();
    assert!(
        renderer.compositor().is_idle(),
        "a scene with no layer effects compiled a pipeline or allocated a texture"
    );
    let idle = renderer.last_composite();
    assert_eq!(idle.layers, 0);
    assert_eq!(idle.layer_passes, 0);
    assert_eq!(idle.blur_passes, 0);
    assert_eq!(idle.layer_texture_bytes, 0);
    assert_eq!(idle.skipped_layer_effects, 0);

    // One group is enough to allocate, and the textures are retained for the next frame.
    let mut grouped = Scene::new();
    grouped.clear(Color::BLACK);
    let handle = grouped
        .begin_group(
            PaintLayerKey::default(),
            Rect::new(0.0, 0.0, 16.0, 16.0),
            Rect::new(0.0, 0.0, 32.0, 32.0),
            crate::LayerEffects {
                transform: crate::Transform2D::rotate_degrees(30.0),
                ..Default::default()
            },
        )
        .expect("the first group fits every bound");
    grouped.push_quad_in(
        handle.content_key(),
        Quad::new(Rect::new(0.0, 0.0, 16.0, 16.0), Color::WHITE),
    );
    grouped.end_group(handle);
    grouped.finish();
    renderer
        .render_to_snapshot(&grouped, Size::new(32.0, 32.0), 1.0)
        .unwrap();
    let first = renderer.last_composite();
    assert_eq!(first.layers, 1);
    assert_eq!(first.layer_passes, 1);
    assert_eq!(first.blur_passes, 0);
    // One 32x32 group texture plus the shared destination capture.
    let retained = renderer.compositor().retained_bytes();
    assert!(retained > 0 && retained <= crate::MAX_LAYER_TEXTURE_BYTES);
    assert_eq!(first.layer_texture_bytes, retained);

    renderer
        .render_to_snapshot(&grouped, Size::new(32.0, 32.0), 1.0)
        .unwrap();
    assert_eq!(
        renderer.compositor().retained_bytes(),
        retained,
        "a settled window must reuse its retained group textures instead of allocating again"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn a_blurred_group_records_two_separable_passes_and_stays_inside_its_budget() {
    use crate::scene::PaintLayerKey;

    let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        font_system,
    ))
    .unwrap();
    let mut scene = Scene::new();
    scene.clear(Color::BLACK);
    let handle = scene
        .begin_group(
            PaintLayerKey::default(),
            Rect::new(4.0, 4.0, 16.0, 16.0),
            Rect::new(0.0, 0.0, 32.0, 32.0),
            crate::LayerEffects {
                blur: 4.0,
                ..Default::default()
            },
        )
        .expect("the first group fits every bound");
    scene.push_quad_in(
        handle.content_key(),
        Quad::new(Rect::new(4.0, 4.0, 16.0, 16.0), Color::WHITE),
    );
    scene.end_group(handle);
    scene.finish();
    let snapshot = renderer
        .render_to_snapshot(&scene, Size::new(32.0, 32.0), 1.0)
        .unwrap();
    let stats = renderer.last_composite();
    assert_eq!(stats.layers, 1);
    assert_eq!(stats.layer_passes, 1);
    assert_eq!(stats.blur_passes, 2, "a separable Gaussian is two passes");
    assert!(stats.layer_texture_bytes <= crate::MAX_LAYER_TEXTURE_BYTES);
    // The blur really spread past the quad's own edge and falls off outwards.
    let inside = snapshot.pixel(12, 12).unwrap()[0];
    let just_outside = snapshot.pixel(2, 12).unwrap()[0];
    let far_outside = snapshot.pixel(30, 12).unwrap()[0];
    assert!(just_outside > 0, "the blur did not spread past the edge");
    assert!(
        far_outside < just_outside && just_outside < inside,
        "the blur must fall off outwards: {inside} {just_outside} {far_outside}"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn retained_layer_pixels_survive_transform_opacity_and_blur_changes() {
    use crate::scene::PaintLayerKey;
    let fonts = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        fonts.clone(),
    ))
    .unwrap();
    let viewport = Size::new(80.0, 80.0);
    for (index, (angle, opacity, blur, color, expected_passes, scale)) in [
        (10.0, 1.0, 2.0, Color::WHITE, 1, 1.0),
        (25.0, 0.5, 2.0, Color::WHITE, 0, 1.0),
        (25.0, 0.5, 4.0, Color::WHITE, 0, 1.0),
        (25.0, 0.5, 4.0, Color::rgb8(220, 30, 20), 1, 1.0),
        (25.0, 0.5, 4.0, Color::rgb8(220, 30, 20), 1, 2.0),
    ]
    .into_iter()
    .enumerate()
    {
        let mut scene = Scene::new();
        scene.clear(Color::BLACK);
        scene.multiply_opacity(opacity);
        let group = scene
            .begin_group(
                PaintLayerKey::default(),
                Rect::new(20.0, 20.0, 20.0, 20.0),
                Rect::from_size(viewport),
                crate::LayerEffects {
                    transform: crate::Transform2D::rotate_degrees(angle)
                        .around(Point::new(30.0, 30.0)),
                    blur,
                    ..Default::default()
                },
            )
            .unwrap();
        scene.push_quad_in(
            group.content_key(),
            Quad::new(Rect::new(20.0, 20.0, 20.0, 20.0), color),
        );
        scene.end_group(group);
        scene.finish();
        let actual = renderer
            .render_to_snapshot(&scene, viewport, scale)
            .unwrap();
        let stats = renderer.last_composite();
        assert_eq!(stats.layer_passes, expected_passes, "case {index}");
        if index == 1 {
            assert_eq!(stats.reused_layers, 1);
            assert_eq!(stats.blur_passes, 0);
        }
        if index == 2 {
            assert_eq!(stats.blur_passes, 2);
        }
        let mut fresh = pollster::block_on(OffscreenRenderer::new(
            PerformanceProfile::Balanced,
            fonts.clone(),
        ))
        .unwrap();
        let expected = fresh.render_to_snapshot(&scene, viewport, scale).unwrap();
        assert_eq!(
            actual.rgba(),
            expected.rgba(),
            "cached compositing changed pixels in case {index}"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn nested_layer_transform_invalidates_parent_pixels_only() {
    use crate::scene::PaintLayerKey;
    let fonts = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        fonts.clone(),
    ))
    .unwrap();
    for angle in [10.0, 25.0] {
        let mut scene = Scene::new();
        let bounds = Rect::new(10.0, 10.0, 40.0, 40.0);
        let viewport = Size::new(64.0, 64.0);
        let outer = scene
            .begin_group(
                PaintLayerKey::default(),
                bounds,
                Rect::from_size(viewport),
                crate::LayerEffects {
                    blur: 1.0,
                    ..Default::default()
                },
            )
            .unwrap();
        let inner = scene
            .begin_group(
                outer.content_key(),
                bounds,
                Rect::from_size(viewport),
                crate::LayerEffects {
                    transform: crate::Transform2D::rotate_degrees(angle)
                        .around(Point::new(30.0, 30.0)),
                    ..Default::default()
                },
            )
            .unwrap();
        scene.push_quad_in(inner.content_key(), Quad::new(bounds, Color::WHITE));
        scene.end_group(inner);
        scene.end_group(outer);
        scene.finish();
        let actual = renderer.render_to_snapshot(&scene, viewport, 1.0).unwrap();
        if angle == 25.0 {
            assert_eq!(renderer.last_composite().layer_passes, 1);
            assert_eq!(renderer.last_composite().reused_layers, 1);
        }
        let mut fresh = pollster::block_on(OffscreenRenderer::new(
            PerformanceProfile::Balanced,
            fonts.clone(),
        ))
        .unwrap();
        let expected = fresh.render_to_snapshot(&scene, viewport, 1.0).unwrap();
        assert_eq!(actual.rgba(), expected.rgba());
    }
}

#[cfg(target_os = "macos")]
#[test]
fn retained_glyph_uploads_match_fresh_text_through_edits_and_buffer_growth() {
    let fonts = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        fonts.clone(),
    ))
    .unwrap();
    let viewport = Size::new(200.0, 80.0);
    for content in [
        "Count 100",
        "Count 101",
        "Count 101",
        "",
        "Count 2",
        "A longer label with several more glyphs",
    ] {
        let mut scene = Scene::new();
        scene.clear(Color::BLACK);
        scene.push_text(TextRun::new(
            TextId::new(90),
            content.into(),
            Rect::new(5.0, 5.0, 190.0, 70.0),
            TextStyle::new(16.0, Color::WHITE),
        ));
        scene.finish();
        let actual = renderer.render_to_snapshot(&scene, viewport, 1.0).unwrap();
        let mut fresh = pollster::block_on(OffscreenRenderer::new(
            PerformanceProfile::Balanced,
            fonts.clone(),
        ))
        .unwrap();
        let expected = fresh.render_to_snapshot(&scene, viewport, 1.0).unwrap();
        assert_eq!(
            actual.rgba(),
            expected.rgba(),
            "glyph uploads differ for {content:?}"
        );
    }
}

#[test]
fn transparent_present_re_premultiplies_into_the_compositor_encoding() {
    let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        font_system,
    ))
    .unwrap();
    let (device, queue) = renderer.gpu();
    let format = TextureFormat::Rgba8UnormSrgb;
    let mut present = super::present::TransparentPresent::new(device, format);
    let (intermediate, _) = present.target(device, format, 0, 3, 1);

    // Pixel 0 is what linear blending leaves for mid grey at 50% coverage: the premultiplied
    // linear value `0.5 * decode(0.5)`, encoded, which is 92 out of 255. Pixel 1 is opaque mid
    // grey, and pixel 2 is clear.
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &intermediate,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[92, 92, 92, 128, 128, 128, 128, 255, 0, 0, 0, 0],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(12),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 3,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("present test output"),
        size: wgpu::Extent3d {
            width: 3,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let output_view = output.create_view(&TextureViewDescriptor::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("present test readback"),
        size: u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("present test encoder"),
    });
    present.present(device, &mut encoder, 0, &output_view);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 3,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    let slice = readback.slice(..);
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    receiver.recv().unwrap().unwrap();
    let mapped = slice.get_mapped_range().unwrap();
    let pixels = mapped[..12].to_vec();
    drop(mapped);

    // A gamma-space compositor expects `coverage * encode(grey)` = 0.5 * 0.5 → 64 of 255, with the
    // coverage itself untouched. Opaque and clear pixels pass through unchanged.
    assert!(
        (pixels[0] as i32 - 64).abs() <= 2,
        "premultiplied red {}",
        pixels[0]
    );
    assert!(
        (pixels[1] as i32 - 64).abs() <= 2,
        "premultiplied green {}",
        pixels[1]
    );
    assert!(
        (pixels[2] as i32 - 64).abs() <= 2,
        "premultiplied blue {}",
        pixels[2]
    );
    assert_eq!(pixels[3], 128);
    assert_eq!(&pixels[4..8], &[128, 128, 128, 255]);
    assert_eq!(&pixels[8..12], &[0, 0, 0, 0]);
}

#[test]
fn ui_target_premultiplies_transparent_backgrounds_in_srgb() {
    let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer = pollster::block_on(OffscreenRenderer::new(
        PerformanceProfile::Balanced,
        font_system,
    ))
    .unwrap();
    let color = Color::rgba8(128, 192, 240, 128);
    let mut cleared = Scene::new();
    cleared.clear(color);
    cleared.finish();
    let mut painted = Scene::new();
    painted.clear(Color::TRANSPARENT);
    painted.push_quad(Quad::new(Rect::new(0.0, 0.0, 8.0, 8.0), color));
    painted.finish();
    let clear = renderer
        .render_to_snapshot(&cleared, Size::new(8.0, 8.0), 1.0)
        .unwrap();
    let paint = renderer
        .render_to_snapshot(&painted, Size::new(8.0, 8.0), 1.0)
        .unwrap();
    for pixel in [clear.pixel(4, 4).unwrap(), paint.pixel(4, 4).unwrap()] {
        for (actual, expected) in pixel.into_iter().zip([64_u8, 96, 120, 128]) {
            assert!(actual.abs_diff(expected) <= 1, "{pixel:?}");
        }
    }
}

#[test]
fn rounded_range_washes_merge_across_syntax_colors_without_affecting_glyphs() {
    let content: Arc<str> = Arc::from("const value");
    let wash = Color::rgba8(120, 180, 240, 60);
    let styled = StyledText::new(content.clone()).with_highlights([
        (
            0..6,
            HighlightStyle::default()
                .color(Color::WHITE)
                .background(wash)
                .background_shape(3.0, 2.0, 1.5),
        ),
        (
            6..11,
            HighlightStyle::default()
                .color(Color::BLACK)
                .background(wash)
                .background_shape(3.0, 2.0, 1.5),
        ),
    ]);
    let style = TextStyle::new(14.0, Color::WHITE).line_height(22.0);
    let mut font_system = create_font_system();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 22.0));
    let highlights = Arc::from(styled.highlights());
    configure_text_buffer(
        &mut buffer,
        &mut font_system,
        &content,
        &style,
        Some(&highlights),
        Some(300.0),
        1.0,
    );
    let geometry =
        collect_styled_text_geometry(&buffer, &highlights, &style, 1.0, 0.0..100.0, None);
    assert_eq!(geometry.backgrounds.len(), 1);
    let background = geometry.backgrounds[0];
    assert_eq!(background.kind, TextPaintKind::Rounded(3.0));
    assert_eq!(background.rect.x, -2.0);
    assert_eq!(background.rect.y, 1.5);
    assert_eq!(background.rect.height, 19.0);
}

#[cfg(target_os = "macos")]
#[test]
fn native_intrinsic_labels_round_once_in_logical_pixels() {
    let fonts = create_shared_font_system(&Assets::default(), &[]).unwrap();
    let mut renderer =
        pollster::block_on(OffscreenRenderer::new(PerformanceProfile::Balanced, fonts)).unwrap();
    let content: Arc<str> = Arc::from("Parity fixture");
    let style = TextStyle::new(12.0, Color::WHITE)
        .family(FontFamily::named("SF Mono"))
        .line_height(18.0);
    let measured = renderer.measure_text(TextId::new(900), &content, &style, None, 2.0);
    assert_eq!(measured.width, 104.0);
    let constrained = renderer.measure_text(
        TextId::new(900),
        &content,
        &style,
        Some(measured.width),
        2.0,
    );
    assert_eq!(constrained.height, 18.0);
}

#[cfg(target_os = "macos")]
#[test]
fn macos_braille_uses_the_native_script_fallback() {
    let mut fonts = create_font_system();
    let style = TextStyle::new(12., Color::WHITE).family(FontFamily::from("SF Mono"));
    let mut buffer = Buffer::new(&mut fonts, Metrics::new(24., 26.));
    configure_text_buffer(&mut buffer, &mut fonts, "⣸⣿⡿", &style, None, None, 2.);
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let face = fonts.db().face(glyph.font_id).unwrap();
            assert_eq!(face.post_script_name, "AppleBraille");
        }
    }
}

#[test]
fn aligned_wrapped_text_can_preserve_break_whitespace() {
    let mut fonts = create_font_system();
    let mut buffer = Buffer::new(&mut fonts, Metrics::new(24., 36.));
    let mut style = TextStyle::new(12., Color::WHITE).family(FontFamily::Monospace);
    style.align = TextAlign::Center;
    configure_text_buffer(
        &mut buffer,
        &mut fonts,
        "hello world again",
        &style,
        None,
        Some(90.),
        2.,
    );
    let trimmed = buffer.layout_runs().next().unwrap().glyphs[0].x;
    style.align = TextAlign::CenterIncludingWhitespace;
    configure_text_buffer(
        &mut buffer,
        &mut fonts,
        "hello world again",
        &style,
        None,
        Some(90.),
        2.,
    );
    let preserved = buffer.layout_runs().next().unwrap().glyphs[0].x;
    assert!(
        preserved < trimmed,
        "the trailing space participates in centering"
    );
}

#[test]
fn wrapped_measurement_reuse_is_invalidated_by_font_metrics() {
    let fonts = Rc::new(RefCell::new(fixture_font_system()));
    let mut renderer =
        pollster::block_on(OffscreenRenderer::new(PerformanceProfile::Balanced, fonts)).unwrap();
    let content: Arc<str> =
        Arc::from("A paragraph whose intrinsic width changes with its font size.");
    let small = TextStyle::new(12., Color::WHITE).family(FontFamily::named("Inter"));
    renderer.measure_text(TextId::new(901), &content, &small, Some(100.), 2.);
    let large = TextStyle::new(24., Color::WHITE).family(FontFamily::named("Inter"));
    let reused = renderer.measure_text(TextId::new(901), &content, &large, None, 2.);
    let fresh = renderer.measure_text(TextId::new(902), &content, &large, None, 2.);
    assert_eq!(reused, fresh);
    assert!(reused.width > 100.);
}

#[test]
fn equal_underlines_do_not_bridge_undecorated_text() {
    let content: Arc<str> = Arc::from("link plain link");
    let styled = StyledText::new(content.clone()).with_highlights([
        (0..4, HighlightStyle::default().underline()),
        (11..15, HighlightStyle::default().underline()),
    ]);
    let style = TextStyle::new(14.0, Color::WHITE).line_height(22.0);
    let mut fonts = fixture_font_system();
    let mut buffer = Buffer::new(&mut fonts, Metrics::new(14.0, 22.0));
    let highlights = Arc::from(styled.highlights());
    configure_text_buffer(
        &mut buffer,
        &mut fonts,
        &content,
        &style,
        Some(&highlights),
        Some(300.0),
        1.0,
    );
    let geometry =
        collect_styled_text_geometry(&buffer, &highlights, &style, 1.0, 0.0..100.0, None);
    assert_eq!(geometry.decorations.len(), 2);
    assert!(geometry.decorations[0].rect.right() < geometry.decorations[1].rect.x);
}
