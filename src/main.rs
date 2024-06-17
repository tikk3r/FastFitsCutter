use clap::Parser;
use fitsio::images::{ImageDescription, ImageType};
use fitsio::FitsFile;
use fitsrs::fits::Fits;
use ndarray::{s, Array, ArrayD, Axis};
use wcs::{LonLat, WCS};

use std::fs::File;
use std::io::BufReader;

/// A Rust interface to summarise LOFAR H5parm calibration tables.
#[derive(Parser, Debug)]
#[command(name = "FITS cutter")]
#[command(author = "Frits Sweijen")]
#[command(version = "0.0.0")]
#[command(
    help_template = "{name} \nVersion: {version} \nAuthor: {author}\n{about-section} \n {usage-heading} {usage} \n {all-args} {tab}"
)]
// #[clap(author="Author Name", version, about="")]
struct Args {
    /// Input image to make a cutout out of.
    fitsimage: String,
    /// Right ascension to centre cutout on.
    #[arg(long)]
    ra: f64,
    /// Declination to centre cutout on.
    #[arg(long)]
    dec: f64,
    /// Size of the cutout in degrees.
    #[arg(long)]
    size: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let f = File::open(
        &args.fitsimage,
        //"../../Downloads/image_full_ampphase_di_m.NS_shift.int.facetRestored.rescaled.fits",
    )
    .unwrap();
    // Use BufReader here to avoid reading the entire file in memory.
    // We just want quick access to the header at this point.
    let mut reader = BufReader::new(f);
    let Fits { hdu } = Fits::from_reader(&mut reader).unwrap();
    let header = hdu.get_header();
    let wcs = WCS::new(&header).unwrap();
    let coord = LonLat::new(args.ra.to_radians(), args.dec.to_radians());
    let coord_pix = wcs.proj_lonlat(&coord).unwrap();
    println!(
        "Centring cutout on (x, y) = ({}, {})",
        coord_pix.x() as usize,
        coord_pix.y() as usize
    );

    let mut fptr = FitsFile::open(&args.fitsimage)?;
    let hdu = fptr.primary_hdu().unwrap();
    let cdelt1: f64 = hdu.read_key(&mut fptr, "CDELT1").unwrap();
    let imsize: usize = (args.size / cdelt1.abs()).ceil() as usize;
    println!("New image size: ({} x {})", imsize, imsize);
    let mut _data: ArrayD<f64> = hdu.read_image(&mut fptr)?;
    // Assume RA and DEC axes are always the last two axes.
    while _data.shape().len() > 2 {
        _data = _data.remove_axis(Axis(0));
    }
    let rrange = coord_pix.y() as usize + 1 - imsize / 2..coord_pix.y() as usize + imsize / 2 + 1;
    let crange = coord_pix.x() as usize + 1 - imsize / 2..coord_pix.x() as usize + imsize / 2 + 1;
    let _cutout = _data.slice(s![rrange, crange]);

    let img_desc = ImageDescription {
        data_type: ImageType::Float,
        dimensions: &[imsize, imsize],
    };
    let mut fptr_new = FitsFile::create("output.fits")
        .with_custom_primary(&img_desc)
        .open()?;
    hdu.write_key(&mut fptr_new, "CRVAL1", args.ra)?;
    hdu.write_key(&mut fptr_new, "CRVAL2", args.dec)?;

    hdu.write_key(&mut fptr_new, "CRPIX1", imsize as u64 / 2)?;
    hdu.write_key(&mut fptr_new, "CRPIX2", imsize as u64 / 2)?;

    hdu.write_key(&mut fptr_new, "CDELT1", cdelt1)?;
    let cdelt2: f64 = hdu.read_key(&mut fptr, "CDELT2").unwrap();
    hdu.write_key(&mut fptr_new, "CDELT2", cdelt2)?;

    let ctype1: std::string::String = hdu.read_key(&mut fptr, "CTYPE1").unwrap();
    hdu.write_key(&mut fptr_new, "CTYPE1", ctype1)?;
    let ctype2: std::string::String = hdu.read_key(&mut fptr, "CTYPE2").unwrap();
    hdu.write_key(&mut fptr_new, "CTYPE2", ctype2)?;

    let radesys: std::string::String = hdu.read_key(&mut fptr, "RADESYS").unwrap();
    hdu.write_key(&mut fptr_new, "RADESYS", radesys)?;
    let lonpole: std::string::String = hdu.read_key(&mut fptr, "LONPOLE").unwrap();
    hdu.write_key(&mut fptr_new, "LONPOLE", lonpole)?;
    let latpole: std::string::String = hdu.read_key(&mut fptr, "LATPOLE").unwrap();
    hdu.write_key(&mut fptr_new, "LATPOLE", latpole)?;

    hdu.write_key(&mut fptr_new, "NAXIS1", imsize as u64)?;
    hdu.write_key(&mut fptr_new, "NAXIS2", imsize as u64)?;

    let cutout_flat = Array::from_iter(_cutout.iter().cloned()).to_vec();
    hdu.write_region(&mut fptr_new, &[&(0..imsize), &(0..imsize)], &cutout_flat)?;
    Ok(())
}
