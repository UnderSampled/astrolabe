#!/usr/bin/env python3
"""
Scan PNG textures to find those with partial transparency (alpha values between 1-254).
This helps determine if the game uses true partial transparency or just binary alpha.
"""

import os
import sys
from pathlib import Path

try:
    from PIL import Image
except ImportError:
    print("PIL not found. Install with: pip install Pillow")
    sys.exit(1)

def check_alpha(image_path):
    """
    Check alpha channel of an image.
    Returns: (has_alpha, has_partial, min_alpha, max_alpha, partial_count, total_alpha_pixels)
    """
    try:
        img = Image.open(image_path)
        if img.mode != 'RGBA':
            return (False, False, None, None, 0, 0)

        pixels = list(img.getdata())
        alpha_values = [p[3] for p in pixels]

        min_alpha = min(alpha_values)
        max_alpha = max(alpha_values)

        # Count pixels with partial transparency (not 0 or 255)
        partial_pixels = [a for a in alpha_values if 0 < a < 255]
        transparent_pixels = [a for a in alpha_values if a < 255]

        has_alpha = min_alpha < 255
        has_partial = len(partial_pixels) > 0

        return (has_alpha, has_partial, min_alpha, max_alpha, len(partial_pixels), len(transparent_pixels))
    except Exception as e:
        return (False, False, None, None, 0, 0)

def scan_directory(base_path):
    """Scan all PNG files in directory recursively."""
    base = Path(base_path)

    results = {
        'total': 0,
        'with_alpha': 0,
        'binary_only': 0,  # Only 0 and 255
        'partial': [],     # Has values between 1-254
    }

    png_files = list(base.rglob('*.png'))
    print(f"Scanning {len(png_files)} PNG files...")

    for png_path in png_files:
        results['total'] += 1
        has_alpha, has_partial, min_a, max_a, partial_count, trans_count = check_alpha(png_path)

        if has_alpha:
            results['with_alpha'] += 1
            if has_partial:
                results['partial'].append({
                    'path': str(png_path.relative_to(base)),
                    'min_alpha': min_a,
                    'max_alpha': max_a,
                    'partial_pixels': partial_count,
                    'transparent_pixels': trans_count
                })
            else:
                results['binary_only'] += 1

    return results

def main():
    texture_dir = sys.argv[1] if len(sys.argv) > 1 else "output/Gamedata/Textures"

    if not os.path.exists(texture_dir):
        print(f"Directory not found: {texture_dir}")
        sys.exit(1)

    print(f"Scanning textures in: {texture_dir}\n")
    results = scan_directory(texture_dir)

    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print(f"Total PNG files:              {results['total']}")
    print(f"Files with any transparency:  {results['with_alpha']}")
    print(f"Binary alpha only (0 or 255): {results['binary_only']}")
    print(f"Partial transparency:         {len(results['partial'])}")
    print()

    if results['partial']:
        print("=" * 60)
        print("FILES WITH PARTIAL TRANSPARENCY")
        print("=" * 60)
        for item in sorted(results['partial'], key=lambda x: -x['partial_pixels']):
            print(f"\n{item['path']}")
            print(f"  Alpha range: {item['min_alpha']} - {item['max_alpha']}")
            print(f"  Partial alpha pixels: {item['partial_pixels']}")
            print(f"  Total transparent pixels: {item['transparent_pixels']}")
    else:
        print("No textures with partial transparency found!")
        print("All transparency is binary (fully opaque or fully transparent).")
        print("This suggests the game likely uses alpha cutout, not alpha blending.")

if __name__ == "__main__":
    main()
