# Fast FITS Cutter

## Installation
To install this package simply clone the repository and run

```bash
cargo install --path .
```

## Usage

```
 Usage: ffc [OPTIONS] --ra <RA> --dec <DEC> --size <SIZE> <FITSIMAGE> 
 Arguments:
  <FITSIMAGE>  Input image to make a cutout out of

Options:
      --ra <RA>            Right ascension to centre cutout on
      --dec <DEC>          Declination to centre cutout on
      --size <SIZE>        Size of the cutout in degrees
      --outfile <OUTFILE>  Size of the cutout in degrees [default: output.fits]
  -h, --help               Print help
  -V, --version            Print version

```
