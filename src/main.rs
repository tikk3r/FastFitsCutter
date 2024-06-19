use clap::Parser;
use fitsio::images::{ImageDescription, ImageType};
use fitsio::FitsFile;
use fitsrs::hdu::header::Header;
use fitsrs::fits::Fits;
use wcs::{LonLat, WCS};

use std::fs::File;
use std::io::BufReader;

/// Make a cutout of a FITS file.
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
    #[arg(long,allow_hyphen_values(true))]
    dec: f64,
    /// Size of the cutout in degrees.
    #[arg(long)]
    size: f64,
    /// Name of the output file.
    #[arg(long, default_value = "output.fits")]
    outfile: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let f = File::open(&args.fitsimage).unwrap();
    let mut reader = BufReader::new(f);
    let header = Header::parse_header(&mut reader)?;
    let wcs = WCS::new(&header).unwrap();
    let coord = LonLat::new(args.ra.to_radians(), args.dec.to_radians());
    let coord_pix = wcs.proj_lonlat(&coord).unwrap();
    println!(
        "Centring cutout on (x, y) = ({}, {})",
        coord_pix.x() as u64,
        coord_pix.y() as u64,
    );

    let mut fptr = FitsFile::open(&args.fitsimage)?;
    let hdu = fptr.primary_hdu().unwrap();
    let cdelt1: f64 = hdu.read_key(&mut fptr, "CDELT1").unwrap();
    let imsize: usize = (args.size / cdelt1.abs()).ceil() as usize;
    println!("New image size: ({} x {})", imsize, imsize);
    let rrange = coord_pix.y() as usize + 1 - imsize / 2..coord_pix.y() as usize + imsize / 2 + 1;
    let crange = coord_pix.x() as usize + 1 - imsize / 2..coord_pix.x() as usize + imsize / 2 + 1;
    let img_desc = ImageDescription {
        data_type: ImageType::Float,
        dimensions: &[imsize, imsize],
    };
    let mut fptr_new = FitsFile::create(args.outfile)
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

    let ctype3: std::string::String = hdu.read_key(&mut fptr, "CTYPE3").unwrap_or("".to_string());
    if ctype3.len() > 0 {
        hdu.write_key(&mut fptr_new, "CTYPE3", ctype3.clone())?;
    }

    let ctype4: std::string::String = hdu.read_key(&mut fptr, "CTYPE4").unwrap_or("".to_string());
    if ctype4.len() > 0 {
        hdu.write_key(&mut fptr_new, "CTYPE4", ctype4.clone())?;
    }

    let radesys: std::string::String = hdu.read_key(&mut fptr, "RADESYS").unwrap();
    hdu.write_key(&mut fptr_new, "RADESYS", radesys)?;
    let lonpole: f64 = hdu.read_key(&mut fptr, "LONPOLE").unwrap();
    hdu.write_key(&mut fptr_new, "LONPOLE", lonpole)?;
    let latpole: f64 = hdu.read_key(&mut fptr, "LATPOLE").unwrap();
    hdu.write_key(&mut fptr_new, "LATPOLE", latpole)?;

    let cutout_flat: Vec<f64>;
    if ctype3.len() > 0 {
        let zcoord = 0..1;
        if ctype4.len() > 0 {
            let wcoord = 0..1;
            cutout_flat = hdu.read_region(&mut fptr, &[&rrange, &crange, &zcoord, &wcoord])?;
        } else {
            cutout_flat = hdu.read_region(&mut fptr, &[&rrange, &crange, &zcoord])?;
        }
    } else {
        cutout_flat = hdu.read_region(&mut fptr, &[&rrange, &crange])?;
    }
    hdu.write_region(&mut fptr_new, &[&(0..imsize), &(0..imsize)], &cutout_flat)?;
    Ok(())
}
