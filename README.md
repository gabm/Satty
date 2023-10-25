# Satty

Satty - a Screenshot Annotation Tool inspired by [Swappy](https://github.com/jtheoof/swappy) and [Flameshot](https://flameshot.org/).

## Install

You can get the official Arch Linux package from the AUR:

```
yay -S satty-bin
```

You can download a prebuilt binary for x86-64 on the [Satty Releases](https://github.com/gabm/satty/releases) page.


## Build from source

You first need to install the native dependencies of Satty (see below) and then run:

```
cargo build --release
```

## Dependencies 

Satty is based on GTK-4 and Adwaita.

### Ubuntu

- libgtk-4-1
- libadwaita-1-0

### Arch Linux

- pango 
- glib2
- cairo
- libadwaita
- gtk4
- gdk-pixbuf2


## Usage

Start by providing a filename or a screenshot via stdin and annotate using the available tools. Save to clipboard or file when finished. Tools and Interface have been kept simple.

![](assets/usage.gif)

### wlroots based compositors (Sway et. al.)

You can bind a key to the following command:

```
grim -g "$(slurp -o -r -c '#ff0000ff')" - | satty --filename - --fullscreen --output-filename ~/Pictures/Screenshots/satty-$(date '+%Y%m%d-%H:%M:%S').png
```


## License

The source code is released under the MPL-2.0 license.
