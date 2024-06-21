# Fast FITS Cutter
Fast FITS Cutter uses the unsafe bindings to CFITSIO from `fitsio` to quickly make cutouts of FITS images by using the capability of reading only a small region instead of the entire image.

## Installation
To install this package simply clone the repository and run

```bash
cargo install --path .
```

## Usage

```
 Usage: ffc [OPTIONS] --size <SIZE> <FITSIMAGE> 
 Arguments:
  <FITSIMAGE>  Input image to make a cutout out of

Options:
      --ra <RA>                    Right ascension to centre cutout on [default: 0.0]
      --dec <DEC>                  Declination to centre cutout on [default: 0.0]
      --size <SIZE>                Size of the cutout in degrees
      --outfile <OUTFILE>          Name of the output file [default: output]
      --sourcetable <SOURCETABLE>  CSV table to read cutout positions from. Should contain three columns with name, right ascension and declination [default: ]
  -h, --help                       Print help
  -V, --version                    Print version
```
