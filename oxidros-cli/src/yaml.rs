use serde_json::Value;
use yaml_rust2::YamlLoader;

/// Parse a YAML string into a `serde_json::Value`.
pub(crate) fn parse_yaml_to_json(s: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let docs = YamlLoader::load_from_str(s).map_err(|e| format!("Invalid YAML: {e}"))?;
    let doc = docs.into_iter().next().ok_or("Empty YAML input")?;
    Ok(yaml_to_json(&doc))
}

fn yaml_to_json(y: &yaml_rust2::Yaml) -> Value {
    match y {
        yaml_rust2::Yaml::Null | yaml_rust2::Yaml::BadValue => Value::Null,
        yaml_rust2::Yaml::Boolean(b) => Value::Bool(*b),
        yaml_rust2::Yaml::Integer(i) => Value::Number((*i).into()),
        yaml_rust2::Yaml::Real(s) => s
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::String(s.clone())),
        yaml_rust2::Yaml::String(s) => Value::String(s.clone()),
        yaml_rust2::Yaml::Array(arr) => Value::Array(arr.iter().map(yaml_to_json).collect()),
        yaml_rust2::Yaml::Hash(map) => {
            let obj = map
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        yaml_rust2::Yaml::String(s) => s.clone(),
                        other => format!("{other:?}"),
                    };
                    (key, yaml_to_json(v))
                })
                .collect();
            Value::Object(obj)
        }
        yaml_rust2::Yaml::Alias(_) => Value::Null,
    }
}
