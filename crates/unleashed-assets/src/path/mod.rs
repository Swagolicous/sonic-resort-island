use std::error::Error;
use std::fmt;
use std::str;

#[derive(Debug, Clone, PartialEq)]
pub struct PathDocument {
    pub geometries: Vec<PathGeometry>,
    pub nodes: Vec<PathNode>,
}

impl PathDocument {
    pub fn geometry(&self, id: &str) -> Option<&PathGeometry> {
        self.geometries.iter().find(|geometry| geometry.id == id)
    }

    pub fn node(&self, id: &str) -> Option<&PathNode> {
        self.nodes.iter().find(|node| node.id == id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathGeometry {
    pub id: String,
    pub name: String,
    /// Width declared on the SonicPath `<spline>` container.
    ///
    /// The inspected Mykonos and SubNY resources all declare `0.2`; no runtime
    /// semantics beyond the XML field itself are assumed here.
    pub width: f32,
    /// One entry for each `<spline3d>` child.
    pub splines: Vec<PathSpline>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathSpline {
    pub knots: Vec<PathKnot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnotType {
    Bezier,
    Corner,
    BezierCorner,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathKnot {
    pub kind: KnotType,
    pub in_vec: [f32; 3],
    pub out_vec: [f32; 3],
    pub point: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathNode {
    pub id: String,
    pub name: String,
    pub translate: [f32; 3],
    pub scale: [f32; 3],
    /// Quaternion as serialized by the source XML.
    pub rotate: [f32; 4],
    /// Verbatim `instance/@url`, including its leading `#`.
    pub instance_url: String,
}

impl PathNode {
    /// Returns the geometry id referenced by this node, without the XML `#`
    /// fragment marker.
    pub fn geometry_id(&self) -> &str {
        self.instance_url
            .strip_prefix('#')
            .unwrap_or(&self.instance_url)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathParseError {
    InvalidUtf8,
    MissingRoot,
    MissingGeometryLibrary,
    MissingScene,
    MissingTag(&'static str),
    MissingAttribute {
        tag: &'static str,
        attribute: &'static str,
    },
    InvalidNumber {
        field: &'static str,
        value: String,
    },
    InvalidVector {
        field: &'static str,
        value: String,
    },
    UnsupportedKnotType(String),
    CountMismatch {
        field: &'static str,
        declared: usize,
        actual: usize,
    },
    MalformedXml(&'static str),
}

impl fmt::Display for PathParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 => write!(f, "path XML is not valid UTF-8"),
            Self::MissingRoot => write!(f, "missing SonicPath root"),
            Self::MissingGeometryLibrary => write!(f, "missing GEOMETRY library"),
            Self::MissingScene => write!(f, "missing scene"),
            Self::MissingTag(tag) => write!(f, "missing <{tag}> tag"),
            Self::MissingAttribute { tag, attribute } => {
                write!(f, "missing {attribute} attribute on <{tag}>")
            }
            Self::InvalidNumber { field, value } => {
                write!(f, "invalid numeric value for {field}: {value:?}")
            }
            Self::InvalidVector { field, value } => {
                write!(f, "invalid vector value for {field}: {value:?}")
            }
            Self::UnsupportedKnotType(kind) => write!(f, "unsupported knot type {kind:?}"),
            Self::CountMismatch {
                field,
                declared,
                actual,
            } => write!(
                f,
                "{field} count mismatch: XML declares {declared}, parsed {actual}"
            ),
            Self::MalformedXml(message) => write!(f, "malformed SonicPath XML: {message}"),
        }
    }
}

impl Error for PathParseError {}

/// Parses the narrow SonicPath XML schema evidenced by Sonic Unleashed stage
/// path resources such as `myk_path_act01.path.xml`,
/// `StageGuidePath.path.xml`, and `ny_sub_path.path.xml`.
///
/// This intentionally is not a general XML or spline engine. It extracts only
/// the fields needed by later rail / 2D / camera-guidance work:
///
/// * geometry ids and names;
/// * `<spline>` width and declared child count;
/// * `<spline3d>` knots with their type, in/out handles, and control point;
/// * scene-node ids/names, transforms, and geometry instance link.
///
/// The inspected files contain no explicit flags field. Suffixes such as
/// `@SV`, `@GR`, and `@QS` are preserved verbatim in ids/names rather than
/// being promoted to guessed flag semantics.
pub fn parse_path_xml(bytes: &[u8]) -> Result<PathDocument, PathParseError> {
    let text = str::from_utf8(bytes).map_err(|_| PathParseError::InvalidUtf8)?;
    if !text.contains("<SonicPath>") || !text.contains("</SonicPath>") {
        return Err(PathParseError::MissingRoot);
    }

    let library = find_geometry_library(text)?;
    let geometries = parse_geometries(library)?;

    let scene = first_element(text, "scene")?.ok_or(PathParseError::MissingScene)?;
    let nodes = parse_nodes(scene.body)?;

    Ok(PathDocument { geometries, nodes })
}

#[derive(Debug, Clone, Copy)]
struct Element<'a> {
    opening: &'a str,
    body: &'a str,
    end: usize,
}

fn find_geometry_library(text: &str) -> Result<&str, PathParseError> {
    let mut cursor = 0;
    while let Some(element) = next_element(text, "library", cursor)? {
        if attribute(element.opening, "type") == Some("GEOMETRY") {
            return Ok(element.body);
        }
        cursor = element.end;
    }
    Err(PathParseError::MissingGeometryLibrary)
}

fn parse_geometries(text: &str) -> Result<Vec<PathGeometry>, PathParseError> {
    let mut geometries = Vec::new();
    let mut cursor = 0;

    while let Some(geometry) = next_element(text, "geometry", cursor)? {
        let id = required_attribute(geometry.opening, "geometry", "id")?.to_owned();
        let name = required_attribute(geometry.opening, "geometry", "name")?.to_owned();

        let spline =
            first_element(geometry.body, "spline")?.ok_or(PathParseError::MissingTag("spline"))?;
        let declared_splines = parse_usize_attribute(spline.opening, "spline", "count")?;
        let width = parse_f32_attribute(spline.opening, "spline", "width")?;

        let mut splines = Vec::new();
        let mut spline_cursor = 0;
        while let Some(spline3d) = next_element(spline.body, "spline3d", spline_cursor)? {
            let declared_knots = parse_usize_attribute(spline3d.opening, "spline3d", "count")?;
            let knots = parse_knots(spline3d.body)?;
            if knots.len() != declared_knots {
                return Err(PathParseError::CountMismatch {
                    field: "spline3d/knot",
                    declared: declared_knots,
                    actual: knots.len(),
                });
            }
            splines.push(PathSpline { knots });
            spline_cursor = spline3d.end;
        }

        if splines.len() != declared_splines {
            return Err(PathParseError::CountMismatch {
                field: "spline/spline3d",
                declared: declared_splines,
                actual: splines.len(),
            });
        }

        geometries.push(PathGeometry {
            id,
            name,
            width,
            splines,
        });
        cursor = geometry.end;
    }

    if geometries.is_empty() {
        return Err(PathParseError::MissingTag("geometry"));
    }

    Ok(geometries)
}

fn parse_knots(text: &str) -> Result<Vec<PathKnot>, PathParseError> {
    let mut knots = Vec::new();
    let mut cursor = 0;

    while let Some(knot) = next_element(text, "knot", cursor)? {
        let kind = match required_attribute(knot.opening, "knot", "type")? {
            "bezier" => KnotType::Bezier,
            "corner" => KnotType::Corner,
            "bezier_corner" => KnotType::BezierCorner,
            other => return Err(PathParseError::UnsupportedKnotType(other.to_owned())),
        };

        let in_vec = parse_vec3_element(knot.body, "invec")?;
        let out_vec = parse_vec3_element(knot.body, "outvec")?;
        let point = parse_vec3_element(knot.body, "point")?;

        knots.push(PathKnot {
            kind,
            in_vec,
            out_vec,
            point,
        });
        cursor = knot.end;
    }

    Ok(knots)
}

fn parse_nodes(text: &str) -> Result<Vec<PathNode>, PathParseError> {
    let mut nodes = Vec::new();
    let mut cursor = 0;

    while let Some(node) = next_element(text, "node", cursor)? {
        let id = required_attribute(node.opening, "node", "id")?.to_owned();
        let name = required_attribute(node.opening, "node", "name")?.to_owned();
        let translate = parse_vec3_element(node.body, "translate")?;
        let scale = parse_vec3_element(node.body, "scale")?;
        let rotate = parse_vec4_element(node.body, "rotate")?;

        let instance_open = find_opening_tag(node.body, "instance")
            .ok_or(PathParseError::MissingTag("instance"))?;
        let instance_url = required_attribute(instance_open, "instance", "url")?.to_owned();

        nodes.push(PathNode {
            id,
            name,
            translate,
            scale,
            rotate,
            instance_url,
        });
        cursor = node.end;
    }

    Ok(nodes)
}

fn parse_vec3_element(text: &str, tag: &'static str) -> Result<[f32; 3], PathParseError> {
    let element = first_element(text, tag)?.ok_or(PathParseError::MissingTag(tag))?;
    parse_vec3(element.body.trim(), tag)
}

fn parse_vec4_element(text: &str, tag: &'static str) -> Result<[f32; 4], PathParseError> {
    let element = first_element(text, tag)?.ok_or(PathParseError::MissingTag(tag))?;
    let value = element.body.trim();
    let parts: Vec<_> = value.split_whitespace().collect();
    if parts.len() != 4 {
        return Err(PathParseError::InvalidVector {
            field: tag,
            value: value.to_owned(),
        });
    }

    let mut out = [0.0; 4];
    for (index, part) in parts.iter().enumerate() {
        out[index] = parse_f32(part, tag)?;
    }
    Ok(out)
}

fn parse_vec3(value: &str, field: &'static str) -> Result<[f32; 3], PathParseError> {
    let parts: Vec<_> = value.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(PathParseError::InvalidVector {
            field,
            value: value.to_owned(),
        });
    }

    Ok([
        parse_f32(parts[0], field)?,
        parse_f32(parts[1], field)?,
        parse_f32(parts[2], field)?,
    ])
}

fn parse_f32(value: &str, field: &'static str) -> Result<f32, PathParseError> {
    let parsed = value
        .parse::<f32>()
        .map_err(|_| PathParseError::InvalidNumber {
            field,
            value: value.to_owned(),
        })?;
    if !parsed.is_finite() {
        return Err(PathParseError::InvalidNumber {
            field,
            value: value.to_owned(),
        });
    }
    Ok(parsed)
}

fn parse_usize_attribute(
    opening: &str,
    tag: &'static str,
    name: &'static str,
) -> Result<usize, PathParseError> {
    let value = required_attribute(opening, tag, name)?;
    value
        .parse::<usize>()
        .map_err(|_| PathParseError::InvalidNumber {
            field: name,
            value: value.to_owned(),
        })
}

fn parse_f32_attribute(
    opening: &str,
    tag: &'static str,
    name: &'static str,
) -> Result<f32, PathParseError> {
    parse_f32(required_attribute(opening, tag, name)?, name)
}

fn required_attribute<'a>(
    opening: &'a str,
    tag: &'static str,
    name: &'static str,
) -> Result<&'a str, PathParseError> {
    attribute(opening, name).ok_or(PathParseError::MissingAttribute {
        tag,
        attribute: name,
    })
}

fn attribute<'a>(opening: &'a str, name: &str) -> Option<&'a str> {
    let bytes = opening.as_bytes();
    let name_bytes = name.as_bytes();
    let mut cursor = 0;

    while cursor + name_bytes.len() < bytes.len() {
        let relative = opening[cursor..].find(name)?;
        let start = cursor + relative;
        let end_name = start + name_bytes.len();

        let valid_left =
            start == 0 || bytes[start - 1].is_ascii_whitespace() || bytes[start - 1] == b'<';
        if !valid_left {
            cursor = end_name;
            continue;
        }

        let mut pos = end_name;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if bytes.get(pos) != Some(&b'=') {
            cursor = end_name;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if bytes.get(pos) != Some(&b'"') {
            return None;
        }
        pos += 1;
        let value_start = pos;
        let relative_end = opening[value_start..].find('"')?;
        return Some(&opening[value_start..value_start + relative_end]);
    }

    None
}

fn first_element<'a>(
    text: &'a str,
    tag: &'static str,
) -> Result<Option<Element<'a>>, PathParseError> {
    next_element(text, tag, 0)
}

fn next_element<'a>(
    text: &'a str,
    tag: &'static str,
    from: usize,
) -> Result<Option<Element<'a>>, PathParseError> {
    let needle = format!("<{tag}");
    let Some(relative_start) = text[from..].find(&needle) else {
        return Ok(None);
    };
    let start = from + relative_start;

    let name_end = start + needle.len();
    let delimiter = text.as_bytes().get(name_end).copied();
    if !matches!(
        delimiter,
        Some(b'>') | Some(b'/') | Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')
    ) {
        return next_element(text, tag, name_end);
    }

    let relative_open_end = text[start..]
        .find('>')
        .ok_or(PathParseError::MalformedXml("unterminated opening tag"))?;
    let open_end = start + relative_open_end + 1;
    let opening = &text[start..open_end];

    if opening.trim_end().ends_with("/>") {
        return Err(PathParseError::MalformedXml(
            "self-closing tag used where an element body is required",
        ));
    }

    let close = format!("</{tag}>");
    let relative_close = text[open_end..]
        .find(&close)
        .ok_or(PathParseError::MalformedXml("missing closing tag"))?;
    let close_start = open_end + relative_close;
    let end = close_start + close.len();

    Ok(Some(Element {
        opening,
        body: &text[open_end..close_start],
        end,
    }))
}

fn find_opening_tag<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let needle = format!("<{tag}");
    let mut from = 0;
    loop {
        let relative = text[from..].find(&needle)?;
        let start = from + relative;
        let name_end = start + needle.len();
        let delimiter = text.as_bytes().get(name_end).copied();
        if matches!(
            delimiter,
            Some(b'>') | Some(b'/') | Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')
        ) {
            let relative_end = text[start..].find('>')?;
            return Some(&text[start..start + relative_end + 1]);
        }
        from = name_end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<SonicPath>
  <library type="GEOMETRY">
    <geometry id="test@QS-geometry" name="test@QS-geometry">
      <spline count="2" width="0.2">
        <spline3d count="2">
          <knot type="corner">
            <invec>0 1 2</invec>
            <outvec>3 4 5</outvec>
            <point>6 7 8</point>
          </knot>
          <knot type="bezier_corner">
            <invec>-1 -2 -3</invec>
            <outvec>-4 -5 -6</outvec>
            <point>-7 -8 -9</point>
          </knot>
        </spline3d>
        <spline3d count="1">
          <knot type="bezier">
            <invec>1e-6 2e-6 3e-6</invec>
            <outvec>4e-6 5e-6 6e-6</outvec>
            <point>7e-6 8e-6 9e-6</point>
          </knot>
        </spline3d>
      </spline>
    </geometry>
  </library>
  <scene id="DefaultScene">
    <node id="test@QS" name="test@QS">
      <translate>10 20 30</translate>
      <scale>1 1 1</scale>
      <rotate>0 0 0 1</rotate>
      <instance url="#test@QS-geometry"/>
    </node>
  </scene>
</SonicPath>"##;

    #[test]
    fn parses_evidenced_sonic_path_fields() {
        let document = parse_path_xml(SAMPLE.as_bytes()).unwrap();

        assert_eq!(document.geometries.len(), 1);
        let geometry = &document.geometries[0];
        assert_eq!(geometry.id, "test@QS-geometry");
        assert_eq!(geometry.name, "test@QS-geometry");
        assert_eq!(geometry.width, 0.2);
        assert_eq!(geometry.splines.len(), 2);
        assert_eq!(geometry.splines[0].knots.len(), 2);
        assert_eq!(geometry.splines[1].knots.len(), 1);

        let first = &geometry.splines[0].knots[0];
        assert_eq!(first.kind, KnotType::Corner);
        assert_eq!(first.in_vec, [0.0, 1.0, 2.0]);
        assert_eq!(first.out_vec, [3.0, 4.0, 5.0]);
        assert_eq!(first.point, [6.0, 7.0, 8.0]);

        assert_eq!(geometry.splines[0].knots[1].kind, KnotType::BezierCorner);
        assert_eq!(geometry.splines[1].knots[0].kind, KnotType::Bezier);

        let node = &document.nodes[0];
        assert_eq!(node.id, "test@QS");
        assert_eq!(node.translate, [10.0, 20.0, 30.0]);
        assert_eq!(node.scale, [1.0, 1.0, 1.0]);
        assert_eq!(node.rotate, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(node.instance_url, "#test@QS-geometry");
        assert_eq!(node.geometry_id(), "test@QS-geometry");
        assert_eq!(
            document.geometry(node.geometry_id()).unwrap().id,
            "test@QS-geometry"
        );
    }

    #[test]
    fn rejects_spline_count_mismatch() {
        let bad = SAMPLE.replace(r#"<spline count="2""#, r#"<spline count="3""#);
        assert_eq!(
            parse_path_xml(bad.as_bytes()).unwrap_err(),
            PathParseError::CountMismatch {
                field: "spline/spline3d",
                declared: 3,
                actual: 2,
            }
        );
    }

    #[test]
    fn rejects_knot_count_mismatch() {
        let bad = SAMPLE.replacen(r#"<spline3d count="2">"#, r#"<spline3d count="3">"#, 1);
        assert_eq!(
            parse_path_xml(bad.as_bytes()).unwrap_err(),
            PathParseError::CountMismatch {
                field: "spline3d/knot",
                declared: 3,
                actual: 2,
            }
        );
    }

    #[test]
    fn rejects_unknown_knot_type_instead_of_guessing() {
        let bad = SAMPLE.replacen(r#"<knot type="corner">"#, r#"<knot type="mystery">"#, 1);
        assert_eq!(
            parse_path_xml(bad.as_bytes()).unwrap_err(),
            PathParseError::UnsupportedKnotType("mystery".to_owned())
        );
    }

    #[test]
    fn real_mykonos_assets_parse_when_extracted_fixture_exists() {
        let root = std::env::temp_dir().join("ur-mykonos");
        let main = root.join("myk_path_act01.path.xml");
        let guide = root.join("StageGuidePath.path.xml");
        if !(main.exists() && guide.exists()) {
            return;
        }

        let main_doc = parse_path_xml(&std::fs::read(main).unwrap()).unwrap();
        assert_eq!(main_doc.geometries.len(), 6);
        assert_eq!(main_doc.nodes.len(), 6);

        let quick = main_doc.geometry("myk_Act1_quick001@QS-geometry").unwrap();
        assert_eq!(quick.width, 0.2);
        assert_eq!(quick.splines.len(), 2);
        assert_eq!(quick.splines[0].knots.len(), 129);
        assert_eq!(quick.splines[1].knots.len(), 129);

        let camera = main_doc.geometry("myk_Act1_camera001-geometry").unwrap();
        assert_eq!(camera.splines.len(), 1);
        assert_eq!(camera.splines[0].knots.len(), 2);

        let quick_node = main_doc.node("myk_Act1_quick001@QS").unwrap();
        assert_eq!(quick_node.geometry_id(), "myk_Act1_quick001@QS-geometry");
        assert_eq!(quick_node.translate, [-206.193, -50.3601, -741.758]);

        let guide_doc = parse_path_xml(&std::fs::read(guide).unwrap()).unwrap();
        assert_eq!(guide_doc.geometries.len(), 1);
        assert_eq!(guide_doc.nodes.len(), 1);
        let geometry = guide_doc.geometry("StageGuidePath-geometry").unwrap();
        assert_eq!(geometry.splines.len(), 1);
        assert_eq!(geometry.splines[0].knots.len(), 15);
        assert_eq!(geometry.splines[0].knots[0].kind, KnotType::BezierCorner);
        assert_eq!(
            guide_doc.node("StageGuidePath").unwrap().geometry_id(),
            "StageGuidePath-geometry"
        );
    }
}
