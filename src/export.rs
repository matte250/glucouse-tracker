use std::cmp::Reverse;
use std::error::Error;
use std::path::Path;

#[cfg(feature = "gui")]
use eframe::egui::ColorImage;
use plotters::prelude::{
    BLUE, ChartBuilder, IntoDrawingArea, LineSeries, PathElement, RGBColor, WHITE,
};
use plotters_bitmap::BitMapBackend;
use printpdf::*;

use crate::models::GlucoseReading;

const PAGE_W: f32 = 210.0; // A4 width in mm
const PAGE_H: f32 = 297.0; // A4 height in mm
const MARGIN: f32 = 15.0;
const FONT_SIZE_HEADER: f32 = 10.0;
const FONT_SIZE_ROW: f32 = 9.0;
const ROW_HEIGHT: f32 = 5.0;

const COL_DATE_X: f32 = 15.0;
const COL_TIME_X: f32 = 60.0;
const COL_VALUE_X: f32 = 105.0;

const HEADLESS_W: u32 = 1200;
const HEADLESS_H: u32 = 400;

#[cfg(feature = "gui")]
pub fn export_pdf(
    path: &Path,
    graph_image: &ColorImage,
    readings: &[GlucoseReading],
) -> Result<(), Box<dyn Error>> {
    let width = graph_image.size[0];
    let height = graph_image.size[1];
    let rgb_pixels: Vec<u8> = graph_image
        .pixels
        .iter()
        .flat_map(|c| {
            let [r, g, b, _a] = c.to_array();
            [r, g, b]
        })
        .collect();

    write_pdf_from_rgb(path, rgb_pixels, width, height, readings)
}

pub fn export_pdf_headless(
    path: &Path,
    readings: &[GlucoseReading],
) -> Result<(), Box<dyn Error>> {
    let rgb_pixels = render_chart_to_rgb(readings)?;
    write_pdf_from_rgb(
        path,
        rgb_pixels,
        HEADLESS_W as usize,
        HEADLESS_H as usize,
        readings,
    )
}

fn render_chart_to_rgb(readings: &[GlucoseReading]) -> Result<Vec<u8>, Box<dyn Error>> {
    // Register a system font for headless rendering (Linux/Windows)
    {
        let font_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];
        for path in &font_paths {
            if let Ok(data) = std::fs::read(path) {
                let _ = plotters::style::register_font(
                    "sans-serif",
                    plotters::style::FontStyle::Normal,
                    Box::leak(data.into_boxed_slice()),
                );
                break;
            }
        }
    }

    let mut rgb_buf = vec![0u8; (HEADLESS_W * HEADLESS_H * 3) as usize];

    {
        let root = BitMapBackend::with_buffer(&mut rgb_buf, (HEADLESS_W, HEADLESS_H))
            .into_drawing_area();
        root.fill(&WHITE)?;

        let mut sorted = readings.to_vec();
        sorted.sort_by_key(|r| r.recorded_at);

        if !sorted.is_empty() {
            let min_ts = sorted.first().unwrap().recorded_at.and_utc().timestamp() as f64;
            let max_ts = sorted.last().unwrap().recorded_at.and_utc().timestamp() as f64;
            let ts_range = (max_ts - min_ts).abs();
            let x_max = if ts_range < 1.0 { min_ts + 1.0 } else { max_ts };

            let min_val = sorted
                .iter()
                .map(|r| r.value)
                .fold(f64::INFINITY, f64::min)
                .min(3.0);
            let max_val = sorted
                .iter()
                .map(|r| r.value)
                .fold(f64::NEG_INFINITY, f64::max)
                .max(12.0);

            let mut chart = ChartBuilder::on(&root)
                .margin(20)
                .x_label_area_size(40)
                .y_label_area_size(60)
                .build_cartesian_2d(min_ts..x_max, min_val..max_val)?;

            chart
                .configure_mesh()
                .x_label_formatter(&|ts| {
                    let secs = *ts as i64;
                    chrono::DateTime::from_timestamp(secs, 0)
                        .map(|dt| dt.format("%m-%d").to_string())
                        .unwrap_or_default()
                })
                .y_desc("mmol/L")
                .draw()?;

            chart.draw_series(LineSeries::new(
                sorted.iter().map(|r| {
                    let ts = r.recorded_at.and_utc().timestamp() as f64;
                    (ts, r.value)
                }),
                &BLUE,
            ))?;

            // Threshold lines
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(min_ts, 4.0), (x_max, 4.0)],
                RGBColor(255, 165, 0),
            )))?;
            chart.draw_series(std::iter::once(PathElement::new(
                vec![(min_ts, 10.0), (x_max, 10.0)],
                RGBColor(255, 60, 60),
            )))?;
        }

        root.present()?;
    }

    Ok(rgb_buf)
}

fn write_pdf_from_rgb(
    path: &Path,
    rgb_pixels: Vec<u8>,
    width: usize,
    height: usize,
    readings: &[GlucoseReading],
) -> Result<(), Box<dyn Error>> {
    let mut doc = PdfDocument::new("Glucose Report");

    let raw_image = RawImage {
        pixels: RawImageData::U8(rgb_pixels),
        width,
        height,
        data_format: RawImageFormat::RGB8,
        tag: Vec::new(),
    };
    let image_id = doc.add_image(&raw_image);

    let avail_w_mm = PAGE_W - 2.0 * MARGIN;
    let aspect = height as f32 / width as f32;
    let img_h_mm = avail_w_mm * aspect;
    let dpi = width as f32 * 72.0 / (avail_w_mm * 2.834645669);
    let img_y_mm = PAGE_H - MARGIN - img_h_mm;

    let mut sorted_readings = readings.to_vec();
    sorted_readings.sort_by_key(|r| Reverse(r.recorded_at));

    let mut pages = Vec::new();
    let mut ops = Vec::new();

    ops.push(Op::UseXobject {
        id: image_id.clone(),
        transform: XObjectTransform {
            translate_x: Some(Pt(MARGIN * 2.834645669)),
            translate_y: Some(Pt(img_y_mm * 2.834645669)),
            dpi: Some(dpi),
            ..XObjectTransform::default()
        },
    });

    let table_start_y = img_y_mm - 10.0;
    let mut y = table_start_y;

    add_table_header(&mut ops, y);
    y -= ROW_HEIGHT + 1.0;

    for reading in &sorted_readings {
        if y < MARGIN {
            pages.push(PdfPage::new(Mm(PAGE_W), Mm(PAGE_H), std::mem::take(&mut ops)));
            y = PAGE_H - MARGIN;
            add_table_header(&mut ops, y);
            y -= ROW_HEIGHT + 1.0;
        }

        let date_str = reading.recorded_at.format("%Y-%m-%d").to_string();
        let time_str = reading.recorded_at.format("%H:%M").to_string();
        let value_str = format!("{:.1}", reading.value);

        add_text_row(&mut ops, y, &date_str, &time_str, &value_str);
        y -= ROW_HEIGHT;
    }

    pages.push(PdfPage::new(Mm(PAGE_W), Mm(PAGE_H), ops));

    let bytes = doc
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut Vec::new());

    std::fs::write(path, bytes)?;
    Ok(())
}

fn add_table_header(ops: &mut Vec<Op>, y: f32) {
    ops.push(Op::SaveGraphicsState);

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFontSizeBuiltinFont {
        font: BuiltinFont::HelveticaBold,
        size: Pt(FONT_SIZE_HEADER),
    });
    ops.push(Op::SetLineHeight { lh: Pt(FONT_SIZE_HEADER) });
    ops.push(Op::SetFillColor {
        col: Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }),
    });
    ops.push(Op::SetTextCursor { pos: Point::new(Mm(COL_DATE_X), Mm(y)) });
    ops.push(Op::WriteTextBuiltinFont {
        items: vec![TextItem::Text("Date".to_string())],
        font: BuiltinFont::HelveticaBold,
    });
    ops.push(Op::EndTextSection);

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFontSizeBuiltinFont {
        font: BuiltinFont::HelveticaBold,
        size: Pt(FONT_SIZE_HEADER),
    });
    ops.push(Op::SetTextCursor { pos: Point::new(Mm(COL_TIME_X), Mm(y)) });
    ops.push(Op::WriteTextBuiltinFont {
        items: vec![TextItem::Text("Time".to_string())],
        font: BuiltinFont::HelveticaBold,
    });
    ops.push(Op::EndTextSection);

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFontSizeBuiltinFont {
        font: BuiltinFont::HelveticaBold,
        size: Pt(FONT_SIZE_HEADER),
    });
    ops.push(Op::SetTextCursor { pos: Point::new(Mm(COL_VALUE_X), Mm(y)) });
    ops.push(Op::WriteTextBuiltinFont {
        items: vec![TextItem::Text("Value (mmol/L)".to_string())],
        font: BuiltinFont::HelveticaBold,
    });
    ops.push(Op::EndTextSection);

    let line_y = y - 1.0;
    ops.push(Op::SetOutlineColor {
        col: Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }),
    });
    ops.push(Op::SetOutlineThickness { pt: Pt(0.5) });
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![
                LinePoint {
                    p: Point::new(Mm(COL_DATE_X), Mm(line_y)),
                    bezier: false,
                },
                LinePoint {
                    p: Point::new(Mm(150.0), Mm(line_y)),
                    bezier: false,
                },
            ],
            is_closed: false,
        },
    });

    ops.push(Op::RestoreGraphicsState);
}

fn add_text_row(ops: &mut Vec<Op>, y: f32, date: &str, time: &str, value: &str) {
    ops.push(Op::SaveGraphicsState);

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFontSizeBuiltinFont {
        font: BuiltinFont::Helvetica,
        size: Pt(FONT_SIZE_ROW),
    });
    ops.push(Op::SetLineHeight { lh: Pt(FONT_SIZE_ROW) });
    ops.push(Op::SetFillColor {
        col: Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }),
    });
    ops.push(Op::SetTextCursor { pos: Point::new(Mm(COL_DATE_X), Mm(y)) });
    ops.push(Op::WriteTextBuiltinFont {
        items: vec![TextItem::Text(date.to_string())],
        font: BuiltinFont::Helvetica,
    });
    ops.push(Op::EndTextSection);

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFontSizeBuiltinFont {
        font: BuiltinFont::Helvetica,
        size: Pt(FONT_SIZE_ROW),
    });
    ops.push(Op::SetTextCursor { pos: Point::new(Mm(COL_TIME_X), Mm(y)) });
    ops.push(Op::WriteTextBuiltinFont {
        items: vec![TextItem::Text(time.to_string())],
        font: BuiltinFont::Helvetica,
    });
    ops.push(Op::EndTextSection);

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFontSizeBuiltinFont {
        font: BuiltinFont::Helvetica,
        size: Pt(FONT_SIZE_ROW),
    });
    ops.push(Op::SetTextCursor { pos: Point::new(Mm(COL_VALUE_X), Mm(y)) });
    ops.push(Op::WriteTextBuiltinFont {
        items: vec![TextItem::Text(value.to_string())],
        font: BuiltinFont::Helvetica,
    });
    ops.push(Op::EndTextSection);

    ops.push(Op::RestoreGraphicsState);
}
