use super::*;

pub fn report_err<T, E: Display>(result: Result<T, E>, msg: impl AsRef<str>) -> Result<T, ()> {
    match result {
        Err(err) => {
            error!("{}: {err}", msg.as_ref());
            Err(())
        }
        Ok(value) => Ok(value),
    }
}

pub fn report_warn<T, E: Display>(result: Result<T, E>, msg: impl AsRef<str>) -> Result<T, ()> {
    match result {
        Err(err) => {
            warn!("{}: {err}", msg.as_ref());
            Err(())
        }
        Ok(value) => Ok(value),
    }
}

pub fn load_state(
    geng: &Geng,
    future: impl Future<Output = impl geng::State> + 'static,
) -> impl geng::State {
    geng::LoadingScreen::new(geng, geng::EmptyLoadingScreen::new(geng), future)
}

pub fn smoothstep<T: Float>(x: T) -> T {
    T::from_f32(3.0) * x * x - T::from_f32(2.0) * x * x * x
}

pub fn pixel_perfect_pos(pos: vec2<Coord>) -> vec2<f32> {
    let pos = pos.map(Coord::as_f32);
    let pixel = pos.map(|x| (x * PIXELS_PER_UNIT as f32).round());
    pixel / PIXELS_PER_UNIT as f32
}

pub fn aabb_outline(aabb: Aabb2<f32>) -> Chain<f32> {
    let [a, b, c, d] = aabb.corners();
    Chain::new(vec![(a + b) / 2.0, a, d, c, b, (a + b) / 2.0])
}

pub fn fit_text(text: impl AsRef<str>, font: impl AsRef<geng::Font>, target: Aabb2<f32>) -> f32 {
    // TODO: check height
    target.width()
        / font
            .as_ref()
            .measure_bounding_box(
                text.as_ref(),
                vec2(geng::TextAlign::LEFT, geng::TextAlign::LEFT),
            )
            .unwrap()
            .width()
}

pub fn split_text_lines(
    text: impl AsRef<str>,
    font: impl AsRef<geng::Font>,
    size: f32,
    target_width: f32,
) -> Vec<String> {
    let font = font.as_ref();
    let mut lines = Vec::new();

    let measure = |str: &str| {
        font.measure_bounding_box(str, vec2(geng::TextAlign::LEFT, geng::TextAlign::LEFT))
            .unwrap_or(Aabb2::ZERO)
            .width()
            * size
    };

    for raw_line in text.as_ref().lines() {
        let mut line = String::new();
        for word in raw_line.split_whitespace() {
            if line.is_empty() {
                line.push_str(word);
            } else {
                let width = measure(&line);
                if width + measure(" ") + measure(word) < target_width {
                    // Word fits in the line
                    line.push(' ');
                    line.push_str(word);
                } else {
                    // Start new line
                    let mut new_line = String::new();
                    std::mem::swap(&mut new_line, &mut line);
                    lines.push(new_line);
                    line = word.to_owned();
                }
            }
        }
        lines.push(line);
    }

    lines
}
