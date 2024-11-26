# rePocket

Another reMarkable native application to ~~sync~~ download the latest articles from Pocket into a dedicated folder in the device.

## Current Features

* Downloads the latest 10 _articles_ since the previous request. Articles will include _some_ images!

* Archive articles in Pocket by moving them to the folder Pocket/Archive in the reMarkable

* Syncing is user triggered from the UI, (or the CLI). Any changes to the `Sync` folder will trigger syncing. Changes in this instance means adding as Favorite, or editing a tag, as simple as that!

* The tool can be built and run in a host to generate all the files necessary to create the folder and its contents and add them to the device via SSH

* The installation should be impervious to reMarkable updates

## Future Features and _Featured_ Bugs

For a full list visit registered [enhancements](https://github.com/sm13/rePocket/labels/enhancement)

## Installation

### Requirements

* A properly [configured SSH](https://remarkable.guide/guide/access/index.html) connection to the reMarkable, call it `remarkable`
* [Rust](https://www.rust-lang.org/learn/get-started) and Cargo in order to build from sources
* If you'd like to run the binary in the device, you'll need a cross-compiler toolchain. You can follow one of the guides from [here](https://remarkable.guide/devel/toolchains.html). Under macos, I ended up installing the toolchain provided [here](https://github.com/messense/homebrew-macos-cross-toolchains/) (mostly because I found the other resource _later_)

### Steps

- Clone the repository

- Build the binaries ([rePocketAuth](../README.md) and [rePocket](../README.md)) fom sources following the instructions for each of them

- Run the installation script from the repositrory root

```bash
./install.sh
```

The script will do the following:

- Launch `rePocketAuth` to authenticate _your_ build for the app with Pocket

- Connect via SSH to the reMarkable to create the neccesary file structure and copy the `rePocket` binary

```
# If will create the following folders, if necessary
#
# /home/root/.local/bin
# /home/root/.local/share/repocket
#
# It will copy rePocket to ~/.local/bin
# It will move the authentication file to ~/.local/share/repocket
```

## Thank yous! Credits, and the like

Although I set up to satisfy my curiosity and learn some rust in the process I couldn't have done this without leaning on the work of many others:

* [motemen's](https://github.com/motemen) [go-pocket](https://github.com/motemen/go-pocket) and [GliderGeek's](https://github.com/GliderGeek) [pocket2rm](https://github.com/GliderGeek/pocket2rm) which were always my go-to when I run into a wall

* https://remarkable.guide for their documentation and all efforts

* [canselcik/libremarkable](https://github.com/canselcik/libremarkable) helped me get cross-compilation working
