# Astrolabe

A toolkit for extracting and converting game data from **Hype: The Time Quest** (1999) into Godot-compatible formats.

## About

Hype: The Time Quest was developed by Ubisoft (as Playmobil Interactive) for Brandstätter Group using the OpenSpace engine (shared with Tonic Trouble and Rayman 2). This project provides tools to:

1. Extract assets from the original game ISO
2. Convert 3D models to glTF format
3. Generate Godot scene files for maps
4. Capture state graphs and interactions as GDScript

## Legal Notice

This project does not include any copyrighted game assets. Users must provide their own legally obtained copy of the original game ISO. This approach follows the precedent set by projects like OpenMW and Freespace Open.

## Requirements

- dotnet (TODO: clarify)
- A legally obtained copy of Hype: The Time Quest (ISO format)

## Usage

TODO

## Project Structure

```
astrolabe/
├── src/
│   └── astrolabe/
├── reference/
│   └── raymap/              # Raymap Unity project (git submodule)
├── tests/
└── docs/
```

## Acknowledgments

This project heavily references [Raymap](https://github.com/byvar/raymap) by the Rayman community, which provides invaluable documentation of the OpenSpace engine formats.

## License

MIT License - See LICENSE file for details.
