#!/usr/bin/env python3
"""Export a FigCraft artboard to SVG for inspection, without altering the source.

Usage: export_figcraft.py document.figcraft ARTBOARD_ID output.svg
Text shaping is delegated to the SVG renderer, so use the archive thumbnail for
native typography comparison. Supports the rectangle/text/component node types
used by the Keyforge v0.3 mocks.
"""
import html
import json
import sys
import zipfile
from pathlib import Path


def export(source, artboard, destination):
    with zipfile.ZipFile(source) as archive:
        nodes = json.loads(archive.read("document.json"))["nodes"]
    root = nodes[str(artboard)]

    def paint(layers):
        for layer in layers:
            if layer.get("visible", True) and layer.get("type") == "solid":
                color = layer["color"]
                return "#{:02x}{:02x}{:02x}".format(*(round(color[k] * 255) for k in "rgb"))
        return "none"

    def node(identifier, top=False):
        value = nodes[str(identifier)]
        if value.get("visible") is False:
            return ""
        transform = value.get("transform", [1, 0, 0, 1, 0, 0]) if not top else [1, 0, 0, 1, 0, 0]
        result = [f'<g transform="matrix({" ".join(map(str, transform))})">']
        fill = paint(value.get("fills", []))
        width, height = value.get("width", 0), value.get("height", 0)
        if value["type"] == "text":
            style = value.get("style", {})
            size = style.get("font_size", 14)
            weight = "600" if "Bold" in style.get("font_style", "") else "500" if "Medium" in style.get("font_style", "") else "400"
            align = value.get("align_h", "left")
            x, anchor = (width / 2, "middle") if align == "center" else (width, "end") if align == "right" else (0, "start")
            y = height / 2 + size * .35 if value.get("align_v") == "middle" else size
            family = html.escape(style.get("font_family", "Inter"))
            text = html.escape(value.get("characters", ""))
            result.append(f'<text x="{x}" y="{y}" fill="{fill}" font-family="{family}, sans-serif" font-size="{size}" font-weight="{weight}" text-anchor="{anchor}">{text}</text>')
        else:
            radius = value.get("corners", {}).get("radii", [0])[0]
            stroke = paint(value.get("strokes", []))
            weight = value.get("stroke", {}).get("weight", 1)
            result.append(f'<rect width="{width}" height="{height}" rx="{radius}" fill="{fill}" stroke="{stroke}" stroke-width="{weight}"/>')
        result.extend(node(child) for child in value.get("children", []))
        result.append("</g>")
        return "".join(result)

    Path(destination).write_text(f'<svg xmlns="http://www.w3.org/2000/svg" width="{root["width"]}" height="{root["height"]}" viewBox="0 0 {root["width"]} {root["height"]}">{node(artboard, True)}</svg>')


if __name__ == "__main__":
    export(sys.argv[1], int(sys.argv[2]), sys.argv[3])
