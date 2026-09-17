"""Small bounded SVG figures from validated observations only."""
import math
from xml.etree import ElementTree as ET
from common import require

NS = "http://www.w3.org/2000/svg"
ET.register_namespace("", NS)


def node(parent, name, **attrs):
    return ET.SubElement(parent, "{" + NS + "}" + name, {key.replace("_", "-"): str(value) for key, value in attrs.items()})


def label(parent, x, y, text, size=12):
    node(parent, "text", x=x, y=y, font_family="sans-serif", font_size=size, fill="#222222").text = str(text)


def canvas(title, height):
    svg = ET.Element("{" + NS + "}svg", {"viewBox": f"0 0 1100 {height}", "width": "1100", "height": str(height), "role": "img"})
    node(svg, "title").text = title
    node(svg, "rect", x=0, y=0, width=1100, height=height, fill="#ffffff")
    label(svg, 24, 28, title, 18)
    return svg


def save(path, svg):
    with path.open("xb") as stream:
        stream.write(ET.tostring(svg, encoding="utf-8", xml_declaration=True))


def rates(path, observations, title, goal=None):
    require(0 < len(observations) <= 128, "plot row bound")
    require(all(math.isfinite(value) and value > 0 for _, value in observations), "plot rates")
    maximum = max([value for _, value in observations] + ([goal] if goal else [])) * 1.12
    svg = canvas(title, 100 + 34 * len(observations))
    label(svg, 24, 51, "Host-observed tokens/s; excludes warmups. No GPU timing or qualification claim.")
    for index, (name, value) in enumerate(observations):
        y = 70 + index * 34
        label(svg, 24, y + 17, name if len(name) <= 30 else name[:27] + "...")
        node(svg, "rect", x=275, y=y, width=720 * value / maximum, height=23, fill="#257b75")
        label(svg, 280 + 720 * value / maximum, y + 17, f"{value:.3f}")
    if goal:
        x = 275 + 720 * goal / maximum
        node(svg, "line", x1=x, x2=x, y1=65, y2=75 + len(observations) * 34, stroke="#a64455", stroke_dasharray="4 4")
        label(svg, x - 90, 93 + len(observations) * 34, f"{goal:g} tok/s goal, not a measurement")
    save(path, svg)


def timeline(path, trace):
    events = trace["events"]
    lanes = sorted({(row["clock"], row["lane"]) for row in events})
    left = min(row["begin_ns"] - row["max_error_ns"] for row in events)
    right = max(row["end_ns"] + row["max_error_ns"] for row in events)
    require(right > left and len(lanes) <= 128 and len(events) <= 4096, "timeline bounds")
    svg = canvas("Recorded timeline (domains explicit; no inferred GPU schedule)", 105 + len(lanes) * 28)
    label(svg, 24, 50, "Only actual event endpoints; whiskers show declared clock uncertainty. Cross-domain overlap is not computed.")
    positions = {lane: index for index, lane in enumerate(lanes)}
    palette = ("#257b75", "#ac4b60", "#5965a5", "#78642b")
    for index, (clock, lane) in enumerate(lanes):
        label(svg, 16, 82 + 28 * index, (clock + ":" + lane)[:34], 10)
    for row in events:
        y = 68 + positions[(row["clock"], row["lane"])] * 28
        x = 280 + 790 * (row["begin_ns"] - left) / (right - left)
        width = 790 * (row["end_ns"] - row["begin_ns"]) / (right - left)
        rect = node(svg, "rect", x=x, y=y, width=max(width, 0.25), height=19, fill=palette[row["token"] % len(palette)])
        node(rect, "title").text = f"{row['label']} run={row['run']} token={row['token']} duration_ns={row['end_ns'] - row['begin_ns']:.3f} error_ns={row['max_error_ns']:.3f}"
        error = 790 * row["max_error_ns"] / (right - left)
        if error:
            for end in (x, x + width):
                node(svg, "line", x1=end-error, x2=end+error, y1=y+23, y2=y+23, stroke="#444444")
    label(svg, 280, 96 + len(lanes) * 28, f"Retained observed interval: {(right-left)/1e6:.6f} ms. Absolute private identifiers omitted.")
    save(path, svg)
