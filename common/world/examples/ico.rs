use factor_world::ico::*;
use std::f64::consts::{FRAC_PI_2, PI};
use std::io;

const PALETTE: [[u8; 3]; 12] = [
    [31, 119, 180],  // rgb(31, 119, 180)
    [255, 127, 14],  // rgb(255, 127, 14)
    [44, 160, 44],   // rgb(44, 160, 44)
    [214, 39, 40],   // rgb(214, 39, 40)
    [148, 103, 189], // rgb(148, 103, 189)
    [140, 86, 75],   // rgb(140, 86, 75)
    [227, 119, 194], // rgb(227, 119, 194)
    [127, 127, 127], // rgb(127, 127, 127)
    [189, 189, 34],  // rgb(189, 189, 34)
    [23, 190, 207],  // rgb(23, 190, 207)
    [255, 0, 255],   // error, rgb(255, 0, 255)
    [0, 0, 0],       // grid, rgb(0, 0, 0)
];

const HEIGHT: usize = 512;
const LINE_THICKNESS: f64 = 0.0065;
const MATCH_X: bool = true;
const MATCH_Y: bool = true;

fn main() -> io::Result<()> {
    let mut image = [0u8; HEIGHT * HEIGHT * 2];
    for (y, row) in image.chunks_mut(HEIGHT * 2).enumerate() {
        let lat = (y as f64).mul_add(-PI / HEIGHT as f64, FRAC_PI_2);
        for (x, px) in row.iter_mut().enumerate() {
            let (mut region, coords) =
                region_offset_raw((x as f64).mul_add(PI / HEIGHT as f64, PI), lat);
            let matches_x = MATCH_X && (coords.x % 0.05).abs() < LINE_THICKNESS;
            let matches_y = MATCH_Y && (coords.y % 0.05).abs() < LINE_THICKNESS;
            if region < 10 && (matches_x || matches_y) {
                region = 11;
            }
            *px = region;
        }
    }
    let file = std::fs::File::create("map.png")?;
    let mut encoder = png::Encoder::new(file, HEIGHT as u32 * 2, HEIGHT as u32);
    encoder.set_palette(PALETTE.as_flattened());
    encoder.set_color(png::ColorType::Indexed);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image)?;
    writer.finish()?;
    Ok(())
}
