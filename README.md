# <img src="assets/satty.svg" height="42"> Satty: Modern Screenshot Annotation. 

Satty is a screenshot annotation tool inspired by [Swappy](https://github.com/jtheoof/swappy) and [Flameshot](https://flameshot.org/).

![](assets/usage.gif)

Satty has been created to provide the following improvements over existing screenshot annotation tools:

- very simple and easy to understand toolset (like Swappy)
- fullscreen annotation mode and post shot cropping (like Flameshot)
- working on wlroots based compositors (Sway, Hyprland, River, ...)
- minimal, modern looking UI, thanks to GTK and Adwaita
- be a playground for new features (post window selection, post paint editing, ...)

## Install

### Arch Linux

You can get the official Arch Linux package from the AUR:

```
yay -S satty-bin
```

### Gentoo

You can get the Gentoo package from the Guru overlay:

```
eselect repository enable guru
emerge --sync guru
emerge -av satty
```

Pending PR for Gentoo overlay: https://github.com/gentoo/gentoo/pull/33908

### Alpine Linux

Satty is available in [Alpine Testing](https://pkgs.alpinelinux.org/packages?name=satty&branch=edge&repo=&arch=&maintainer=). You can install it by uncommenting the testing repo in `/etc/apk/repositories` and then run:

```
apk add satty
```

### Prebuilt Sources

You can download a prebuilt binary for x86-64 on the [Satty Releases](https://github.com/gabm/satty/releases) page.


## Usage

Start by providing a filename or a screenshot via stdin and annotate using the available tools. Save to clipboard or file when finished. Tools and Interface have been kept simple.

All configuration is done via the command line interface:

```sh
» satty --help                
A screenshot annotation tool inspired by Swappy and Flameshot.

Usage: satty [OPTIONS] --filename <FILENAME>

Options:
  -f, --filename <FILENAME>
          Path to input image or '-' to read from stdin
      --fullscreen
          Start Satty in fullscreen mode
      --output-filename <OUTPUT_FILENAME>
          Filename to use for saving action, omit to disable saving to file
      --early-exit
          Exit directly after copy/save action
      --init-tool <TOOL>
          Select the tool on startup [default: pointer] [possible values: pointer, crop, line, arrow, rectangle, text, marker, blur, brush]
      --copy-command <COPY_COMMAND>
          Configure the command to be called on copy, for example `wl-copy`
  -h, --help
          Print help
  -V, --version
          Print version
```

### wlroots based compositors (Sway, Hyprland, Wayfire, River, ...)

You can bind a key to the following command:

```
grim -g "$(slurp -o -r -c '#ff0000ff')" - | satty --filename - --fullscreen --output-filename ~/Pictures/Screenshots/satty-$(date '+%Y%m%d-%H:%M:%S').png
```


## Build from source

You first need to install the native dependencies of Satty (see below) and then run:

```sh
# build release binary, located in ./target/release/satty
make

# optional: install to /usr/local
PREFIX=/use/local make install

# optional: uninstall from /usr/local
PREFIX=/use/local make uninstall
```

## Dependencies 

Satty is based on GTK-4 and Adwaita.

### Ubuntu

- libgtk-4-1
- libadwaita-1-0

### Arch Linux & Gentoo

- pango 
- glib2
- cairo
- libadwaita
- gtk4
- gdk-pixbuf2

## License

The source code is released under the MPL-2.0 license.
