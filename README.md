Stork Editor
[![Discord Badge]][discord]
=============

[Discord Badge]: https://img.shields.io/static/v1?message=Discord&logo=discord&labelColor=5c5c5c&color=7289DA&logoColor=white&label=%20
[discord]: https://discord.gg/Fy4za2WsT6

Stork is a ROM hacking tool for Yoshi's Island DS. YIDS has an immensely powerful engine, while having the same charming graphics and platforming as the original, and a level editor was begging to be made for it. I had created one previously, but it was written in C++ and Qt, and was developed before I fully understood the file structure of YIDS. Rust provides a dramatically more stable framework, and is somehow faster as well.

The game version used is **USA 1.0**, please rip legally. Support for other versions is in progress

## Features

- View and edit 100% of levels
- Export working ROMs
- Edit Collision, Tiles, Sprites, Paths, Triggers, and more
- Interconnect maps within a course via entrances and exits
- Ease tile creation with Brushes for drawing common items
- View helpful display engine information such as loaded palettes and tiles
- Helpful documentation and workflows

## Building

`cargo build --release`

If on Linux, and you want to [Cross](https://github.com/cross-rs/cross) compile to Windows:
```
cargo install cross --git https://github.com/cross-rs/cross
rustup target add x86_64-pc-windows-gnu
cross build --release --target x86_64-pc-windows-gnu
```

©YoshiDonoshi
