use clap::Parser;
use fitsio::hdu::FitsHdu;
use fitsio::headers::{ReadsKey, WritesKey};
use fitsio::images::{ImageDescription, ImageType};
use fitsio::FitsFile;
use fitsrs::hdu::header::Header;
use rayon::prelude::*;
use wcs::{ImgXY, LonLat, WCS};

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
    #[arg(long, default_value_t = 0.0)]
    ra: f64,
    /// Declination to centre cutout on.
    #[arg(long, allow_hyphen_values(true), default_value_t = 0.0)]
    dec: f64,
    /// Size of the cutout in degrees.
    #[arg(long)]
    size: f64,
    /// Name of the output file.
    #[arg(long, default_value = "output")]
    outfile: String,
    /// CSV table to read cutout positions from. Should contain three columns with name, right
    /// ascension and declination.
    #[arg(long, default_value = "")]
    sourcetable: String,
}

fn copy_key_if_exists<T: Default + PartialEq + ReadsKey + WritesKey>(
    key: &str,
    hdu: &FitsHdu,
    from_img: &mut FitsFile,
    to_img: &mut FitsFile,
) -> Result<(), Box<dyn std::error::Error>> {
    let val: T = hdu.read_key(from_img, key).unwrap_or_else(|_| T::default());
    if val != T::default() {
        hdu.write_key(to_img, key, val)?;
    }
    Ok(())
}

fn make_cutout(
    fitsimage: &String,
    wcs: &WCS,
    ra: &f64,
    dec: &f64,
    size: &f64,
    outfile: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut fptr = FitsFile::open(fitsimage)?;
    let hdu = fptr.primary_hdu().unwrap();

    let naxis1: i64 = hdu.read_key(&mut fptr, "NAXIS1").unwrap_or_else(|_| 0);
    let naxis2: i64 = hdu.read_key(&mut fptr, "NAXIS2").unwrap_or_else(|_| 0);

    let cdelt1: f64 = hdu.read_key(&mut fptr, "CDELT1").unwrap_or_else(|_| 0.0);
    let cdelt2: f64 = hdu.read_key(&mut fptr, "CDELT2").unwrap_or_else(|_| 0.0);

    if cdelt1 == 0.0 || cdelt2 == 0.0 {
        eprintln!("Error: One of CDELT1 or CDELT2 is zero. Please check the file.");
        std::process::exit(-1);
    }
    let coord = LonLat::new(ra.to_radians(), dec.to_radians());
    let coord_pix = wcs.proj_lonlat(&coord).unwrap();

    let coord_ref_pix = ImgXY::new(coord_pix.x(), coord_pix.y());
    let coord_ref = wcs.unproj_lonlat(&coord_ref_pix).unwrap();

    let x_pix = coord_pix.x().round() as i64;
    let y_pix = coord_pix.y().round() as i64;

    if x_pix < 0 || x_pix >= naxis1 || y_pix < 0 || y_pix >= naxis2 {
        println!("Source {} completely outside image, skipping!", outfile);
        return Ok(());
    }

    let mut imsize: i64 = (size / cdelt1.abs()).ceil() as i64;

    let mut lim_low_row = x_pix - imsize / 2;
    let mut lim_up_row = x_pix + imsize / 2 + 1;
    let mut lim_low_col = y_pix - imsize / 2;
    let mut lim_up_col = y_pix + imsize / 2 + 1;

    while (lim_up_row >= naxis2 || lim_up_col >= naxis1 || lim_low_row < 0 || lim_low_col < 0)
        && imsize > 2
    {
        imsize = (imsize as f64 / 2.0).floor() as i64;

        lim_low_row = x_pix - imsize / 2;
        lim_up_row = x_pix + imsize / 2 + 1;
        lim_low_col = y_pix - imsize / 2;
        lim_up_col = y_pix + imsize / 2 + 1;
    }

    // Not sure why, but sometimes it is off by one.
    if ((lim_up_row - lim_low_row) == imsize + 1) && ((lim_up_col - lim_low_col) == imsize + 1) {
        imsize = imsize + 1;
    }
    let rrange = lim_low_row as usize..lim_up_row as usize;
    let crange = lim_low_col as usize..lim_up_col as usize;

    let img_desc = ImageDescription {
        data_type: ImageType::Float,
        dimensions: &[imsize.try_into().unwrap(), imsize.try_into().unwrap()],
    };
    let mut fptr_new = FitsFile::create(&outfile)
        .with_custom_primary(&img_desc)
        .open()?;
    hdu.write_key(&mut fptr_new, "CRVAL1", coord_ref.lon().to_degrees() + cdelt1.abs() / 2.0)?;
    hdu.write_key(&mut fptr_new, "CRVAL2", coord_ref.lat().to_degrees())?;

    hdu.write_key(&mut fptr_new, "CRPIX1", (imsize as f64/2.0).ceil() as u64)?;
    hdu.write_key(&mut fptr_new, "CRPIX2", (imsize as f64/2.0).ceil() as u64)?;

    hdu.write_key(&mut fptr_new, "CDELT1", cdelt1)?;
    hdu.write_key(&mut fptr_new, "CDELT2", cdelt2)?;

    let ctype1: std::string::String = hdu
        .read_key(&mut fptr, "CTYPE1")
        .unwrap_or_else(|_| "".to_string());
    let ctype2: std::string::String = hdu
        .read_key(&mut fptr, "CTYPE2")
        .unwrap_or_else(|_| "".to_string());
    hdu.write_key(&mut fptr_new, "CTYPE1", ctype1)?;
    hdu.write_key(&mut fptr_new, "CTYPE2", ctype2)?;

    let ctype3: std::string::String = hdu.read_key(&mut fptr, "CTYPE3").unwrap_or("".to_string());
    if ctype3.len() > 0 {
        hdu.write_key(&mut fptr_new, "CTYPE3", ctype3.clone())?;
    }

    let ctype4: std::string::String = hdu.read_key(&mut fptr, "CTYPE4").unwrap_or("".to_string());
    if ctype4.len() > 0 {
        hdu.write_key(&mut fptr_new, "CTYPE4", ctype4.clone())?;
    }

    copy_key_if_exists::<String>("RADESYS", &hdu, &mut fptr, &mut fptr_new)?;
    copy_key_if_exists::<f64>("LONPOLE", &hdu, &mut fptr, &mut fptr_new)?;
    copy_key_if_exists::<f64>("LATPOLE", &hdu, &mut fptr, &mut fptr_new)?;

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
    assert!(cutout_flat.len() == (imsize as usize).pow(2));
    hdu.write_region(
        &mut fptr_new,
        &[&(0..imsize as usize), &(0..imsize as usize)],
        &cutout_flat,
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let f = File::open(&args.fitsimage).unwrap();
    let mut reader = BufReader::new(f);
    let header = Header::parse_header(&mut reader)?;
    let wcs = WCS::new(&header).unwrap();

    if args.sourcetable.len() > 0 {
        let temp_reader = File::open(args.sourcetable)?;
        let mut csv_rdr = csv::Reader::from_reader(temp_reader);
        let vals: Vec<Result<csv::StringRecord, csv::Error>> = csv_rdr.records().collect();
        println!("Found {} sources in catalogue", vals.len());
        vals.par_iter().for_each(|result| {
            let name = &result.as_ref().unwrap()[0];
            let ra: f64 = result.as_ref().unwrap()[1].parse().unwrap();
            let dec: f64 = result.as_ref().unwrap()[2].parse().unwrap();
            //println!("Making cutout for {}", name);
            let _ = make_cutout(
                &args.fitsimage,
                &wcs,
                &ra,
                &dec,
                &args.size,
                format!("{}.fits", name),
            );
        });
    } else {
        make_cutout(
            &args.fitsimage,
            &wcs,
            &args.ra,
            &args.dec,
            &args.size,
            format!("{}.fits", args.outfile),
        )?;
    }

    Ok(())
}
