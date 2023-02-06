use batbox::color::Hsla;

use super::*;

/// The color mode that is used in color selection: RGB, HSV, or HSL.
#[derive(Debug, Clone, Copy)]
pub enum ColorMode {
    Rgb(Rgba<f32>),
    Hsv(Hsva<f32>),
    Hsl(Hsla<f32>),
}

impl ColorMode {
    pub fn to_rgba(self) -> Rgba<f32> {
        match self {
            ColorMode::Rgb(rgba) => rgba,
            ColorMode::Hsv(hsva) => hsva.into(),
            ColorMode::Hsl(hsla) => hsla.into(),
        }
    }
}

pub fn color_selector<'a>(
    cx: &'a geng::ui::Controller,
    color: &mut Rgba<f32>,
    float_scale: &mut bool,
    color_mode: &mut Option<ColorMode>,
    font: Rc<geng::Font>,
    text_size: f32,
) -> impl geng::ui::Widget + 'a {
    use geng::ui::*;

    let slider =
        |name, range, value: &mut f32| ui::slider(cx, name, value, range, font.clone(), text_size);

    if color_mode.is_none() {
        *color_mode = Some(ColorMode::Rgb(*color));
    }
    let color_mode = color_mode.as_mut().unwrap();
    let (mode_name, mut components) = match color_mode {
        ColorMode::Rgb(rgba) => (
            "RGB",
            [
                (&mut rgba.r, 0.0..=100.0, "Red"),
                (&mut rgba.g, 0.0..=100.0, "Green"),
                (&mut rgba.b, 0.0..=100.0, "Blue"),
            ],
        ),
        ColorMode::Hsv(hsva) => (
            "HSV",
            [
                (&mut hsva.h, 0.0..=360.0, "Hue"),
                (&mut hsva.s, 0.0..=100.0, "Saturation"),
                (&mut hsva.v, 0.0..=100.0, "Value"),
            ],
        ),
        ColorMode::Hsl(hsla) => (
            "HSL",
            [
                (&mut hsla.h, 0.0..=360.0, "Hue"),
                (&mut hsla.s, 0.0..=100.0, "Saturation"),
                (&mut hsla.l, 0.0..=100.0, "Lightness"),
            ],
        ),
    };

    let select = {
        let ui = geng::ui::column(
            components
                .iter_mut()
                .map(|(value, range, name)| {
                    if *float_scale {
                        *range = 0.0..=1.0;
                    }
                    **value *= *range.end();
                    let range = *range.start() as f64..=*range.end() as f64;
                    slider(name.to_owned(), range, value).boxed()
                })
                .collect(),
        );
        for (value, range, _) in components {
            *value /= *range.end();
        }
        ui
    };

    *color = color_mode.to_rgba();

    geng::ui::row![
        geng::ui::ColorBox::divider(*color, text_size).padding_right(text_size.into()),
        geng::ui::column![
            geng::ui::row![
                {
                    let scale = geng::ui::Button::new(cx, "Scale");
                    if scale.was_clicked() {
                        *float_scale = !*float_scale;
                    }
                    scale.padding_right(text_size.into())
                },
                {
                    let mode = geng::ui::Button::new(cx, mode_name);
                    if mode.was_clicked() {
                        let color = *color;
                        *color_mode = match color_mode {
                            ColorMode::Rgb(_) => ColorMode::Hsv(color.into()),
                            ColorMode::Hsv(_) => ColorMode::Hsl(color.into()),
                            ColorMode::Hsl(_) => ColorMode::Rgb(color),
                        };
                    }
                    mode
                },
            ],
            select,
        ]
    ]
}
