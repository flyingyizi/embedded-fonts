# Introduction

althrough [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) defaultly support mono fonts. but is it not enough for other language, e.g. chinese, kerea, japan and so on.

This project includes a small tool convert-bdf to split(optional) and convert the BDF file format into the internal data structure for text render by the [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) library. this work is based on  [embedded-graphic BDF utils](https://github.com/embedded-graphics/bdf).

convert-bdf result is rust code, the result can be merge to you rust crate to display text. usage please refer the examples.

# TTF (truetype font) conversion

## Overview
This is the conversion procedure for truetype fonts:

- 1.Find out a suitable point size (ftview)
- 2.Convert TTF to BDF (otf2bdf)
- 3.Convert BDF to a rust code (convert-bdf)

## TTF point size
A truetype font often does not look very well with every point size. You can use the unix command `ftview` from the freetype tool collection to check the font with different point sizes:
```shell
$ftview 8 <fontname>.ttf
```
Different point sizes can be accessed with cursor up and down. Often it is useful to turn off aliasing by pressing "a".

## TTF to BDF conversion
The tool `otf2bdf` can convert the truetype font into bitmap format (bdf). For a linux environment, `otf2bdf` should be available as software package.

This conversion is done for a specific point size:
```shell
$otf2bdf -p <pointsize> -r 75 -o <fontname>.bdf <fontname>.ttf
```
The result can be checked with font tools, e.g.  `fontforge`.

## BDF to rust-file conversion
Use the tool convert-bdf (part of this project) to create a rust-file:
```shell
convert-bdf   --range "中国欢迎China welcomes日本へようこそWelcome to Japan북한 환영Welcome North Korea"   wenquanyi_12pt.bdf
```

## Add font to a project

refer examples

# PCF (Portable Compiled Format) conversion
Fonts distributed in the .pcf or .pcf.gz file format are part of the X11 distribution. Steps are:

- 1.Convert PCF to BDF (pcf2bdf)
- 2.Convert BDF to the internal representation (`convert-bdf`)

pcf2bdf is often available as a software package on a linux system:

for example:

```shell

$sudo apt-get install xfonts-wqy

$apt-file list xfonts-wqy
xfonts-wqy: /etc/X11/fonts/misc/xfonts-wqy-1.alias
xfonts-wqy: /etc/fonts/conf.avail/85-xfonts-wqy-1.conf
xfonts-wqy: /usr/share/doc/xfonts-wqy/AUTHORS.gz
xfonts-wqy: /usr/share/doc/xfonts-wqy/LOGO.png
xfonts-wqy: /usr/share/doc/xfonts-wqy/README.gz
xfonts-wqy: /usr/share/doc/xfonts-wqy/changelog.Debian.gz
xfonts-wqy: /usr/share/doc/xfonts-wqy/copyright
xfonts-wqy: /usr/share/fonts/X11/misc/wenquanyi_10pt.pcf
xfonts-wqy: /usr/share/fonts/X11/misc/wenquanyi_11pt.pcf
xfonts-wqy: /usr/share/fonts/X11/misc/wenquanyi_12pt.pcf
xfonts-wqy: /usr/share/fonts/X11/misc/wenquanyi_13px.pcf
xfonts-wqy: /usr/share/fonts/X11/misc/wenquanyi_9pt.pcf

# uncompress , then get bdf file
$sudo apt-get install pcf2bdf
$pcf2bdf -v -o wenquanyi_9pt.bdf  wenquanyi_9pt.pcf 

$tools/convert-bdf --help
convert-bdf

USAGE:
    convert-bdf [OPTIONS] <BDF_FILE>

ARGS:
    <BDF_FILE>    BDF input

OPTIONS:
    -h, --help                       Print help information
    -o, --output <OUTPUT>            output rust embedded glyphs [default: ./]
        --range <RANGE>              list of characters,defaultly export all glyphs in the bdf. e.g
                                     --range "abc" means only export a,b and c code's glyphs

$convert-bdf  --range "中國zhongguo"  wenquanyi_12pt.bdf
output rust glyphs file :"./wenquanyi_12pt.rs"
```


See the section for truetype conversion for further handling of the BDF file.

## Create new BDF fonts
There are several tools available for the creation of new fonts:

[gbdfed](http://www.math.nmsu.edu/~mleisher/Software/gbdfed/).
[fontforge](http://fontforge.sourceforge.net/) Both tools can export to the BDF file format.

