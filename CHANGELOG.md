# Changelog

## 0.6.0

Canvases should now use premultiplied alpha, and are converted to straight alpha during export.

## 0.5.2

Updated CLI help for the `--output` argument.

## 0.5.1

Added support for specifying a `.png` or `.json` file when using the `--output` argument
when running a Rig. Previously you could only specify a folder, which was inconvenient
if there was only one output Component.


## 0.5.0

Initial public release.

Linux versions currently have Sixel support disabled in MUSL builds, it is enabled in GNU builds.
