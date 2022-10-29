pub use mybdf::{BdfFont, BdfGlyph};
pub use mystyle::BdfTextStyle;

mod mystyle {
    use embedded_graphics::{
        prelude::*,
        primitives::Rectangle,
        text::{
            renderer::{CharacterStyle, TextMetrics, TextRenderer},
            Baseline,
        },
    };

    use super::mybdf::BdfFont;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct BdfTextStyle<'a, C> {
        font: &'a BdfFont<'a>,
        color: C,
    }

    impl<'a, C: PixelColor> BdfTextStyle<'a, C> {
        pub fn new(font: &'a BdfFont<'a>, color: C) -> Self {
            Self { font, color }
        }
    }

    impl<C: PixelColor> CharacterStyle for BdfTextStyle<'_, C> {
        type Color = C;

        fn set_text_color(&mut self, text_color: Option<Self::Color>) {
            // TODO: support transparent text
            if let Some(color) = text_color {
                self.color = color;
            }
        }

        // TODO: implement additional methods
    }

    impl<C: PixelColor> TextRenderer for BdfTextStyle<'_, C> {
        type Color = C;

        fn draw_string<D>(
            &self,
            text: &str,
            mut position: Point,
            _baseline: Baseline,
            target: &mut D,
        ) -> Result<Point, D::Error>
        where
            D: DrawTarget<Color = Self::Color>,
        {
            // TODO: handle baseline

            for c in text.chars() {
                let glyph = self.font.get_glyph(c);

                glyph.draw(position, self.color, self.font.data, target)?;

                position.x += glyph.device_width as i32;
            }

            Ok(position)
        }

        fn draw_whitespace<D>(
            &self,
            width: u32,
            position: Point,
            _baseline: Baseline,
            _target: &mut D,
        ) -> Result<Point, D::Error>
        where
            D: DrawTarget<Color = Self::Color>,
        {
            // TODO: handle baseline

            Ok(position + Size::new(width, 0))
        }

        fn measure_string(&self, text: &str, position: Point, _baseline: Baseline) -> TextMetrics {
            // TODO: handle baseline
            let dx = text
                .chars()
                .map(|c| self.font.get_glyph(c).device_width)
                .sum();

            // TODO: calculate bounding box
            TextMetrics {
                bounding_box: Rectangle::new(position, Size::zero()),
                next_position: position + Size::new(dx, 0),
            }
        }

        fn line_height(&self) -> u32 {
            self.font.line_height
        }
    }
}

mod mybdf {
    use embedded_graphics::{
        iterator::raw::RawDataSlice,
        pixelcolor::raw::{LittleEndian, RawU1},
        prelude::*,
        primitives::Rectangle,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct BdfFont<'a> {
        pub replacement_character: usize,
        pub line_height: u32,
        pub glyphs: &'a [BdfGlyph],
        pub data: &'a [u8],
    }

    impl<'a> BdfFont<'a> {
        pub fn get_glyph(&self, c: char) -> &'a BdfGlyph {
            self.glyphs
                .iter()
                .find(|g| g.character == c)
                .unwrap_or_else(|| &self.glyphs[self.replacement_character])
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct BdfGlyph {
        pub character: char,
        pub bounding_box: Rectangle,
        pub device_width: u32,
        pub start_index: usize,
    }

    impl BdfGlyph {
        pub fn draw<D: DrawTarget>(
            &self,
            position: Point,
            color: D::Color,
            data: &[u8],
            target: &mut D,
        ) -> Result<(), D::Error> {
            let mut data_iter = RawDataSlice::<RawU1, LittleEndian>::new(data).into_iter();

            if self.start_index > 0 {
                data_iter.nth(self.start_index - 1);
            }

            self.bounding_box
                .translate(position)
                .points()
                .zip(data_iter)
                .filter(|(_p, c)| *c == RawU1::new(1))
                .map(|(p, _c)| Pixel(p, color))
                .draw(target)
        }
    }
}
