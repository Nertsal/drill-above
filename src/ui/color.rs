use super::*;

pub fn color_selector<'a>(
    cx: &'a geng::ui::Controller,
    color: &mut Rgba<f32>,
    float_scale: &mut bool,
    hsv_mode: &mut Option<Hsva<f32>>,
    font: &'a Rc<geng::Font>,
    text_size: f32,
) -> impl geng::ui::Widget + 'a {
    use geng::ui::*;

    let slider = |name, range, value: &mut f32| ui::slider(cx, name, value, range, font, text_size);

    let select = match hsv_mode {
        Some(hsva) => {
            let scale = if *float_scale {
                0.0..=1.0
            } else {
                hsva.s *= 100.0;
                hsva.v *= 100.0;
                0.0..=100.0
            };
            let ui = geng::ui::column![
                slider(
                    "Hue",
                    if *float_scale {
                        0.0..=1.0
                    } else {
                        hsva.h *= 360.0;
                        0.0..=360.0
                    },
                    &mut hsva.h
                ),
                slider("Saturation", scale.clone(), &mut hsva.s),
                slider("Value", scale, &mut hsva.v),
            ];
            if !*float_scale {
                hsva.h /= 360.0;
                hsva.s /= 100.0;
                hsva.v /= 100.0;
            }
            *color = (*hsva).into();
            ui
        }
        None => {
            let scale = if *float_scale {
                0.0..=1.0
            } else {
                *color = color.map_rgb(|x| x * 255.0);
                0.0..=255.0
            };
            let ui = geng::ui::column![
                slider("Red", scale.clone(), &mut color.r),
                slider("Green", scale.clone(), &mut color.g),
                slider("Blue", scale, &mut color.b),
            ];
            if !*float_scale {
                *color = color.map_rgb(|x| x / 255.0);
            }
            ui
        }
    };
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
                    let text = if hsv_mode.is_some() { "HSV" } else { "RGB" };
                    let mode = geng::ui::Button::new(cx, text);
                    if mode.was_clicked() {
                        if hsv_mode.is_some() {
                            *hsv_mode = None;
                        } else {
                            *hsv_mode = Some((*color).into());
                        }
                    }
                    mode
                },
            ],
            select,
        ]
    ]
}
