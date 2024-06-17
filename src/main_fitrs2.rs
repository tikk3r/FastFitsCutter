use clap::{command, Parser};

use ndarray::Axis;
use plotters::prelude::*;
use rustronomy_fits::prelude::*;
use std::path::Path;

use fitsrs;
use wcs::{LonLat,WCS};
use std::fs::File;
use std::io::{Cursor, Read};

const OUT_FILE_NAME: &str = "matshow.png";

/// A Rust interface to summarise LOFAR H5parm calibration tables.
#[derive(Parser, Debug)]
#[command(name = "Rust FITS plot demo")]
#[command(author = "Frits Sweijen")]
#[command(version = "0.0.0")]
#[command(
    help_template = "{name} \nVersion: {version} \nAuthor: {author}\n{about-section} \n {usage-heading} {usage} \n {all-args} {tab}"
)]

// #[clap(author="Author Name", version, about="")]
struct Args {
    /// FITS image to display
    fits: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let f = Fits::open(Path::new(Path::new(&args.fits)))?;
    let data = match f.get_hdu(0).unwrap().get_data() {
        Some(Extension::Image(img)) => img.as_f32_array()?,
        _ => panic!("Failed to read FITS image."),
    };

    let mut data_squeezed = data.clone();
    if data_squeezed.shape().len() > 2 {
        println!("More than 2 axes found.");
        while data_squeezed.shape().len() > 2 {
            let axis = data_squeezed.shape().len() - 1;
            data_squeezed = data_squeezed.remove_axis(Axis(axis));
        }
        println!("Extra axes stripped.");
    }

    let data = &data_squeezed;

    let mut f = File::open(
        "../../Downloads/image_full_ampphase_di_m.NS_shift.int.facetRestored.rescaled.fits",
        //"../../Downloads/tail.fits",
    )
    .unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    let mut reader = Cursor::new(&buf[..]);
    let fitsrs::fits::Fits { hdu } = fitsrs::fits::Fits::from_reader(&mut reader).unwrap();
    let header = hdu.get_header();
    let wcs = WCS::new(&header).unwrap();
    let coord = LonLat::new(216.7489976f64.to_radians(), 34.1999570f64.to_radians());
    let coord_pix = wcs.proj_lonlat(&coord).unwrap();
    println!("Centring cutout on (x, y) = ({}, {})", coord_pix.x(), coord_pix.y());
    let pix_x = coord_pix.x().floor() as u64;
    let pix_y = coord_pix.y().floor() as u64;

    //let data = hdu.get_data();
    //data[[pix_x-100..pix_x+100, pix_y-100..pix_y+100]];
    // We can't call .max() since floats don't implement Ord.
    //https://stackoverflow.com/questions/57813951/whats-the-fastest-way-of-finding-the-index-of-the-maximum-value-in-an-array
    let mut data_max: f32 = 0.0;
    for row in data.axis_iter(Axis(1)) {
        let (max_idx, max_val) =
            row.iter()
                .enumerate()
                .fold((0, row[0]), |(idx_max, val_max), (idx, val)| {
                    if &val_max > val {
                        (idx_max, val_max)
                    } else {
                        (idx, *val)
                    }
                });
        data_max = max_val;
    }

    let mut data_min: f32 = 0.0;
    for row in data.axis_iter(Axis(1)) {
        let (min_idx, min_val) =
            row.iter()
                .enumerate()
                .fold((0, row[0]), |(idx_min, val_min), (idx, val)| {
                    if &val_min < val {
                        (idx_min, val_min)
                    } else {
                        (idx, *val)
                    }
                });
        data_min = min_val;
    }

    let dyn_range = 10.0f32.powf(((data_max - data_min) / data_min).abs());
    println!("Data min: {}", data_min);
    println!("Data max: {}", data_max);
    println!("Dynamic range: {}", dyn_range);

    let naxis1 = data.shape()[0];
    let naxis2 = data.shape()[1];
    let root = BitMapBackend::new(OUT_FILE_NAME, (naxis1 as u32 - 1, naxis2 as u32 - 1))
        .into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        //.caption("Matshow Example", ("sans-serif", 80))
        //.margin(5)
        //.top_x_label_area_size(40)
        //.y_label_area_size(40)
        .build_cartesian_2d(0..naxis1 - 1, naxis2 - 1..0)?;
    //.build_cartesian_2d(0i32..15i32, 15i32..0i32)?;

    chart
        .configure_mesh()
        //.x_labels(15)
        //.y_labels(15)
        //.max_light_lines(4)
        //.x_label_offset(35)
        //.y_label_offset(25)
        //.disable_x_mesh()
        //.disable_y_mesh()
        //.label_style(("sans-serif", 20))
        .draw()?;

    chart.draw_series(
        (0..naxis1)
            //.rev()
            .flat_map(move |i| (0..naxis2).map(move |j| (i, j, data[[i, j]])))
            .map(|(i, j, d)| {
                Rectangle::new(
                    [(i, naxis2 - j), (i + 1, naxis2 - j + 1)],
                    HSLColor(
                        //240.0 / 360.0 - 240.0 / 360.0 * d as f64 * 1000.0,
                        240.0 / 360.0 - 240.0 / 360.0 * d as f64 * dyn_range as f64,
                        0.7,
                        0.1 + 0.4 * d as f64 * dyn_range as f64,
                        //0.1 + 0.4 * d as f64 * 1000.0,
                    )
                    .filled(),
                )
            }),
    )?;
    /*
    chart.draw_series(
        matrix
            .iter()
            .zip(0..)
            .flat_map(|(l, y)| l.iter().zip(0..).map(move |(v, x)| (x, y, v)))
            .map(|(x, y, v)| {
                Rectangle::new(
                    [(x, y), (x + 1, y + 1)],
                    HSLColor(
                        240.0 / 360.0 - 240.0 / 360.0 * (*v as f64 / 20.0),
                        0.7,
                        0.1 + 0.4 * *v as f64 / 20.0,
                    )
                    .filled(),
                )
            }),
    )?;
    */

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
    println!("Result has been saved to {}", OUT_FILE_NAME);
    Ok(())
}
