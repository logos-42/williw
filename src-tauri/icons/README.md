# Tauri App Icons

This directory contains application icons for different platforms.

## Required Icons

Please add the following icon files to this directory:

- `32x32.png` - 32x32 PNG icon
- `128x128.png` - 128x128 PNG icon
- `128x128@2x.png` - 256x256 PNG icon (Retina display)
- `icon.icns` - macOS icon
- `icon.ico` - Windows icon

## Creating Icons

You can use online tools like [favicon.io](https://favicon.io/) or 
[Canva](https://www.canva.com/) to create icons from a logo.

For macOS .icns files, you can use:
- [Image2Icon](https://www.img2icnsapp.com/)
- [iConvert Icons](https://iconverticons.com/online/)

For Windows .ico files, you can use:
- [ConvertICO](https://convertico.com/)
- [IcoFX](https://icofx.ro/)

## Placeholder

If you don't have custom icons yet, you can use the Tauri CLI to generate placeholder icons:

```bash
npx tauri icon path/to/your/logo.png
```

This will automatically generate all required icon formats from a single PNG file.
