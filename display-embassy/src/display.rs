use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::RgbColor,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder},
    text::{Baseline, TextStyle},
};

/// Some shared styles
pub struct Styles {
    pub char: MonoTextStyle<'static, Rgb565>,
    pub text: TextStyle,
    pub black_fill: PrimitiveStyle<Rgb565>,
    pub white_fill: PrimitiveStyle<Rgb565>,
}

impl Styles {
    pub fn new() -> Styles {
        let char = MonoTextStyle::new(&profont::PROFONT_24_POINT, Rgb565::WHITE);
        let text = TextStyle::with_baseline(Baseline::Top);
        let black_fill = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::BLACK)
            .build();
        let white_fill = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::WHITE)
            .build();
        Styles {
            char,
            text,
            black_fill,
            white_fill,
        }
    }
}
