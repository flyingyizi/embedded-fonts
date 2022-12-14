//! "wenquanyi_12pt.rs" generated by below command:
//! examples>..\tools\convert-bdf --range "中国欢迎China welcomes日本へようこそWelcome to Japan북한 환영Welcome North Korea" wenquanyi_12pt.bdf`

use embedded_fonts::BdfTextStyle;

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    // primitives::Rectangle,
    text::Text,
};

use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};

mod generated;

use generated::FONT_WENQUANYI_12PT;

fn main() -> Result<(), std::convert::Infallible> {
    let mut display = SimulatorDisplay::<Rgb888>::new(Size::new(400, 150));

    let my_style = BdfTextStyle::new(&FONT_WENQUANYI_12PT, Rgb888::BLUE);

    Text::new("中国欢迎China welcomes", Point::new(5, 30), my_style).draw(&mut display)?;
    Text::new("북한 환영Welcome North Korea", Point::new(5, 60), my_style).draw(&mut display)?;
    Text::new(
        "日本へようこそWelcome to Japan",
        Point::new(5, 90),
        my_style,
    )
    .draw(&mut display)?;

    let output_settings = OutputSettingsBuilder::new().scale(2).build();
    Window::new("BDF Font", &output_settings).show_static(&display);

    Ok(())
}
