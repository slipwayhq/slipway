# Changelog

## 0.9.1

Allow `slipway serve . add-api-key` to be run without any arguments.

Don't serialize `allow` or `deny` properties in the `slipway_serve.json` file if they are empty.

## 0.9.0

Rename the `html` format option to `html_js`, and rename the `html_embed` format option to `html`.
This is to reflect that the (new) `html` option is pure HTML, where as the (new) `html_js` option
will ultimately have to use Javascript to prevent flickering when refreshing the image (not yet implemented).

## 0.8.0

Significant refactor of API keys and how we associate them with TRMNL devices. This also allows us to
properly support the TRMNL Redirect plugin.

BREAKING CHANGE:

In your `slipway_serve.json` API Keys are now stored as a list, rather than a map.
The property is now called `api_keys` instead of `hashed_api_keys`.
If your JSON looked like this:

```json
  "hashed_api_keys": {
    "kitchen_device": "hashed_key_1"
  }
```

It should now look like this:

```json
  "api_keys": [
    {
      "hashed_key": "hashed_key_1",
      "description": "kitchen device"
    }
  ]
```

In your devices, the `trmnl` property has been removed.
We no longer store the hashed ID of the device.
Any hashed API keys stored in the `trmnl` property should be moved to the `api_keys` property.

For example, if you had a device `bedroom_trmnl.json` which contained:
```json
  "trmnl": {
    "hashed_key_2": "hashed_key_2"
  }
```

Remove the `trmnl` property and add `hashed_key_2` to your `api_keys` list associated with the device:

```json
  "api_keys": [
    {
      "hashed_key": "hashed_key_2",
      "device": "bedroom_trmnl"
    }
  ]
```


## 0.7.1

Rigs can now specify a default device context, which will be overridden by devices.

Image rotation can now be specified as a `rotate` query string parameter.

Devices can now specify default values for `format`, `image_format` and `rotate`.
These defaults can be overridden by query string parameters.

Roboto and Roboto Mono fonts are now bundled and used by default for `sans-serif` and `monospace` font families.


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
