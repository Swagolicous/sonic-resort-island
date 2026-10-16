use std::{error::Error, fmt, str};

const XCOMPRESS_MAGIC: [u8; 4] = [0x0f, 0xf5, 0x12, 0xee];

#[derive(Debug, Clone, PartialEq)]
pub struct PlacementSet {
    pub objects: Vec<PlacedObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedObject {
    /// The direct child element name under `<SetObject>`.
    ///
    /// On observed Windmill Isle sets this is the runtime object type, e.g.
    /// `Spring`, `Ring`, `ObjCameraParallel`, or `ObjSoundPoint`.
    pub object_type: String,

    /// An explicit direct `<Name>` scalar, if present.
    ///
    /// No direct `<Name>` field is present in the observed Windmill Isle set
    /// layers. This intentionally does not synthesize a name from SetObjectID,
    /// GroupID, CueName, FileName, or the element/type name.
    pub name: Option<String>,

    /// Observed set transforms are four scalar components named w/x/y/z.
    /// They are preserved under those names instead of assigning additional
    /// mathematical semantics here.
    pub position: PlacementVector,
    pub rotation: PlacementVector,

    /// All other leaf values, preserving duplicates and nested paths.
    ///
    /// Examples observed in Windmill Isle include:
    /// `SetObjectID=2223`, `GroundOffset=0`,
    /// `Target/SetObjectID=2366`, and `TargetDirection/w=3019.95`.
    /// Self-closing leaves such as `<PositionList/>` are retained with an empty
    /// value. Position and Rotation leaves are omitted because they are exposed
    /// as structured fields above. An explicit direct Name is also omitted
    /// because it is exposed by `name`.
    pub parameters: Vec<RawParameter>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementVector {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawParameter {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementError {
    message: String,
}

impl PlacementError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for PlacementError {}

/// Parses either a plain Sonic Unleashed set XML payload or the XCompress
/// wrapper used by loose `update/work/.../*.set.xml` files.
///
/// The Windmill Isle control archive contains plain UTF-8 set XML entries,
/// while `update/work/ActD_MykonosAct1/Base.set.xml` begins with the standard
/// XCompress signature and decompresses to the same XML schema.
pub fn parse_set_file(bytes: &[u8]) -> Result<PlacementSet, PlacementError> {
    if bytes.starts_with(&XCOMPRESS_MAGIC) {
        let xml = crate::xcompress::decompress(bytes)
            .map_err(|error| PlacementError::new(format!("decompress set XCompress: {error}")))?;
        parse_set_xml(&xml)
    } else {
        parse_set_xml(bytes)
    }
}

/// Parses the minimal attribute-free set XML schema evidenced by Windmill Isle.
///
/// Supported syntax is intentionally narrow:
/// - the XML declaration;
/// - normal opening/closing elements;
/// - self-closing elements;
/// - UTF-8 scalar text.
///
/// The observed Windmill Isle set layers contain no element attributes,
/// entities, comments, or CDATA. Unsupported markup is rejected rather than
/// guessed.
pub fn parse_set_xml(bytes: &[u8]) -> Result<PlacementSet, PlacementError> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let text = str::from_utf8(bytes)
        .map_err(|error| PlacementError::new(format!("set XML is not UTF-8: {error}")))?;
    let root = parse_xml(text)?;

    if root.name != "SetObject" {
        return Err(PlacementError::new(format!(
            "unexpected set XML root <{}>, expected <SetObject>",
            root.name
        )));
    }

    let mut objects = Vec::new();
    for node in &root.children {
        if node.name == "LayerDefine" {
            continue;
        }

        let position_node = direct_child(node, "Position").ok_or_else(|| {
            PlacementError::new(format!(
                "placement <{}> is missing direct <Position>",
                node.name
            ))
        })?;
        let rotation_node = direct_child(node, "Rotation").ok_or_else(|| {
            PlacementError::new(format!(
                "placement <{}> is missing direct <Rotation>",
                node.name
            ))
        })?;

        let position = parse_vector(position_node, &node.name, "Position")?;
        let rotation = parse_vector(rotation_node, &node.name, "Rotation")?;
        let name = direct_child(node, "Name").map(|child| child.text.trim().to_owned());

        let mut parameters = Vec::new();
        for child in &node.children {
            if child.name == "Position" || child.name == "Rotation" || child.name == "Name" {
                continue;
            }
            flatten_parameters(child, &child.name, &mut parameters);
        }

        objects.push(PlacedObject {
            object_type: node.name.clone(),
            name,
            position,
            rotation,
            parameters,
        });
    }

    Ok(PlacementSet { objects })
}

fn direct_child<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    node.children.iter().find(|child| child.name == name)
}

fn parse_vector(
    node: &XmlNode,
    object_type: &str,
    field_name: &str,
) -> Result<PlacementVector, PlacementError> {
    fn component(
        node: &XmlNode,
        object_type: &str,
        field_name: &str,
        component_name: &str,
    ) -> Result<f32, PlacementError> {
        let child = direct_child(node, component_name).ok_or_else(|| {
            PlacementError::new(format!(
                "<{object_type}> {field_name} is missing <{component_name}>"
            ))
        })?;
        if !child.children.is_empty() {
            return Err(PlacementError::new(format!(
                "<{object_type}> {field_name}/{component_name} is not scalar"
            )));
        }

        let value = child.text.trim();
        value.parse::<f32>().map_err(|error| {
            PlacementError::new(format!(
                "<{object_type}> {field_name}/{component_name} value {value:?} is not f32: {error}"
            ))
        })
    }

    Ok(PlacementVector {
        w: component(node, object_type, field_name, "w")?,
        x: component(node, object_type, field_name, "x")?,
        y: component(node, object_type, field_name, "y")?,
        z: component(node, object_type, field_name, "z")?,
    })
}

fn flatten_parameters(node: &XmlNode, path: &str, out: &mut Vec<RawParameter>) {
    if node.children.is_empty() {
        out.push(RawParameter {
            key: path.to_owned(),
            value: node.text.trim().to_owned(),
        });
        return;
    }

    for child in &node.children {
        let child_path = format!("{path}/{}", child.name);
        flatten_parameters(child, &child_path, out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct XmlNode {
    name: String,
    text: String,
    children: Vec<XmlNode>,
}

fn parse_xml(text: &str) -> Result<XmlNode, PlacementError> {
    let mut cursor = 0usize;
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;

    while cursor < text.len() {
        let Some(relative) = text[cursor..].find('<') else {
            if text[cursor..].trim().is_empty() {
                break;
            }
            return Err(PlacementError::new(
                "set XML has trailing text without a closing tag",
            ));
        };
        let tag_start = cursor + relative;
        let scalar = &text[cursor..tag_start];

        if let Some(node) = stack.last_mut() {
            node.text.push_str(scalar);
        } else if !scalar.trim().is_empty() {
            return Err(PlacementError::new(
                "set XML contains text outside the root element",
            ));
        }

        if text[tag_start..].starts_with("<?") {
            let relative_end = text[tag_start + 2..]
                .find("?>")
                .ok_or_else(|| PlacementError::new("unterminated XML processing instruction"))?;
            let end = tag_start + 2 + relative_end + 2;
            let instruction = &text[tag_start + 2..end - 2];
            if !instruction.trim_start().starts_with("xml") {
                return Err(PlacementError::new(
                    "unsupported XML processing instruction",
                ));
            }
            cursor = end;
            continue;
        }

        if text[tag_start..].starts_with("<!") {
            return Err(PlacementError::new(
                "unsupported XML declaration/comment/CDATA markup",
            ));
        }

        let relative_end = text[tag_start + 1..]
            .find('>')
            .ok_or_else(|| PlacementError::new("unterminated XML tag"))?;
        let tag_end = tag_start + 1 + relative_end;
        let raw = text[tag_start + 1..tag_end].trim();

        if raw.is_empty() {
            return Err(PlacementError::new("empty XML tag"));
        }

        if let Some(close_name) = raw.strip_prefix('/') {
            let close_name = close_name.trim();
            if close_name.is_empty() || close_name.chars().any(char::is_whitespace) {
                return Err(PlacementError::new("invalid XML closing tag"));
            }

            let node = stack
                .pop()
                .ok_or_else(|| PlacementError::new(format!("unexpected </{close_name}>")))?;
            if node.name != close_name {
                return Err(PlacementError::new(format!(
                    "mismatched XML close </{close_name}>, expected </{}>",
                    node.name
                )));
            }

            attach_node(node, &mut stack, &mut root)?;
        } else {
            let self_closing = raw.ends_with('/');
            let body = if self_closing {
                raw[..raw.len() - 1].trim()
            } else {
                raw
            };

            let mut parts = body.split_whitespace();
            let name = parts
                .next()
                .ok_or_else(|| PlacementError::new("empty XML opening tag"))?;
            if parts.next().is_some() {
                return Err(PlacementError::new(format!(
                    "unsupported attributes on <{name}>"
                )));
            }
            if !valid_tag_name(name) {
                return Err(PlacementError::new(format!(
                    "unsupported XML tag name <{name}>"
                )));
            }

            let node = XmlNode {
                name: name.to_owned(),
                text: String::new(),
                children: Vec::new(),
            };

            if self_closing {
                attach_node(node, &mut stack, &mut root)?;
            } else {
                stack.push(node);
            }
        }

        cursor = tag_end + 1;
    }

    if !stack.is_empty() {
        return Err(PlacementError::new(format!(
            "unclosed XML element <{}>",
            stack.last().unwrap().name
        )));
    }

    root.ok_or_else(|| PlacementError::new("set XML has no root element"))
}

fn attach_node(
    node: XmlNode,
    stack: &mut [XmlNode],
    root: &mut Option<XmlNode>,
) -> Result<(), PlacementError> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
        return Ok(());
    }

    if root.is_some() {
        return Err(PlacementError::new("set XML contains multiple roots"));
    }
    *root = Some(node);
    Ok(())
}

fn valid_tag_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8" standalone="no" ?>
<SetObject>
  <LayerDefine>
    <Center><w>0</w><x>0</x><y>0</y><z>0</z></Center>
  </LayerDefine>
  <Spring>
    <GroundOffset>0</GroundOffset>
    <GroupID>WhiteIsland_ACT1_003</GroupID>
    <Position><w>1</w><x>-17.7204</x><y>-57.6588</y><z>-669.09</z></Position>
    <PositionList/>
    <Rotation><w>0.934866</w><x>0.0616557</x><y>-0.168891</y><z>0.3061</z></Rotation>
    <SetObjectID>2296</SetObjectID>
    <Target><SetObjectID>2366</SetObjectID></Target>
  </Spring>
</SetObject>"#;

    #[test]
    fn parses_evidenced_set_shape() {
        let set = parse_set_xml(SAMPLE.as_bytes()).unwrap();
        assert_eq!(set.objects.len(), 1);

        let object = &set.objects[0];
        assert_eq!(object.object_type, "Spring");
        assert_eq!(object.name, None);
        assert_eq!(
            object.position,
            PlacementVector {
                w: 1.0,
                x: -17.7204,
                y: -57.6588,
                z: -669.09,
            }
        );
        assert_eq!(
            object.rotation,
            PlacementVector {
                w: 0.934866,
                x: 0.0616557,
                y: -0.168891,
                z: 0.3061,
            }
        );
        assert_eq!(
            object.parameters,
            vec![
                RawParameter {
                    key: "GroundOffset".into(),
                    value: "0".into(),
                },
                RawParameter {
                    key: "GroupID".into(),
                    value: "WhiteIsland_ACT1_003".into(),
                },
                RawParameter {
                    key: "PositionList".into(),
                    value: "".into(),
                },
                RawParameter {
                    key: "SetObjectID".into(),
                    value: "2296".into(),
                },
                RawParameter {
                    key: "Target/SetObjectID".into(),
                    value: "2366".into(),
                },
            ]
        );
    }

    #[test]
    fn uses_only_explicit_name() {
        let xml = SAMPLE.replace(
            "<GroundOffset>0</GroundOffset>",
            "<Name>authored_name</Name><GroundOffset>0</GroundOffset>",
        );
        let set = parse_set_xml(xml.as_bytes()).unwrap();
        assert_eq!(set.objects[0].name.as_deref(), Some("authored_name"));
        assert!(
            set.objects[0]
                .parameters
                .iter()
                .all(|parameter| parameter.key != "Name")
        );
    }

    #[test]
    fn preserves_duplicate_nested_raw_parameters() {
        let xml = SAMPLE.replace(
            "<Target><SetObjectID>2366</SetObjectID></Target>",
            "<Target><SetObjectID>2366</SetObjectID><SetObjectID>2367</SetObjectID></Target>",
        );
        let set = parse_set_xml(xml.as_bytes()).unwrap();
        let values: Vec<_> = set.objects[0]
            .parameters
            .iter()
            .filter(|parameter| parameter.key == "Target/SetObjectID")
            .map(|parameter| parameter.value.as_str())
            .collect();
        assert_eq!(values, ["2366", "2367"]);
    }

    #[test]
    fn rejects_attributes_not_evidenced_in_set_elements() {
        let xml = SAMPLE.replace("<Spring>", r#"<Spring guessed="true">"#);
        let error = parse_set_xml(xml.as_bytes()).unwrap_err();
        assert!(error.to_string().contains("unsupported attributes"));
    }

    #[test]
    fn rejects_missing_transform_instead_of_guessing() {
        let xml = SAMPLE.replace(
            "<Position><w>1</w><x>-17.7204</x><y>-57.6588</y><z>-669.09</z></Position>",
            "",
        );
        let error = parse_set_xml(xml.as_bytes()).unwrap_err();
        assert!(error.to_string().contains("missing direct <Position>"));
    }

    #[test]
    fn plain_file_entry_point_matches_xml_parser() {
        assert_eq!(
            parse_set_file(SAMPLE.as_bytes()).unwrap(),
            parse_set_xml(SAMPLE.as_bytes()).unwrap()
        );
    }

    #[test]
    fn compressed_file_entry_point_uses_xcompress_decoder() {
        let malformed = [XCOMPRESS_MAGIC.as_slice(), &[0u8; 12]].concat();
        let error = parse_set_file(&malformed).unwrap_err();
        assert!(error.to_string().contains("decompress set XCompress"));
    }
}
