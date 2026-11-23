/// Dot path to specific object property
/// e.g. "user.name" will get "John" from the following object:
/// ```json
/// {
///     "user": {
///         "name": "John"
///     }
/// }
/// ```
/// Works with arrays: "users.0.name"
pub(crate) type PropertyPath<'a> = Vec<&'a str>;

/// Forcefully upserts nested property in the given json value
pub(crate) fn set_nested_property(
    data: &serde_json::Value,
    path: &[&str],
    value: serde_json::Value,
) -> serde_json::Value {
    match path.split_first() {
        Some((property_name, rest)) => match data {
            serde_json::Value::Object(map) => {
                // Preserve all siblings: clone the map, update the target key, return.
                let mut new_map = map.clone();
                let child = map.get(*property_name).unwrap_or(&serde_json::Value::Null);
                new_map.insert(
                    property_name.to_string(),
                    set_nested_property(child, rest, value),
                );
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(array) => {
                let mut new_array = array.clone();
                if let Ok(index) = property_name.parse::<usize>() {
                    if let Some(item) = array.get(index) {
                        new_array[index] = set_nested_property(item, rest, value);
                    } else {
                        // Fill any gaps, then append the computed value.
                        while new_array.len() < index {
                            new_array.push(serde_json::Value::Null);
                        }
                        new_array.push(set_nested_property(&serde_json::Value::Null, rest, value));
                    }
                } else {
                    // Non-numeric key on an array: wrap into an object and append.
                    new_array.push(nest_object(None, property_name, rest, value));
                }
                serde_json::Value::Array(new_array)
            }
            _ => nest_object(None, property_name, rest, value),
        },
        None => value,
    }
}

fn nest_object(
    data_arg: Option<&serde_json::Value>,
    property_name: &str,
    remaining_path: &[&str],
    value: serde_json::Value,
) -> serde_json::Value {
    let mut map = serde_json::Map::default();
    map.insert(
        property_name.to_string(),
        set_nested_property(
            data_arg.unwrap_or(&serde_json::Value::Null),
            remaining_path,
            value,
        ),
    );
    serde_json::Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----------------------------------------------------------------
    // Empty / single-segment paths
    // ----------------------------------------------------------------

    #[test]
    fn empty_path_returns_value_unchanged() {
        let data = serde_json::json!({"a": 1});
        let result = set_nested_property(&data, &[], serde_json::json!("hello"));
        assert_eq!(result, serde_json::json!("hello"));
    }

    #[test]
    fn single_segment_on_object_replaces_existing() {
        let data = serde_json::json!({"name": "old"});
        let result = set_nested_property(&data, &["name"], serde_json::json!("new"));
        assert_eq!(result, serde_json::json!({"name": "new"}));
    }

    #[test]
    fn single_segment_on_object_creates_missing() {
        let data = serde_json::json!({"other": 1});
        let result = set_nested_property(&data, &["name"], serde_json::json!("new"));
        assert_eq!(result, serde_json::json!({"other": 1, "name": "new"}));
    }

    #[test]
    fn single_segment_on_primitive_replaces_with_object() {
        // Forceful upsert: primitives are overwritten by the new structure.
        let data = serde_json::json!("string");
        let result = set_nested_property(&data, &["a", "b"], serde_json::json!(42));
        assert_eq!(result, serde_json::json!({"a": {"b": 42}}));
    }

    // ----------------------------------------------------------------
    // Deeply nested objects
    // ----------------------------------------------------------------

    #[test]
    fn deeply_nested_object_upsert() {
        let data = serde_json::json!({"a": {"b": {"c": 1}}});
        let result = set_nested_property(&data, &["a", "b", "c"], serde_json::json!(99));
        assert_eq!(result, serde_json::json!({"a": {"b": {"c": 99}}}));
    }

    #[test]
    fn deeply_nested_object_creates_missing_levels() {
        let data = serde_json::json!({});
        let result = set_nested_property(&data, &["a", "b", "c"], serde_json::json!("deep"));
        assert_eq!(result, serde_json::json!({"a": {"b": {"c": "deep"}}}));
    }

    #[test]
    fn mid_level_partial_update_preserves_siblings() {
        let data = serde_json::json!({"x": 1, "y": {"z": 2}});
        let result = set_nested_property(&data, &["y", "w"], serde_json::json!(3));
        assert_eq!(result, serde_json::json!({"x": 1, "y": {"z": 2, "w": 3}}));
    }

    // ----------------------------------------------------------------
    // Arrays — index within bounds
    // ----------------------------------------------------------------

    #[test]
    fn array_index_update_existing_element() {
        let data = serde_json::json!([10, 20, 30]);
        let result = set_nested_property(&data, &["1"], serde_json::json!(99));
        assert_eq!(result, serde_json::json!([10, 99, 30]));
    }

    #[test]
    fn array_nested_property_update() {
        let data = serde_json::json!([{"name": "a"}, {"name": "b"}]);
        let result = set_nested_property(&data, &["1", "name"], serde_json::json!("modified"));
        assert_eq!(
            result,
            serde_json::json!([{"name": "a"}, {"name": "modified"}])
        );
    }

    #[test]
    fn array_deep_nested_update() {
        let data = serde_json::json!([{"tags": ["x", "y"]}]);
        let result = set_nested_property(&data, &["0", "tags", "1"], serde_json::json!("z"));
        assert_eq!(result, serde_json::json!([{"tags": ["x", "z"]}]));
    }

    // ----------------------------------------------------------------
    // Arrays — push (index == len)
    // ----------------------------------------------------------------

    #[test]
    fn array_push_at_end_simple() {
        let data = serde_json::json!([1, 2]);
        let result = set_nested_property(&data, &["2"], serde_json::json!(3));
        assert_eq!(result, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn array_push_at_end_with_nested_path() {
        let data = serde_json::json!([{"id": 1}]);
        let result = set_nested_property(&data, &["1", "name"], serde_json::json!("new"));
        assert_eq!(result, serde_json::json!([{"id": 1}, {"name": "new"}]));
    }

    // ----------------------------------------------------------------
    // Arrays — index beyond length (gap-filling with Null)
    // ----------------------------------------------------------------

    #[test]
    fn array_gap_fill_then_set() {
        let data = serde_json::json!([1]);
        let result = set_nested_property(&data, &["3"], serde_json::json!(99));
        assert_eq!(result, serde_json::json!([1, null, null, 99]));
    }

    #[test]
    fn array_gap_fill_nested_path() {
        let data = serde_json::json!([{"a": 1}]);
        let result = set_nested_property(&data, &["2", "b"], serde_json::json!(2));
        assert_eq!(result, serde_json::json!([{"a": 1}, null, {"b": 2}]));
    }

    // ----------------------------------------------------------------
    // Array treated as object when key is non-numeric
    // ----------------------------------------------------------------

    #[test]
    fn array_non_numeric_key_becomes_object() {
        let data = serde_json::json!([1, 2]);
        let result = set_nested_property(&data, &["foo"], serde_json::json!("bar"));
        assert_eq!(result, serde_json::json!([1, 2, {"foo": "bar"}]));
    }

    #[test]
    fn array_non_numeric_key_with_nested_path() {
        let data = serde_json::json!([1]);
        let result = set_nested_property(&data, &["foo", "bar"], serde_json::json!(2));
        assert_eq!(result, serde_json::json!([1, {"foo": {"bar": 2}}]));
    }

    // ----------------------------------------------------------------
    // Mixed nesting: object inside array, array inside object
    // ----------------------------------------------------------------

    #[test]
    fn object_containing_array_updated() {
        let data = serde_json::json!({"items": [1, 2, 3]});
        let result = set_nested_property(&data, &["items", "1"], serde_json::json!(99));
        assert_eq!(result, serde_json::json!({"items": [1, 99, 3]}));
    }

    #[test]
    fn array_containing_object_deep_path() {
        let data = serde_json::json!([{"user": {"name": "A"}}]);
        let result = set_nested_property(&data, &["0", "user", "name"], serde_json::json!("B"));
        assert_eq!(result, serde_json::json!([{"user": {"name": "B"}}]));
    }

    // ----------------------------------------------------------------
    // Scalar / primitive targets
    // ----------------------------------------------------------------

    #[test]
    fn replacing_number_with_nested_object() {
        let data = serde_json::json!({"count": 5});
        let result = set_nested_property(&data, &["count", "inner"], serde_json::json!("x"));
        assert_eq!(result, serde_json::json!({"count": {"inner": "x"}}));
    }

    #[test]
    fn replacing_string_with_nested_object() {
        let data = serde_json::json!({"tag": "hello"});
        let result = set_nested_property(&data, &["tag", "sub"], serde_json::json!(1));
        assert_eq!(result, serde_json::json!({"tag": {"sub": 1}}));
    }

    #[test]
    fn replacing_bool_with_nested_object() {
        let data = serde_json::json!({"flag": true});
        let result = set_nested_property(&data, &["flag", "nested"], serde_json::json!(true));
        assert_eq!(result, serde_json::json!({"flag": {"nested": true}}));
    }

    #[test]
    fn replacing_null_with_nested_object() {
        let data = serde_json::json!({"slot": null});
        let result = set_nested_property(&data, &["slot", "key"], serde_json::json!(42));
        assert_eq!(result, serde_json::json!({"slot": {"key": 42}}));
    }

    // ----------------------------------------------------------------
    // Overwrite existing nested structure
    // ----------------------------------------------------------------

    #[test]
    fn overwrite_entire_subtree() {
        let data = serde_json::json!({"a": {"b": {"c": 1, "d": 2}}});
        let result = set_nested_property(&data, &["a", "b"], serde_json::json!("replaced"));
        assert_eq!(result, serde_json::json!({"a": {"b": "replaced"}}));
    }

    #[test]
    fn overwrite_array_element_with_object() {
        let data = serde_json::json!([1, 2, 3]);
        let result = set_nested_property(&data, &["1"], serde_json::json!({"x": true}));
        assert_eq!(result, serde_json::json!([1, {"x": true}, 3]));
    }

    // ----------------------------------------------------------------
    // Edge cases: negative indices, huge indices, special string keys
    // ----------------------------------------------------------------

    #[test]
    fn negative_string_index_treated_as_object_key() {
        // "-1" doesn't parse as usize → treated as object key
        let data = serde_json::json!([1, 2]);
        let result = set_nested_property(&data, &["-1"], serde_json::json!("neg"));
        assert_eq!(result, serde_json::json!([1, 2, {"-1": "neg"}]));
    }

    #[test]
    fn zero_index_works_correctly() {
        let data = serde_json::json!([10]);
        let result = set_nested_property(&data, &["0", "v"], serde_json::json!(0));
        assert_eq!(result, serde_json::json!([{"v": 0}]));
    }

    #[test]
    fn large_index_fills_gaps() {
        let data = serde_json::json!([1]);
        let result = set_nested_property(&data, &["5"], serde_json::json!(6));
        assert_eq!(result, serde_json::json!([1, null, null, null, null, 6]));
    }

    // ----------------------------------------------------------------
    // Value types preserved correctly
    // ----------------------------------------------------------------

    #[test]
    fn nested_array_value_preserved() {
        let inner = serde_json::json!([10, 20]);
        let data = serde_json::json!({});
        let result = set_nested_property(&data, &["arr"], inner.clone());
        assert_eq!(result, serde_json::json!({"arr": [10, 20]}));
        // Verify it's actually an array, not serialized
        if let serde_json::Value::Array(ref v) = result["arr"] {
            assert_eq!(v.len(), 2);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn boolean_and_numeric_values_preserved() {
        let data = serde_json::json!({});
        let result = set_nested_property(&data, &["flag"], serde_json::json!(false));
        assert_eq!(result, serde_json::json!({"flag": false}));

        let result2 = set_nested_property(&data, &["num"], serde_json::json!(2.71));
        assert_eq!(result2, serde_json::json!({"num": 2.71}));
    }

    #[test]
    fn null_value_can_be_explicitly_set() {
        let data = serde_json::json!({"a": 1});
        let result = set_nested_property(&data, &["a"], serde_json::Value::Null);
        assert_eq!(result, serde_json::json!({"a": null}));
    }
}
