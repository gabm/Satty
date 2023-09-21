# Satty

Satty - a Screenshot Annotation Tool inspired by [Swappy](https://github.com/jtheoof/swappy) and [Flameshot](https://flameshot.org/).

## Usage

Start by providing a filename or a screenshot via stdin and annotate using the available tools. Save to clipboard or file when finished. Tools and Interface have been kept simple.

![](assets/usage.gif)

### wlroots based compositors (Sway et. al.)

You can bind a key to the following command:

```
grim -g "$(slurp -o -c '#ff0000ff')" - | satty --filename - --fullscreen --output-filename ~/Pictures/Screenshots/satty-$(date '+%Y%m%d-%H:%M:%S').png
```


## License

The source code is released under the MPL-2.0 license.
