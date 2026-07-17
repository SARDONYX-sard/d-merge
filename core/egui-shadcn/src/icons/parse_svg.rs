//! Minimal SVG inner-XML parser for Lucide icon data.

/// Parse inner SVG XML (the content inside `<svg>...</svg>`) into icon elements.
pub fn parse_svg(svg_inner: &str) -> Vec<super::icon_element::IconElement> {
    let mut elements = Vec::new();

    // Split on '<' to find each tag
    for chunk in svg_inner.split('<') {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }

        // Take everything up to '>' or '/>'
        let tag_content = chunk.split('>').next().unwrap_or("");

        if let Some(elem) = parse_tag(tag_content) {
            elements.push(elem);
        }
    }

    elements
}

fn parse_tag(tag: &str) -> Option<super::icon_element::IconElement> {
    let tag = tag.trim().trim_end_matches('/');
    let tag_name = tag.split_ascii_whitespace().next()?;

    match tag_name {
        "path" => parse_path_tag(tag),
        "circle" => parse_circle_tag(tag),
        "rect" => parse_rect_tag(tag),
        "line" => parse_line_tag(tag),
        "polyline" => parse_polyline_tag(tag),
        "polygon" => parse_polygon_tag(tag),
        "ellipse" => parse_ellipse_tag(tag),
        _ => None,
    }
}

fn parse_path_tag(tag: &str) -> Option<super::icon_element::IconElement> {
    let d = extract_attr(tag, "d")?;
    let commands = super::parse_path::parse_path_data(&d);
    Some(super::icon_element::IconElement::Path(commands))
}

fn parse_circle_tag(tag: &str) -> Option<super::icon_element::IconElement> {
    let cx = extract_attr(tag, "cx").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let cy = extract_attr(tag, "cy").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let r = extract_attr(tag, "r").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    Some(super::icon_element::IconElement::Circle { cx, cy, r })
}

fn parse_rect_tag(tag: &str) -> Option<super::icon_element::IconElement> {
    let x = extract_attr(tag, "x").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let y = extract_attr(tag, "y").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let width = extract_attr(tag, "width").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let height = extract_attr(tag, "height").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let rx = extract_attr(tag, "rx").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    Some(super::icon_element::IconElement::Rect {
        x,
        y,
        width,
        height,
        rx,
    })
}

fn parse_line_tag(tag: &str) -> Option<super::icon_element::IconElement> {
    let x1 = extract_attr(tag, "x1").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let y1 = extract_attr(tag, "y1").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let x2 = extract_attr(tag, "x2").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let y2 = extract_attr(tag, "y2").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    Some(super::icon_element::IconElement::Line { x1, y1, x2, y2 })
}

fn parse_polyline_tag(tag: &str) -> Option<super::icon_element::IconElement> {
    let points = extract_attr(tag, "points")?;
    Some(super::icon_element::IconElement::Polyline(parse_points(&points)))
}

fn parse_polygon_tag(tag: &str) -> Option<super::icon_element::IconElement> {
    let points = extract_attr(tag, "points")?;
    Some(super::icon_element::IconElement::Polygon(parse_points(&points)))
}

fn parse_ellipse_tag(tag: &str) -> Option<super::icon_element::IconElement> {
    let cx = extract_attr(tag, "cx").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let cy = extract_attr(tag, "cy").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let rx = extract_attr(tag, "rx").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let ry = extract_attr(tag, "ry").and_then(|s| s.parse().ok()).unwrap_or(0.0);
    Some(super::icon_element::IconElement::Ellipse { cx, cy, rx, ry })
}

// ── Helpers ─────────────────────────────────────────────────────

/// Extract the value of `attr="value"` from a tag string.
fn extract_attr(tag: &str, attr_name: &str) -> Option<String> {
    // Look for `attr_name="` — we must match as a word boundary to
    // avoid e.g. "ry" matching inside "stroke-linejoin-ry".
    // We search for ` attr_name="` (space-prefixed) or the tag starts
    // with it after the tag name.
    let needle = format!(" {attr_name}=\"");
    let start = tag.find(&needle).map(|i| i + needle.len())?;
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Parse an SVG `points` attribute ("x1,y1 x2,y2 ...") into coordinate pairs.
fn parse_points(s: &str) -> Vec<(f32, f32)> {
    let nums: Vec<f32> = s
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    nums.chunks(2)
        .filter_map(|pair| {
            if pair.len() == 2 {
                Some((pair[0], pair[1]))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_path_element() {
        let elems = parse_svg(r#"<path d="M 10 20 L 30 40" />"#);
        assert_eq!(elems.len(), 1);
        assert!(matches!(elems[0], super::super::icon_element::IconElement::Path(_)));
    }

    #[test]
    fn parse_circle_element() {
        let elems = parse_svg(r#"<circle cx="12" cy="13" r="8" />"#);
        assert_eq!(elems.len(), 1);
        if let super::super::icon_element::IconElement::Circle { cx, cy, r } = &elems[0] {
            assert_eq!(*cx, 12.0);
            assert_eq!(*cy, 13.0);
            assert_eq!(*r, 8.0);
        } else {
            panic!("Expected Circle");
        }
    }

    #[test]
    fn parse_rect_element() {
        let elems = parse_svg(r#"<rect width="18" height="18" x="3" y="3" rx="2" />"#);
        assert_eq!(elems.len(), 1);
    }

    #[test]
    fn parse_mixed_elements() {
        let svg = r#"<circle cx="12" cy="13" r="8" /> <path d="M 0 0 L 1 1" /> <line x1="1" y1="2" x2="3" y2="4" />"#;
        let elems = parse_svg(svg);
        assert_eq!(elems.len(), 3);
    }

    #[test]
    fn parse_polyline_element() {
        let elems = parse_svg(r#"<polyline points="11 3 11 11 14 8 17 11 17 3" />"#);
        assert_eq!(elems.len(), 1);
        if let super::super::icon_element::IconElement::Polyline(pts) = &elems[0] {
            assert_eq!(pts.len(), 5);
        } else {
            panic!("Expected Polyline");
        }
    }

    #[test]
    fn parse_polygon_element() {
        let elems = parse_svg(r#"<polygon points="12 2 22 8.5 22 15.5 12 22 2 15.5 2 8.5 12 2" />"#);
        assert_eq!(elems.len(), 1);
        if let super::super::icon_element::IconElement::Polygon(pts) = &elems[0] {
            assert_eq!(pts.len(), 7);
        } else {
            panic!("Expected Polygon");
        }
    }

    #[test]
    fn parse_ellipse_element() {
        let elems = parse_svg(r#"<ellipse cx="12" cy="19" rx="9" ry="3" />"#);
        assert_eq!(elems.len(), 1);
    }
}
