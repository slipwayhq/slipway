use serde_json::{Map, Value};

pub(super) fn apply(data: &mut Value, path_str: &str, value: Value) {
    let path = parse_path(path_str);
    apply_inner(data, &path, value)
}

fn apply_inner(data: &mut Value, path: &[PathElement], value: Value) {
    if path.is_empty() {
        *data = value;
        return;
    }

    match &path[0] {
        PathElement::Key(key) => {
            if !data.is_object() {
                *data = Value::Object(Map::new());
            }
            let obj = data.as_object_mut().expect("Should be an object");
            let next = obj.entry(key.clone()).or_insert(Value::Null);
            apply_inner(next, &path[1..], value);
        }
        PathElement::Index(index) => {
            if !data.is_array() {
                *data = Value::Array(Vec::new());
            }
            let arr = data.as_array_mut().expect("Should be an array");
            // Extend the array if needed
            while arr.len() <= *index {
                arr.push(Value::Null);
            }
            apply_inner(&mut arr[*index], &path[1..], value);
        }
        PathElement::Wildcard => {
            if let Some(arr) = data.as_array_mut() {
                for item in arr {
                    apply_inner(item, &path[1..], value.clone());
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum PathElement {
    Key(String),
    Index(usize),
    Wildcard,
}

fn parse_path(path: &str) -> Vec<PathElement> {
    let mut elements = Vec::new();
    let mut chars = path.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c == '.' {
            // Skip the '.' separator
            chars.next();
        } else if c == '[' {
            // Parse index or wildcard
            chars.next(); // Skip '['
            let mut idx = String::new();
            while let Some(&c) = chars.peek() {
                if c == ']' {
                    chars.next(); // Skip ']'
                    break;
                } else {
                    idx.push(
                        chars
                            .next()
                            .expect("Should have character which was peeked"),
                    );
                }
            }
            if idx == "*" {
                elements.push(PathElement::Wildcard);
            } else if let Ok(n) = idx.parse::<usize>() {
                elements.push(PathElement::Index(n));
            } else {
                panic!("Invalid index in path: {}", idx);
            }
        } else {
            // Parse key
            let mut key = String::new();
            while let Some(&c) = chars.peek() {
                if c == '.' || c == '[' {
                    break;
                } else {
                    key.push(
                        chars
                            .next()
                            .expect("Should have character which was peeked"),
                    );
                }
            }
            elements.push(PathElement::Key(key));
        }
    }

    elements
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_path_simple() {
        let path = "a.b.c";
        let parsed = parse_path(path);
        let expected = vec![
            PathElement::Key("a".to_string()),
            PathElement::Key("b".to_string()),
            PathElement::Key("c".to_string()),
        ];
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_path_with_indices() {
        let path = "a.b[2].c[3]";
        let parsed = parse_path(path);
        let expected = vec![
            PathElement::Key("a".to_string()),
            PathElement::Key("b".to_string()),
            PathElement::Index(2),
            PathElement::Key("c".to_string()),
            PathElement::Index(3),
        ];
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_path_with_wildcard() {
        let path = "a.b[*].c";
        let parsed = parse_path(path);
        let expected = vec![
            PathElement::Key("a".to_string()),
            PathElement::Key("b".to_string()),
            PathElement::Wildcard,
            PathElement::Key("c".to_string()),
        ];
        assert_eq!(parsed, expected);
    }

    #[test]
    fn apply_set_simple() {
        let mut data = json!({});
        let path = "a.b.c";
        let value = json!(42);
        apply(&mut data, path, value);
        let expected = json!({"a": {"b": {"c": 42}}});
        assert_eq!(data, expected);
    }

    #[test]
    fn apply_set_with_indices() {
        let mut data = json!({});
        let path = "a.b[1].c[2]";
        let value = json!(42);
        apply(&mut data, path, value);
        let expected = json!({
            "a": {
                "b": [
                    null,
                    {
                        "c": [
                            null,
                            null,
                            42
                        ]
                    }
                ]
            }
        });
        assert_eq!(data, expected);
    }

    #[test]
    fn apply_set_with_wildcard() {
        let mut data = json!({
            "a": {
                "b": [
                    {"c": 1},
                    {"c": 2},
                    {"c": 3}
                ]
            }
        });
        let path = "a.b[*].d";
        let value = json!(4);
        apply(&mut data, path, value);
        let expected = json!({
            "a": {
                "b": [
                    {"c": 1, "d": 4},
                    {"c": 2, "d": 4},
                    {"c": 3, "d": 4}
                ]
            }
        });
        assert_eq!(data, expected);
    }

    #[test]
    fn set_nonexistent_path() {
        let mut data = json!({});
        let path = "x.y[0].z";
        let value = json!(100);
        apply(&mut data, path, value);
        let expected = json!({
            "x": {
                "y": [
                    {
                        "z": 100
                    }
                ]
            }
        });
        assert_eq!(data, expected);
    }

    #[test]
    fn wildcard_on_non_array() {
        let mut data = json!({"a": {"b": {"c": 1}}});
        let path = "a.b[*].d";
        let value = json!(4);
        apply(&mut data, path, value);
        // Since "b" is not an array, wildcard should have no effect
        let expected = json!({"a": {"b": {"c": 1}}});
        assert_eq!(data, expected);
    }
}
