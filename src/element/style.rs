use super::*;

/// Clamp a transform origin to a finite fraction of the element box.
fn sane_transform_origin(origin: Point) -> Point {
    let clamp = |value: f32| {
        if value.is_finite() {
            value.clamp(-16.0, 16.0)
        } else {
            0.5
        }
    };
    Point::new(clamp(origin.x), clamp(origin.y))
}

impl Element {
    pub fn padding_axis(mut self, vertical: f32, horizontal: f32) -> Self {
        self.layout.padding = TaffyRect {
            left: LengthPercentage::length(horizontal),
            right: LengthPercentage::length(horizontal),
            top: LengthPercentage::length(vertical),
            bottom: LengthPercentage::length(vertical),
        };
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.visual.background = Some(color);
        self.visual.background_gradient = None;
        self
    }

    /// Paint a bounded multi-stop gradient behind this element's children.
    ///
    /// Accepts any [`Background`], so a solid [`Color`], a two-stop [`crate::LinearGradient`], or a
    /// multi-stop [`Gradient`] all work. Gradients are evaluated analytically inside the same
    /// rounded, bordered, clipped, and opacity-scaled quad as a solid background: they add no
    /// texture, ramp cache, or extra draw call.
    pub fn bg_gradient(mut self, gradient: impl Into<Background>) -> Self {
        match gradient.into() {
            Background::Solid(color) => {
                self.visual.background = Some(color);
                self.visual.background_gradient = None;
            }
            other => {
                self.visual.background_gradient = other.as_gradient();
            }
        }
        self
    }

    /// Paint a raster image behind this element's children and inside its rounded corners.
    ///
    /// The image uses the same bounded GPU texture cache as [`img`](crate::img) elements and is
    /// painted with the existing image primitive: no additional pipeline, pass, or cache is
    /// created. Repeated tiles are capped by
    /// [`MAX_BACKGROUND_IMAGE_TILES`](crate::MAX_BACKGROUND_IMAGE_TILES).
    pub fn bg_image(
        mut self,
        image: impl Into<Image>,
        size: BackgroundSize,
        repeat: BackgroundRepeat,
        position: BackgroundPosition,
    ) -> Self {
        self.visual.background_image = Some(Box::new(BackgroundImage {
            image: image.into(),
            size,
            repeat,
            position: BackgroundPosition::new(position.x, position.y),
        }));
        self
    }

    /// Paint one centered raster background scaled until it covers the element box.
    pub fn bg_image_cover(self, image: impl Into<Image>) -> Self {
        self.bg_image(
            image,
            BackgroundSize::Cover,
            BackgroundRepeat::NoRepeat,
            BackgroundPosition::CENTER,
        )
    }

    /// Paint one centered raster background scaled until it fits inside the element box.
    pub fn bg_image_contain(self, image: impl Into<Image>) -> Self {
        self.bg_image(
            image,
            BackgroundSize::Contain,
            BackgroundRepeat::NoRepeat,
            BackgroundPosition::CENTER,
        )
    }

    /// Tile a raster background at its decoded pixel size along both axes.
    pub fn bg_image_tiled(self, image: impl Into<Image>) -> Self {
        self.bg_image(
            image,
            BackgroundSize::Auto,
            BackgroundRepeat::Repeat,
            BackgroundPosition::TOP_LEFT,
        )
    }

    /// Remove any raster background.
    pub fn bg_image_none(mut self) -> Self {
        self.visual.background_image = None;
        self
    }

    /// Paint a linear gradient at `angle`, where `0` degrees points to the top of the element.
    ///
    /// `angle` accepts `f32` degrees or a [`crate::GradientDirection`]. At most
    /// [`MAX_GRADIENT_STOPS`](crate::MAX_GRADIENT_STOPS) stops are retained; a bare list of
    /// colors is spaced evenly.
    pub fn bg_linear_gradient(
        self,
        angle: impl Into<GradientAngle>,
        stops: impl Into<ColorStops>,
    ) -> Self {
        self.bg_gradient(Gradient::linear(angle, stops))
    }

    /// Paint a centered elliptical radial gradient sized to the element's farthest corner.
    pub fn bg_radial_gradient(self, stops: impl Into<ColorStops>) -> Self {
        self.bg_gradient(Gradient::radial(stops))
    }

    /// Paint a radial gradient with an explicit ending shape and center.
    pub fn bg_radial_gradient_at(
        self,
        shape: RadialGradientShape,
        center: GradientCenter,
        stops: impl Into<ColorStops>,
    ) -> Self {
        self.bg_gradient(Gradient::radial(stops).shape(shape).center(center))
    }

    /// Paint a conic gradient sweeping clockwise from `from_angle` around the element's center.
    pub fn bg_conic_gradient(
        self,
        from_angle: impl Into<GradientAngle>,
        stops: impl Into<ColorStops>,
    ) -> Self {
        self.bg_gradient(Gradient::conic(from_angle, stops))
    }

    /// Set the opacity of this element and all of its descendants.
    ///
    /// Opacity is paint-only: transparent elements keep their layout, pointer behavior, focus,
    /// and accessibility semantics. Nested opacity values multiply like GPUI and the web.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.visual.opacity = finite_opacity(opacity);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths = Insets::all(width);
        self.visual.border_color = Some(color);
        self.layout.border = TaffyRect {
            left: LengthPercentage::length(width),
            right: LengthPercentage::length(width),
            top: LengthPercentage::length(width),
            bottom: LengthPercentage::length(width),
        };
        self
    }

    /// Set the widths of the inside border in top, right, bottom, left order.
    pub fn border_widths(mut self, widths: Insets) -> Self {
        let widths = Insets {
            top: finite_nonnegative(widths.top),
            right: finite_nonnegative(widths.right),
            bottom: finite_nonnegative(widths.bottom),
            left: finite_nonnegative(widths.left),
        };
        self.visual.border_widths = widths;
        self.layout.border = TaffyRect {
            left: LengthPercentage::length(widths.left),
            right: LengthPercentage::length(widths.right),
            top: LengthPercentage::length(widths.top),
            bottom: LengthPercentage::length(widths.bottom),
        };
        self
    }

    /// Set the shared color used by every non-zero border edge.
    pub fn border_color(mut self, color: Color) -> Self {
        self.visual.border_color = Some(color);
        self
    }

    pub fn border_top_width(mut self, width: f32) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths.top = width;
        self.layout.border.top = LengthPercentage::length(width);
        self
    }

    pub fn border_right_width(mut self, width: f32) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths.right = width;
        self.layout.border.right = LengthPercentage::length(width);
        self
    }

    pub fn border_bottom_width(mut self, width: f32) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths.bottom = width;
        self.layout.border.bottom = LengthPercentage::length(width);
        self
    }

    pub fn border_left_width(mut self, width: f32) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths.left = width;
        self.layout.border.left = LengthPercentage::length(width);
        self
    }

    pub fn border_top(self, width: f32, color: Color) -> Self {
        self.border_top_width(width).border_color(color)
    }

    pub fn border_right(self, width: f32, color: Color) -> Self {
        self.border_right_width(width).border_color(color)
    }

    pub fn border_bottom(self, width: f32, color: Color) -> Self {
        self.border_bottom_width(width).border_color(color)
    }

    pub fn border_left(self, width: f32, color: Color) -> Self {
        self.border_left_width(width).border_color(color)
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.visual.radius = sanitize_corner_radius(radius);
        self.visual.corner_radii = None;
        self
    }

    /// Round each corner independently.
    ///
    /// Per-corner radii replace the single [`Element::rounded`] value. They are paint-only and,
    /// unlike the uniform radius, are not interpolated by style transitions.
    pub fn corner_radii(mut self, radii: Corners) -> Self {
        self.visual.corner_radii = Some(Corners {
            top_left: sanitize_corner_radius(radii.top_left),
            top_right: sanitize_corner_radius(radii.top_right),
            bottom_right: sanitize_corner_radius(radii.bottom_right),
            bottom_left: sanitize_corner_radius(radii.bottom_left),
        });
        self
    }

    fn with_corner(self, apply: impl FnOnce(&mut Corners)) -> Self {
        let mut corners = self.visual.corners(self.visual.radius);
        apply(&mut corners);
        self.corner_radii(corners)
    }

    /// Round the top-left corner.
    pub fn rounded_tl(self, radius: f32) -> Self {
        self.with_corner(|corners| corners.top_left = radius)
    }

    /// Round the top-right corner.
    pub fn rounded_tr(self, radius: f32) -> Self {
        self.with_corner(|corners| corners.top_right = radius)
    }

    /// Round the bottom-right corner.
    pub fn rounded_br(self, radius: f32) -> Self {
        self.with_corner(|corners| corners.bottom_right = radius)
    }

    /// Round the bottom-left corner.
    pub fn rounded_bl(self, radius: f32) -> Self {
        self.with_corner(|corners| corners.bottom_left = radius)
    }

    /// Round both top corners.
    pub fn rounded_t(self, radius: f32) -> Self {
        self.with_corner(|corners| {
            corners.top_left = radius;
            corners.top_right = radius;
        })
    }

    /// Round both bottom corners.
    pub fn rounded_b(self, radius: f32) -> Self {
        self.with_corner(|corners| {
            corners.bottom_left = radius;
            corners.bottom_right = radius;
        })
    }

    /// Round both left corners.
    pub fn rounded_l(self, radius: f32) -> Self {
        self.with_corner(|corners| {
            corners.top_left = radius;
            corners.bottom_left = radius;
        })
    }

    /// Round both right corners.
    pub fn rounded_r(self, radius: f32) -> Self {
        self.with_corner(|corners| {
            corners.top_right = radius;
            corners.bottom_right = radius;
        })
    }

    /// Round every corner to half of the shorter side, producing a pill or circle.
    pub fn rounded_full(self) -> Self {
        self.rounded(MAX_CORNER_RADIUS)
    }

    /// Select how the inside border is painted along its perimeter.
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.visual.border_style = style;
        self
    }

    /// Paint the inside border as one continuous ring.
    pub fn border_solid(self) -> Self {
        self.border_style(BorderStyle::Solid)
    }

    /// Paint the inside border as evenly distributed dashes.
    pub fn border_dashed(self) -> Self {
        self.border_style(BorderStyle::Dashed)
    }

    /// Paint the inside border as evenly distributed dots.
    pub fn border_dotted(self) -> Self {
        self.border_style(BorderStyle::Dotted)
    }

    /// Paint a ring outside the border box without affecting layout.
    ///
    /// The ring follows the element's corner radii, grown by the outline offset and width, and
    /// is painted after the element's own background and border.
    pub fn outline(mut self, width: f32, color: Color) -> Self {
        let offset = self.visual.outline.map_or(0.0, |outline| outline.offset);
        let style = self
            .visual
            .outline
            .map_or(BorderStyle::Solid, |outline| outline.style);
        self.visual.outline = Some(Outline::new(width, color).offset(offset).style(style));
        self
    }

    /// Set the gap between the border box and the outline ring. Negative values are accepted.
    pub fn outline_offset(mut self, offset: f32) -> Self {
        let outline = self
            .visual
            .outline
            .unwrap_or_else(|| Outline::new(0.0, Color::TRANSPARENT));
        self.visual.outline = Some(outline.offset(offset));
        self
    }

    /// Select how the outline ring is painted along its perimeter.
    pub fn outline_style(mut self, style: BorderStyle) -> Self {
        let outline = self
            .visual
            .outline
            .unwrap_or_else(|| Outline::new(0.0, Color::TRANSPARENT));
        self.visual.outline = Some(outline.style(style));
        self
    }

    /// Paint the outline as evenly distributed dashes.
    pub fn outline_dashed(self) -> Self {
        self.outline_style(BorderStyle::Dashed)
    }

    /// Paint the outline as evenly distributed dots.
    pub fn outline_dotted(self) -> Self {
        self.outline_style(BorderStyle::Dotted)
    }

    /// Remove the outline ring.
    pub fn outline_none(mut self) -> Self {
        self.visual.outline = None;
        self
    }

    pub fn rounded_sm(self) -> Self {
        self.rounded(4.0)
    }
    pub fn rounded_md(self) -> Self {
        self.rounded(6.0)
    }
    pub fn rounded_lg(self) -> Self {
        self.rounded(8.0)
    }
    pub fn rounded_xl(self) -> Self {
        self.rounded(12.0)
    }
    pub fn rounded_2xl(self) -> Self {
        self.rounded(16.0)
    }

    /// Paint one CSS-like box shadow without affecting layout.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.visual.shadows = Some(Arc::from([shadow]));
        self
    }

    /// Paint multiple CSS-like box shadows in declaration order.
    ///
    /// As on the web, the first shadow is painted on top of later shadows.
    pub fn shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        self.visual.shadows = Some(collect_box_shadows(shadows));
        self
    }

    pub fn shadow_none(mut self) -> Self {
        self.visual.shadows = None;
        self
    }

    pub fn shadow_sm(self) -> Self {
        self.shadow(shadow_sm_preset())
    }

    pub fn shadow_md(self) -> Self {
        self.shadows(shadow_md_preset())
    }

    pub fn shadow_lg(self) -> Self {
        self.shadows(shadow_lg_preset())
    }

    pub fn shadow_xl(self) -> Self {
        self.shadows(shadow_xl_preset())
    }

    pub fn shadow_2xl(self) -> Self {
        self.shadow(shadow_2xl_preset())
    }

    /// Choose how image, SVG, or path content is fitted into its layout box.
    pub fn object_fit(mut self, fit: ObjectFit) -> Self {
        match &mut self.kind {
            ElementKind::Image(image) => image.object_fit = fit,
            ElementKind::Svg(svg) => svg.object_fit = fit,
            ElementKind::Path(path) => path.object_fit = fit,
            _ => {}
        }
        self
    }

    /// Override a path element's inherited text color with a solid color or linear gradient.
    pub fn path_background(mut self, background: impl Into<Background>) -> Self {
        if let ElementKind::Path(path) = &mut self.kind {
            path.background = Some(background.into());
        }
        self
    }

    /// Replace the four parameter vectors supplied to this custom shader instance.
    pub fn shader_parameters(mut self, parameters: impl Into<ShaderParameters>) -> Self {
        let ElementKind::CustomShader(shader) = &mut self.kind else {
            panic!("shader_parameters can only be applied to a custom shader element");
        };
        shader.parameters = parameters.into();
        self
    }

    /// Apply a render-only transform to SVG content without affecting flexbox layout.
    pub fn svg_transform(mut self, transform: SvgTransform) -> Self {
        if let ElementKind::Svg(svg) = &mut self.kind {
            svg.transform = transform;
        }
        self
    }

    /// Render this element's raster content in grayscale without decoding another image.
    ///
    /// This is `filters([Filter::Grayscale(1.0)])`, or clearing the chain when `false`.
    pub fn grayscale(self, grayscale: bool) -> Self {
        if grayscale {
            self.filters([Filter::Grayscale(1.0)])
        } else {
            self.filters([])
        }
    }

    /// Apply a bounded chain of CSS-shaped filters to this element.
    ///
    /// A chain of colour filters alone applies to this element's own raster content: an image
    /// element's pixels and any `bg_image` tiles on the same element. It costs one color matrix
    /// per primitive and allocates nothing.
    ///
    /// Adding a [`Filter::Blur`] or [`Filter::DropShadow`] promotes the element to a *compositing
    /// group*: the whole subtree — text included — renders into a bounded offscreen texture first,
    /// and the entire chain, colour filters and all, then applies to that texture. See
    /// [`Element::blur`] and [`Element::drop_shadow`].
    ///
    /// At most [`MAX_FILTERS_PER_ELEMENT`](crate::MAX_FILTERS_PER_ELEMENT) filters are retained,
    /// and the colour part of the chain collapses into one matrix before it reaches the GPU, so
    /// the number of declared filters never changes per-frame work.
    pub fn filters(mut self, filters: impl IntoIterator<Item = Filter>) -> Self {
        self.visual.filters = Filters::new(filters);
        self
    }

    /// Append one color filter to this element's chain.
    pub fn filter(mut self, filter: Filter) -> Self {
        self.visual.filters = self.visual.filters.push(filter);
        self
    }

    /// Scale the brightness of this element's raster content. `1.0` leaves it unchanged.
    pub fn brightness(self, amount: f32) -> Self {
        self.filter(Filter::Brightness(amount))
    }

    /// Scale the contrast of this element's raster content. `1.0` leaves it unchanged.
    pub fn contrast(self, amount: f32) -> Self {
        self.filter(Filter::Contrast(amount))
    }

    /// Scale the saturation of this element's raster content. `1.0` leaves it unchanged.
    pub fn saturate(self, amount: f32) -> Self {
        self.filter(Filter::Saturate(amount))
    }

    /// Invert this element's raster content. `0.0` leaves it unchanged.
    pub fn invert(self, amount: f32) -> Self {
        self.filter(Filter::Invert(amount))
    }

    /// Apply a sepia tone to this element's raster content. `0.0` leaves it unchanged.
    pub fn sepia(self, amount: f32) -> Self {
        self.filter(Filter::Sepia(amount))
    }

    /// Rotate the hues of this element's raster content by `degrees`.
    pub fn hue_rotate(self, degrees: f32) -> Self {
        self.filter(Filter::HueRotate(degrees))
    }

    /// Blur this element's whole subtree by `radius` logical pixels of standard deviation.
    ///
    /// This promotes the element to a compositing group: its subtree renders into a bounded
    /// offscreen texture and a separable Gaussian is applied to it, so text blurs with its
    /// background. `radius` is clamped to [`MAX_BLUR_RADIUS`](crate::MAX_BLUR_RADIUS); a radius of
    /// zero declares nothing and allocates nothing.
    pub fn blur(self, radius: f32) -> Self {
        self.filter(Filter::Blur(radius))
    }

    /// Paint a blurred, offset, tinted copy of this element's subtree behind it.
    ///
    /// Unlike [`Element::shadow`], which is an analytic shadow of the element's rounded box, this
    /// follows the subtree's real painted alpha, so text and images cast their own silhouette.
    /// `blur` is the CSS `drop-shadow()` length, half of which is the Gaussian standard deviation,
    /// and is clamped to twice [`MAX_BLUR_RADIUS`](crate::MAX_BLUR_RADIUS). Promotes the element
    /// to a compositing group.
    pub fn drop_shadow(self, offset_x: f32, offset_y: f32, blur: f32, color: Color) -> Self {
        self.filter(Filter::DropShadow(DropShadow::new(
            crate::Vector::new(offset_x, offset_y),
            blur,
            color,
        )))
    }

    /// Transform this element's whole subtree, painting and hit testing alike.
    ///
    /// The transform acts around [`Element::transform_origin`], which defaults to the centre of
    /// the element's border box. Layout is unaffected: the element still occupies its untransformed
    /// box, exactly as a CSS `transform` does. Anything but a whole-pixel translation promotes the
    /// element to a compositing group.
    pub fn transform(mut self, transform: Transform2D) -> Self {
        self.visual.transform = transform;
        self
    }

    /// Move the point a transform acts around, as a fraction of the element's border box.
    ///
    /// `(0.0, 0.0)` is the top-left corner and the default `(0.5, 0.5)` the centre.
    pub fn transform_origin(mut self, x: f32, y: f32) -> Self {
        self.visual.transform_origin = sane_transform_origin(Point::new(x, y));
        self
    }

    /// Translate this element's subtree by logical pixels without affecting layout.
    ///
    /// A whole-pixel translation is a paint offset and needs no offscreen texture.
    pub fn translate(self, x: f32, y: f32) -> Self {
        self.transform(Transform2D::translate(x, y))
    }

    /// Rotate this element's subtree clockwise around its transform origin.
    pub fn rotate_degrees(self, degrees: f32) -> Self {
        self.transform(Transform2D::rotate_degrees(degrees))
    }

    /// Scale this element's subtree around its transform origin.
    pub fn scale(self, x: f32, y: f32) -> Self {
        self.transform(Transform2D::scale(x, y))
    }

    /// Scale this element's subtree uniformly around its transform origin.
    pub fn scale_uniform(self, scale: f32) -> Self {
        self.transform(Transform2D::scale_uniform(scale))
    }

    /// Skew this element's subtree around its transform origin, in degrees.
    pub fn skew_degrees(self, x: f32, y: f32) -> Self {
        self.transform(Transform2D::skew_degrees(x, y))
    }

    /// Blur whatever is already painted behind this element, clipped to its rounded box.
    ///
    /// The renderer copies the target region behind the element, blurs the copy, and draws it
    /// under this element's own background. `radius` is clamped to
    /// [`MAX_BLUR_RADIUS`](crate::MAX_BLUR_RADIUS); zero clears the effect.
    ///
    /// The window surface must support `COPY_SRC`. When it does not, the element paints without
    /// its backdrop and the frame counts it in
    /// [`RenderStats::skipped_layer_effects`](crate::RenderStats).
    pub fn backdrop_blur(mut self, radius: f32) -> Self {
        self.visual.backdrop = self
            .visual
            .backdrop
            .without_blur()
            .push(Filter::Blur(radius));
        self
    }

    /// Apply a bounded colour-filter chain to whatever is already painted behind this element.
    ///
    /// Combine with [`Element::backdrop_blur`] for the usual translucent-material look. Carries
    /// the same surface requirement and the same honest degradation.
    pub fn backdrop_filter(mut self, filters: impl IntoIterator<Item = Filter>) -> Self {
        let blur = self.visual.backdrop.blur();
        let mut chain = Filters::new(filters);
        if blur > 0.0 {
            chain = chain.push(Filter::Blur(blur));
        }
        self.visual.backdrop = chain;
        self
    }

    /// Combine this element's subtree with what is already painted behind it.
    ///
    /// Every mode but [`BlendMode::Normal`] promotes the element to a compositing group, and every
    /// mode but `normal` and [`BlendMode::Screen`] additionally copies the destination. See
    /// `docs/graphics.md` for the exact formulas and their cost.
    pub fn blend_mode(mut self, blend: BlendMode) -> Self {
        self.visual.blend = blend;
        self
    }

    /// Render a replacement element when an image resource has been loading for 200 ms.
    pub fn with_loading<E: IntoElement + 'static>(
        mut self,
        render: impl Fn() -> E + 'static,
    ) -> Self {
        if let ElementKind::Image(image) = &mut self.kind {
            image.loading = Some(ImageReplacement::new(render));
        }
        self
    }

    /// Render a replacement element when an image resource fails to load.
    pub fn with_fallback<E: IntoElement + 'static>(
        mut self,
        render: impl Fn() -> E + 'static,
    ) -> Self {
        if let ElementKind::Image(image) = &mut self.kind {
            image.fallback = Some(ImageReplacement::new(render));
        }
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.typography.color = Some(color);
        self
    }

    pub fn text_size(mut self, size: f32) -> Self {
        self.typography.font_size = Some(size.max(1.0));
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.typography.line_height = Some(height.max(1.0));
        self
    }

    /// Quantize glyph advances to one logical monospace cell width.
    pub fn monospace_width(mut self, width: f32) -> Self {
        self.typography.monospace_width = Some(width.max(1.0));
        self
    }

    pub fn text_xs(self) -> Self {
        self.text_size(12.0).line_height(16.0)
    }
    pub fn text_sm(self) -> Self {
        self.text_size(14.0).line_height(20.0)
    }
    pub fn text_base(self) -> Self {
        self.text_size(16.0).line_height(24.0)
    }
    pub fn text_lg(self) -> Self {
        self.text_size(18.0).line_height(26.0)
    }
    pub fn text_xl(self) -> Self {
        self.text_size(20.0).line_height(28.0)
    }
    pub fn text_2xl(self) -> Self {
        self.text_size(24.0).line_height(32.0)
    }
    pub fn text_3xl(self) -> Self {
        self.text_size(30.0).line_height(36.0)
    }

    pub fn font_weight(mut self, weight: Weight) -> Self {
        self.typography.weight = Some(weight);
        self
    }

    pub fn font_normal(self) -> Self {
        self.font_weight(Weight::NORMAL)
    }

    pub fn font_medium(self) -> Self {
        self.font_weight(Weight::MEDIUM)
    }

    pub fn font_semibold(self) -> Self {
        self.font_weight(Weight::SEMIBOLD)
    }

    pub fn font_bold(self) -> Self {
        self.font_weight(Weight::BOLD)
    }

    /// Set the inherited font slant.
    pub fn font_style(mut self, style: GlyphStyle) -> Self {
        self.typography.font_style = Some(style);
        self
    }

    pub fn italic(self) -> Self {
        self.font_style(GlyphStyle::Italic)
    }

    pub fn not_italic(self) -> Self {
        self.font_style(GlyphStyle::Normal)
    }

    /// Optically thicken descendant glyph stems without selecting a heavier font face.
    pub fn font_thicken(mut self, thicken: bool) -> Self {
        self.typography.font_thicken = Some(thicken);
        self
    }

    /// Underline descendant text with the font's native single-line metrics.
    pub fn underline(mut self) -> Self {
        self.typography.underline = Some(TextUnderline::Single);
        self.typography.underline_wavy = Some(false);
        self.typography.underline_thickness = Some(1.0);
        self
    }

    /// Underline descendant text with two native lines.
    pub fn double_underline(mut self) -> Self {
        self.typography.underline = Some(TextUnderline::Double);
        self.typography.underline_wavy = Some(false);
        self.typography.underline_thickness = Some(1.0);
        self
    }

    /// Strike through descendant text with the font's native metrics.
    pub fn line_through(mut self) -> Self {
        self.typography.strikethrough = Some(true);
        self
    }

    /// Remove inherited underline and strikethrough decoration.
    pub fn text_decoration_none(mut self) -> Self {
        self.typography.underline = Some(TextUnderline::None);
        self.typography.underline_color = Some(None);
        self.typography.underline_wavy = Some(false);
        self.typography.underline_thickness = Some(1.0);
        self.typography.strikethrough = Some(false);
        self.typography.strikethrough_color = Some(None);
        self
    }

    /// Set the inherited underline color, enabling a single underline when needed.
    pub fn text_decoration_color(mut self, color: Color) -> Self {
        if self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None)
        {
            self.typography.underline = Some(TextUnderline::Single);
            self.typography.underline_wavy = Some(false);
            self.typography.underline_thickness = Some(1.0);
        }
        self.typography.underline_color = Some(Some(color));
        self
    }

    /// Use an ordinary solid underline, enabling one when no underline is inherited.
    pub fn text_decoration_solid(mut self) -> Self {
        let enables_underline = self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None);
        if enables_underline {
            self.typography.underline = Some(TextUnderline::Single);
            self.typography.underline_thickness = Some(1.0);
        }
        self.typography.underline_wavy = Some(false);
        self
    }

    /// Use a GPU-rendered spell-checker-style wavy underline.
    pub fn text_decoration_wavy(mut self) -> Self {
        let enables_underline = self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None);
        if enables_underline {
            self.typography.underline = Some(TextUnderline::Single);
            self.typography.underline_thickness = Some(1.0);
        }
        self.typography.underline_wavy = Some(true);
        self
    }

    fn text_decoration_thickness(mut self, thickness: f32) -> Self {
        if self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None)
        {
            self.typography.underline = Some(TextUnderline::Single);
        }
        self.typography.underline_thickness = Some(thickness);
        self
    }

    pub fn text_decoration_0(self) -> Self {
        self.text_decoration_thickness(0.0)
    }

    pub fn text_decoration_1(self) -> Self {
        self.text_decoration_thickness(1.0)
    }

    pub fn text_decoration_2(self) -> Self {
        self.text_decoration_thickness(2.0)
    }

    pub fn text_decoration_4(self) -> Self {
        self.text_decoration_thickness(4.0)
    }

    pub fn text_decoration_8(self) -> Self {
        self.text_decoration_thickness(8.0)
    }

    pub fn font_family(mut self, family: impl Into<FontFamily>) -> Self {
        let family = family.into();
        assert_valid_font_family(&family);
        self.typography.family = Some(family);
        self
    }

    /// Set the inherited OpenType feature table used while shaping descendant text.
    pub fn font_features(mut self, features: FontFeatures) -> Self {
        self.typography.features = Some(features);
        self
    }

    /// Set the ordered families tried after the primary family and before platform fallbacks.
    ///
    /// Passing an empty stack intentionally clears an inherited custom fallback stack.
    pub fn font_fallbacks(mut self, fallbacks: FontFallbacks) -> Self {
        self.typography.fallbacks = Some((!fallbacks.is_empty()).then_some(fallbacks));
        self
    }

    /// Replace the complete inherited font configuration.
    pub fn font(mut self, font: Font) -> Self {
        assert_valid_font_family(&font.family);
        self.typography.family = Some(font.family);
        self.typography.features = Some(font.features);
        self.typography.fallbacks = Some(normalize_fallbacks(font.fallbacks));
        self.typography.weight = Some(font.weight);
        self.typography.font_style = Some(font.style);
        self
    }

    /// Align descendant text lines within their assigned element width.
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.typography.align = Some(align);
        self
    }

    pub fn text_left(self) -> Self {
        self.text_align(TextAlign::Left)
    }

    pub fn text_center(self) -> Self {
        self.text_align(TextAlign::Center)
    }

    pub fn text_right(self) -> Self {
        self.text_align(TextAlign::Right)
    }

    pub fn text_justify(self) -> Self {
        self.text_align(TextAlign::Justify)
    }

    pub fn no_wrap(mut self) -> Self {
        self.typography.wrap = Some(TextWrap::None);
        self
    }

    /// Set inherited CSS-like whitespace behavior.
    pub fn white_space(mut self, white_space: WhiteSpace) -> Self {
        self.typography.wrap = Some(white_space.into());
        self
    }

    /// Allow ordinary word wrapping, matching GPUI and `white-space: normal` on the web.
    pub fn whitespace_normal(self) -> Self {
        self.white_space(WhiteSpace::Normal)
    }

    /// Keep each logical line unwrapped, matching GPUI and `white-space: nowrap` on the web.
    pub fn whitespace_nowrap(self) -> Self {
        self.white_space(WhiteSpace::Nowrap)
    }

    /// Set the replacement used when text exceeds its assigned width.
    pub fn text_overflow(mut self, overflow: TextOverflow) -> Self {
        self.typography.text_overflow = Some(overflow);
        self
    }

    /// Preserve the start of overflowing text and append an ellipsis.
    pub fn text_ellipsis(self) -> Self {
        self.text_overflow(TextOverflow::ellipsis())
    }

    /// Preserve the end of overflowing text and prepend an ellipsis.
    pub fn text_ellipsis_start(self) -> Self {
        self.text_overflow(TextOverflow::ellipsis_start())
    }

    /// Preserve both ends of overflowing text and place an ellipsis in the middle.
    pub fn text_ellipsis_middle(self) -> Self {
        self.text_overflow(TextOverflow::ellipsis_middle())
    }

    /// Limit descendant text leaves to a fixed number of visible lines.
    ///
    /// Combine this with [`Self::text_ellipsis`] when the final visible line should carry an
    /// ellipsis. Values below one are normalized to one.
    pub fn line_clamp(mut self, lines: usize) -> Self {
        self.typography.line_clamp = Some(lines.max(1));
        self.overflow_hidden()
    }

    /// Web-style single-line truncation: no wrapping, hidden overflow, and a trailing ellipsis.
    pub fn truncate(self) -> Self {
        self.overflow_hidden().whitespace_nowrap().text_ellipsis()
    }

    /// Use cheap one-glyph-per-character shaping for app-controlled text and fonts.
    ///
    /// Keep the default advanced mode for complex scripts, ligatures, or general font fallback.
    pub fn text_shaping_basic(mut self) -> Self {
        self.typography.shaping = Some(TextShaping::Basic);
        self
    }

    pub fn text_shaping(mut self, shaping: TextShaping) -> Self {
        self.typography.shaping = Some(shaping);
        self
    }

    pub fn wrap(mut self) -> Self {
        self.typography.wrap = Some(TextWrap::Word);
        self
    }

    pub fn overflow_hidden(mut self) -> Self {
        self.layout.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Hidden,
        };
        self
    }

    pub fn overflow_y_scroll(mut self) -> Self {
        self.layout.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Scroll,
        };
        self
    }

    /// Apply one retained scroll request. Reusing the revision preserves subsequent user scrolling.
    pub fn scroll_to(mut self, offset: crate::Vector, revision: u64) -> Self {
        self.scroll_request = Some(ScrollRequest {
            revision,
            offset,
            child: None,
        });
        self.scroll_to_end_revision = None;
        self
    }

    /// Scroll a declared child into the leading edge, with a signed logical offset.
    pub fn scroll_to_child(mut self, child: usize, offset: crate::Vector, revision: u64) -> Self {
        self.scroll_request = Some(ScrollRequest {
            revision,
            offset,
            child: Some(child),
        });
        self.scroll_to_end_revision = None;
        self
    }

    /// Follow the end of an ordinary vertical overflow container when `revision` changes.
    ///
    /// The first declaration starts at the end. Later revisions keep following only while the
    /// user was already at the previous end; scrolling away pauses following, and returning to
    /// the end resumes it on the next revision. This performs no scheduling or per-frame work.
    pub fn scroll_to_end(mut self, revision: u64) -> Self {
        self.scroll_request = None;
        self.scroll_to_end_revision = Some(revision);
        self
    }

    /// Bind this clipped viewport to a fixed-height [`VirtualList`].
    ///
    /// The mounted rows remain application-controlled, while QuickGUI owns the native-style
    /// retained scrollbar, wheel routing, pointer capture, hover expansion, and one-shot
    /// autohide. Offset changes rebuild the virtualized view only when the offset actually moves;
    /// hover and visibility changes stay paint-only.
    pub fn virtual_scroll(mut self, list: &VirtualList) -> Self {
        self.layout.overflow.y = Overflow::Hidden;
        self.virtual_scroll = Some(VirtualScrollStyle {
            handle: list.scroll_handle(),
            max_offset_y: list.max_scroll_offset(),
            measurement_revision: 0,
            mount: list.scroll_mount(),
        });
        self
    }

    /// Bind this clipped viewport to a differently sized [`ListState`].
    ///
    /// Use [`ListState::visible_rows`] and [`ListState::render_rows`] to mount a bounded normal-flow
    /// slice. QuickGUI measures those rows after Taffy layout, preserves the logical top item while
    /// estimates converge, and schedules only the correcting rebuilds that actually changed a
    /// measurement. Wheel input, scrollbar capture, hover expansion, and autohide use the same
    /// retained path as ordinary scrolling and fixed-height virtualization.
    pub fn variable_virtual_scroll(mut self, list: &ListState) -> Self {
        self.layout.overflow.y = Overflow::Hidden;
        let (handle, max_offset_y, measurement_revision, mount) = list.scroll_binding();
        self.virtual_scroll = Some(VirtualScrollStyle {
            handle,
            max_offset_y,
            measurement_revision,
            mount,
        });
        self
    }

    pub fn absolute(mut self) -> Self {
        self.layout.position = Position::Absolute;
        self
    }

    /// Create a stacking context within the current render plane.
    ///
    /// Values are relative to the nearest ancestor stacking context. Elements sharing a value stay
    /// batched and retain source order.
    pub fn z_index(mut self, value: i16) -> Self {
        self.z_index = Some(value);
        self
    }

    /// Paint this subtree in the viewport overlay plane above ordinary application content.
    ///
    /// The element becomes absolutely positioned, escapes ancestor clipping, and blocks pointer
    /// events from falling through its own bounds.
    pub fn overlay(mut self) -> Self {
        self.layout.position = Position::Absolute;
        self.plane = Some(ScenePlane::Overlay);
        self.portal = true;
        self.blocks_pointer = true;
        self
    }

    /// Position this floating element relative to a stable element ID.
    pub fn anchor_to(mut self, target: impl Into<ElementId>, placement: AnchorPlacement) -> Self {
        self = self.overlay();
        self.anchor = Some(AnchorStyle {
            rounding_scale: None,
            target: AnchorTarget::Element(target.into()),
            placement,
            gap: DEFAULT_ANCHOR_GAP,
            align_offset: 0.0,
            viewport_margin: DEFAULT_VIEWPORT_MARGIN,
            flip: true,
            sticky: true,
        });
        self
    }

    /// Position a viewport overlay relative to a logical point.
    ///
    /// This is the cursor-point counterpart of [`Self::anchor_to`] and is intended for context
    /// menus. Placement uses the same flip, alternate-alignment, and viewport-clamping rules.
    pub fn anchor_at(mut self, point: crate::Point, placement: AnchorPlacement) -> Self {
        self = self.overlay();
        let point = crate::Point::new(
            if point.x.is_finite() { point.x } else { 0.0 },
            if point.y.is_finite() { point.y } else { 0.0 },
        );
        self.anchor = Some(AnchorStyle {
            rounding_scale: None,
            target: AnchorTarget::Point(point),
            placement,
            gap: 0.0,
            align_offset: 0.0,
            viewport_margin: DEFAULT_VIEWPORT_MARGIN,
            flip: true,
            sticky: true,
        });
        self
    }

    /// Set the distance between an anchored surface and its trigger.
    pub fn anchor_gap(mut self, gap: f32) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.gap = gap.max(0.0);
        }
        self
    }

    /// Shift an anchored surface along its cross axis before collision handling runs.
    ///
    /// [`Self::anchor_gap`] moves the surface away from its anchor on the placement side; this
    /// moves it along the perpendicular axis, so a start-aligned popup can hang slightly past the
    /// trigger without changing which side it opens on. Positive values move right for a top or
    /// bottom placement and down for a left or right placement. The offset is applied before the
    /// surface is clamped into the viewport, so it can never push a surface off screen.
    pub fn anchor_align_offset(mut self, offset: f32) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.align_offset = if offset.is_finite() { offset } else { 0.0 };
        }
        self
    }

    /// Snap the anchor to device pixels and its translation to logical pixels.
    /// This matches native GPUI overlay placement while keeping the surface in the viewport.
    pub fn anchor_offset_rounding(mut self, scale: f32) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.rounding_scale = (scale.is_finite() && scale > 0.).then_some(scale);
        }
        self
    }

    /// Enable side and alignment changes when the preferred placement does not fit.
    /// Disable this for surfaces that must slide into the viewport without flipping.
    pub fn anchor_flip(mut self, flip: bool) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.flip = flip;
        }
        self
    }

    /// Choose whether an anchored surface is kept inside the collision viewport.
    ///
    /// QuickGUI clamps anchored surfaces into the margin-inset viewport by default, so a popup
    /// stays fully visible while its trigger approaches an edge. Passing `false` lets the surface
    /// stay locked to its anchor and travel off screen with it, which is what a popup pinned to a
    /// scrolling row wants when the row leaves the viewport.
    pub fn anchor_sticky(mut self, sticky: bool) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.sticky = sticky;
        }
        self
    }

    /// Set the minimum distance between an anchored surface and the content viewport edge.
    pub fn viewport_margin(mut self, margin: f32) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.viewport_margin = margin.max(0.0);
        }
        self
    }

    /// Publish the placement this anchored element actually resolved to into an owned handle.
    ///
    /// A declared [`AnchorPlacement`] is a preference. QuickGUI flips the side and re-aligns the
    /// cross axis whenever the preference does not fit, so presentation that must follow the real
    /// placement — a popover arrow, a directional transform origin, a popup sized to the room it
    /// was given — reads [`AnchorPlacementHandle::resolved`] instead of the declared preference.
    ///
    /// The handle is written during the paint QuickGUI was already performing. When the resolved
    /// value changes, exactly one correcting frame is requested; an unchanged placement adds no
    /// redraw source, so a settled window stays settled. Binding it on an element with no anchor
    /// does nothing.
    pub fn report_anchor_placement(mut self, handle: AnchorPlacementHandle) -> Self {
        self.anchor_placement = Some(handle);
        self
    }

    /// Whether this element publishes its resolved anchor placement.
    pub fn reports_anchor_placement(&self) -> bool {
        self.anchor_placement.is_some()
    }

    /// Publish this element's painted bounds through an application-owned handle.
    ///
    /// The handle receives the window-relative bounds of the frame QuickGUI was already painting,
    /// so behavior that must match real layout — a splitter total, a scroll area's viewport and
    /// content extents — reads [`LayoutBoundsHandle::bounds`] instead of guessing. When the bounds
    /// change, exactly one correcting frame is requested; unchanged bounds add no redraw source.
    pub fn report_bounds(mut self, handle: LayoutBoundsHandle) -> Self {
        self.layout_bounds = Some(handle);
        self
    }

    /// Whether this element publishes its painted bounds.
    pub fn reports_bounds(&self) -> bool {
        self.layout_bounds.is_some()
    }

    /// Show a delayed, pointer-passive GPU tooltip while this element is hovered.
    ///
    /// The detached tooltip tree is laid out only after its exact delay expires. Entering a
    /// pending tooltip schedules no frame loop, and moving between ordinary points inside the
    /// trigger does not rebuild the application view.
    pub fn tooltip(mut self, tooltip: impl Into<Tooltip>) -> Self {
        let tooltip = tooltip.into();
        if self.accessibility.description.is_none() {
            self.accessibility.description = tooltip.accessibility_description.clone();
        }
        self.tooltip = Some(tooltip);
        self
    }

    /// Prevent pointer events inside this element from reaching lower visual layers.
    pub fn block_pointer(mut self) -> Self {
        self.blocks_pointer = true;
        self
    }

    /// Set web-style native window dragging behavior for this element's layout box.
    pub fn app_region(mut self, region: AppRegion) -> Self {
        self.app_region = Some(region);
        self
    }

    /// Make this element a native window drag region.
    pub fn app_region_drag(self) -> Self {
        self.app_region(AppRegion::Drag)
    }

    /// Restore normal pointer input inside an ancestor window drag region.
    pub fn app_region_no_drag(self) -> Self {
        self.app_region(AppRegion::NoDrag)
    }

    pub fn relative(mut self) -> Self {
        self.layout.position = Position::Relative;
        self
    }

    pub fn top(mut self, value: f32) -> Self {
        self.layout.inset.top = LengthPercentageAuto::length(value);
        self
    }

    pub fn right(mut self, value: f32) -> Self {
        self.layout.inset.right = LengthPercentageAuto::length(value);
        self
    }

    pub fn bottom(mut self, value: f32) -> Self {
        self.layout.inset.bottom = LengthPercentageAuto::length(value);
        self
    }

    pub fn left(mut self, value: f32) -> Self {
        self.layout.inset.left = LengthPercentageAuto::length(value);
        self
    }

    pub fn inset_0(mut self) -> Self {
        let zero = LengthPercentageAuto::length(0.0);
        self.layout.inset = TaffyRect {
            left: zero,
            right: zero,
            top: zero,
            bottom: zero,
        };
        self
    }

    // ---------------------------------------------------------------------------------------
    // Direction-relative alignment and extended text styling.
    //
    // Everything below inherits through the subtree like the other typography helpers and is
    // resolved once per layout build, so retained shaping keys stay canonical.
    // ---------------------------------------------------------------------------------------

    /// Align text to the inline start edge (left in LTR, right in RTL).
    pub fn text_start(self) -> Self {
        self.text_align(TextAlign::Start)
    }

    /// Align text to the inline end edge (right in LTR, left in RTL).
    pub fn text_end(self) -> Self {
        self.text_align(TextAlign::End)
    }

    /// Force the base paragraph direction used when shaping bidirectional text.
    ///
    /// [`Element::rtl`] already sets this for its subtree. Use this helper to override shaping
    /// direction without mirroring layout.
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.typography.direction = Some(direction);
        self
    }

    /// Paint one drop shadow beneath this subtree's glyphs.
    ///
    /// The offset and color are exact. `blur` is approximated: the shadow is painted as a set of
    /// offset copies spread over the blur radius with proportionally reduced alpha, which reads
    /// like a soft shadow but is not a true Gaussian blur. Offsets are clamped to
    /// [`TextShadow::MAX_OFFSET`] and the blur radius to [`TextShadow::MAX_BLUR`].
    pub fn text_shadow(mut self, offset_x: f32, offset_y: f32, blur: f32, color: Color) -> Self {
        self.typography.shadow = Some(Some(TextShadow::new(offset_x, offset_y, blur, color)));
        self
    }

    /// Remove any inherited text shadow.
    pub fn text_shadow_none(mut self) -> Self {
        self.typography.shadow = Some(None);
        self
    }

    /// Add `spacing` logical pixels of advance after every glyph cluster.
    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.typography.letter_spacing = Some(sane_text_spacing(spacing));
        self
    }

    /// Add `spacing` logical pixels of advance after every space character.
    pub fn word_spacing(mut self, spacing: f32) -> Self {
        self.typography.word_spacing = Some(sane_text_spacing(spacing));
        self
    }

    /// Case-map non-editable text before shaping.
    ///
    /// The transform applies to [`text`](crate::text) and [`styled_text`](crate::styled_text)
    /// content only. Selection, copy, and accessibility keep reporting the original string, and
    /// editable [`text_input`](crate::text_input) content is never transformed so that its value,
    /// caret indices, and IME state stay byte-identical to the controlled value.
    pub fn text_transform(mut self, transform: TextTransform) -> Self {
        self.typography.transform = Some(Some(transform));
        self
    }

    /// Render text uppercased.
    pub fn uppercase(self) -> Self {
        self.text_transform(TextTransform::Uppercase)
    }

    /// Render text lowercased.
    pub fn lowercase(self) -> Self {
        self.text_transform(TextTransform::Lowercase)
    }

    /// Render every word's first character uppercased.
    pub fn capitalize(self) -> Self {
        self.text_transform(TextTransform::Capitalize)
    }

    /// Remove an inherited case mapping.
    pub fn text_transform_none(mut self) -> Self {
        self.typography.transform = Some(None);
        self
    }

    /// Draw a line above this subtree's text.
    pub fn overline(mut self) -> Self {
        self.typography.overline = Some(true);
        self
    }

    /// Draw a colored line above this subtree's text.
    pub fn overline_color(mut self, color: Color) -> Self {
        self.typography.overline = Some(true);
        self.typography.overline_color = Some(Some(color));
        self
    }

    /// Choose where a line may break inside a run of characters.
    pub fn word_break(mut self, word_break: WordBreak) -> Self {
        self.typography.word_break = Some(word_break);
        self
    }

    /// Choose whether an otherwise unbreakable word may be broken to avoid overflow.
    pub fn overflow_wrap(mut self, overflow_wrap: OverflowWrap) -> Self {
        self.typography.overflow_wrap = Some(overflow_wrap);
        self
    }

    /// Choose whether author-placed soft hyphens (`U+00AD`) may render at a break.
    pub fn hyphens(mut self, hyphens: Hyphens) -> Self {
        self.typography.hyphens = Some(hyphens);
        self
    }
}

fn sanitize_corner_radius(radius: f32) -> f32 {
    if radius.is_finite() {
        radius.clamp(0.0, MAX_CORNER_RADIUS)
    } else {
        0.0
    }
}
