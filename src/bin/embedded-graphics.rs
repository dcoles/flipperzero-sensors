//! Flipper Zero App demoing use of the `embedded-graphics` crate.

#![no_main]
#![no_std]

use core::{ffi::{c_int, CStr}, mem, time::Duration};

use flipperzero::furi::thread::sleep;
use flipperzero_rt::{entry, manifest};
use flipperzero_sys as sys;

use eg_seven_segment::SevenSegmentStyleBuilder;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle}, pixelcolor::BinaryColor, prelude::*, primitives::{Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle}, text::{Alignment, Baseline, Text, TextStyleBuilder}
};
use shared::{furi::record::Record, gui::Gui};

manifest!(
    name = "Embedded Graphics",
    app_version = 1,
    has_icon = true,
    // See https://github.com/flipperzero-rs/flipperzero/blob/v0.7.2/docs/icons.md for icon format
    icon = "../rustacean-10x10.icon",
);

macro_rules! sprintf {
    ($fmt:expr) => {
        ::flipperzero::furi::string::FuriString::from($fmt)
    };
    ($fmt:expr, $($arg:expr),+) => {
        {
            let mut s = ::flipperzero::furi::string::FuriString::new();
            ::flipperzero_sys::furi_string_printf(s.as_mut_ptr(), ($fmt as &CStr).as_ptr(), $($arg),+);

            s
        }
    }
}

/// Draw some primitive shapes and some text underneath.
fn draw_shapes<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    // Create styles used by the drawing operations.
    let thin_stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    let thick_stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 3);
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);
    let character_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

    let yoffset = 14;

        // Draw a triangle.
        Triangle::new(
            Point::new(16, 16 + yoffset),
            Point::new(16 + 16, 16 + yoffset),
            Point::new(16 + 8, yoffset),
        )
        .into_styled(thin_stroke)
        .draw(display)?;

        // Draw a filled square
        Rectangle::new(Point::new(52, yoffset), Size::new(16, 16))
            .into_styled(fill)
            .draw(display)?;

        // Draw a circle with a 3px wide stroke.
        Circle::new(Point::new(88, yoffset), 17)
            .into_styled(thick_stroke)
            .draw(display)?;

        // Draw centered text.
        let text = "embedded-graphics";
        Text::with_alignment(
            text,
            display.bounding_box().center() + Point::new(0, 15),
            character_style,
            Alignment::Center,
        )
        .draw(display)?;

    Ok(())
}

/// Draws a digital clock with the current local time to the specified display
fn draw_clock<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    // Create a BinaryColor style
    let character_style = SevenSegmentStyleBuilder::new()
        .segment_color(BinaryColor::On)
        .build();

    // Create a centered alignment for text
    let text_style = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Middle)
        .build();


    let mut datetime: sys::DateTime;
    unsafe {
        datetime = mem::zeroed();
        sys::furi_hal_rtc_get_datetime(&raw mut datetime);
    }

    let time = unsafe { sprintf!(c"%02d:%02d:%02d", datetime.hour as c_int, datetime.minute as c_int, datetime.second as c_int) };
    let time = time.as_c_str().to_str().unwrap();

    // Create text from current time and draw to display
    Text::with_text_style(
        &time,
        display.bounding_box().center(),
        character_style,
        text_style,
    )
    .draw(display)?;

    Ok(())
}

entry!(main);
fn main(_args: Option<&CStr>) -> i32 {
    let gui = Record::<Gui>::open();

    let mut canvas = unsafe { gui.direct_draw_aquire() };

    canvas.clear();
    draw_shapes(&mut canvas).unwrap();
    canvas.commit();
    sleep(Duration::from_secs(3));

    for _ in 0..50 {
        canvas.clear();
        draw_clock(&mut canvas).unwrap();
        canvas.commit();
        sleep(Duration::from_millis(100));
    }

    unsafe { gui.direct_draw_release(); }

    0
}
