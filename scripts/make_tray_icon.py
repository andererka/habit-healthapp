#!/usr/bin/env python3
"""Draw the menu bar icon.

A template image: macOS ignores the colour and uses only the alpha channel,
tinting the shape to match the menu bar (light, dark, or highlighted). So this
draws opaque black on transparency.

Drawn large and downsampled, which is the cheapest way to get clean antialiased
edges at the 22pt size macOS actually renders.

    python3 scripts/make_tray_icon.py
"""

from PIL import Image, ImageDraw

SCALE = 16
SIZE = 44  # 22pt at 2x, the retina menu bar size
BIG = SIZE * SCALE

BLACK = (0, 0, 0, 255)


def draw() -> Image.Image:
    image = Image.new("RGBA", (BIG, BIG), (0, 0, 0, 0))
    pen = ImageDraw.Draw(image)

    unit = BIG / 44.0
    stroke = int(4.4 * unit)
    centre = BIG / 2

    # A figure with its arms raised: the clearest silhouette for "get up and
    # stretch" at a size where anything more detailed turns to mush.
    head_r = 4.4 * unit
    head_y = 9.5 * unit
    pen.ellipse(
        [centre - head_r, head_y - head_r, centre + head_r, head_y + head_r],
        fill=BLACK,
    )

    shoulder_y = 17.5 * unit
    hip_y = 27.5 * unit
    pen.line([(centre, shoulder_y), (centre, hip_y)], fill=BLACK, width=stroke)

    # Arms up and out.
    for direction in (-1, 1):
        pen.line(
            [(centre, shoulder_y + 1.5 * unit), (centre + direction * 11 * unit, 12 * unit)],
            fill=BLACK,
            width=stroke,
        )

    # Legs apart.
    for direction in (-1, 1):
        pen.line(
            [(centre, hip_y), (centre + direction * 7.5 * unit, 37 * unit)],
            fill=BLACK,
            width=stroke,
        )

    # Round off the limb ends so the figure does not look chiselled.
    cap = stroke / 2
    for point in [
        (centre - 11 * unit, 12 * unit),
        (centre + 11 * unit, 12 * unit),
        (centre - 7.5 * unit, 37 * unit),
        (centre + 7.5 * unit, 37 * unit),
    ]:
        pen.ellipse(
            [point[0] - cap, point[1] - cap, point[0] + cap, point[1] + cap],
            fill=BLACK,
        )

    return image.resize((SIZE, SIZE), Image.LANCZOS)


if __name__ == "__main__":
    out = "src-tauri/icons/tray.png"
    draw().save(out)
    print(f"wrote {out}")
