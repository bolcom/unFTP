# Developers Guide

Run `makepkg` on an ArchLinux machine

To see contents of the package:

```sh
pacman -Qpl unftp-0.14.2-1-x86_64.pkg.tar.zst
```

To install locally:

```sh
sudo pacman -U unftp-0.14.2-1-x86_64.pkg.tar.zst
```

To get package info:

`pacman -Qs unftp`