//! Helpers to work with ROS parameters
//!  - Load yaml files
//!
use crate::common::Result;
use oxidros::{oxidros_core, parameter::Value};
use std::{collections::BTreeMap, io::Read, path::Path};
use yaml_rust2::{Yaml, YamlLoader, yaml::Hash};

/// ROS nodes parameters type alias
pub type NodesParameters = BTreeMap<String, BTreeMap<String, Value>>;

fn yaml2value(value: &Yaml) -> Result<Value> {
    match value {
        Yaml::Null | Yaml::BadValue => Ok(Value::NotSet),
        Yaml::Boolean(v) => Ok(Value::Bool(*v)),
        Yaml::Integer(v) => Ok(Value::I64(*v)),
        Yaml::Real(_) => Ok(Value::F64(value.as_f64().ok_or("Invalid float")?)),
        Yaml::String(v) => Ok(Value::String(v.to_owned())),
        Yaml::Array(v) => {
            let vv = v.first().cloned().ok_or("")?;
            match vv {
                Yaml::Boolean(_) => Ok(Value::VecBool(
                    v.iter()
                        .map(|vv| vv.as_bool().ok_or("bool expected"))
                        .collect::<std::result::Result<Vec<bool>, _>>()?,
                )),
                Yaml::Integer(_) => Ok(Value::VecI64(
                    v.iter()
                        .map(|vv| vv.as_i64().ok_or("i64 expected"))
                        .collect::<std::result::Result<Vec<i64>, _>>()?,
                )),
                Yaml::Real(_) => Ok(Value::VecF64(
                    v.iter()
                        .map(|vv| vv.as_f64().ok_or("f64 expected"))
                        .collect::<std::result::Result<Vec<f64>, _>>()?,
                )),
                Yaml::String(_) => Ok(Value::VecString(
                    v.iter()
                        .map(|vv| vv.as_str().map(|vv| vv.to_owned()).ok_or("String expected"))
                        .collect::<std::result::Result<Vec<String>, _>>()?,
                )),
                _ => Err("Not a valid array".into()),
            }
        }
        _ => Err("Invalid Yaml type".into()),
    }
}
fn parse_node_params(values: &Hash) -> Result<BTreeMap<String, Value>> {
    let mut params = BTreeMap::new();
    for (k, v) in values {
        let Yaml::String(key) = k else {
            continue;
        };
        params.insert(key.to_owned(), yaml2value(v)?);
    }
    Ok(params)
}

/// A helper function to load a yaml file of parameters
pub fn load_prameters_file<P: AsRef<Path>>(path: P) -> Result<NodesParameters> {
    let mut parameters = NodesParameters::default();
    let mut file =
        std::fs::File::open(path).map_err(|e| oxidros_core::Error::Other(e.to_string()))?;
    let mut s = String::new();
    file.read_to_string(&mut s)
        .map_err(|e| oxidros_core::Error::Other(e.to_string()))?;
    let docs =
        YamlLoader::load_from_str(&s).map_err(|e| oxidros_core::Error::Other(e.to_string()))?;
    for doc in docs {
        let Yaml::Hash(h) = doc else {
            log::warn!("Unexpected Yaml type");
            continue;
        };
        for v in h {
            let (Yaml::String(name), Yaml::Hash(params)) = v else {
                continue;
            };
            let Some(Yaml::Hash(values)) = params.get(&Yaml::String("ros__parameters".to_string()))
            else {
                continue;
            };
            parameters.insert(name, parse_node_params(values)?);
        }
    }
    Ok(parameters)
}
/// A helper function to load a yaml file of parameters from env
pub fn load_prameters_from_env() -> Result<NodesParameters> {
    let mut args = std::env::args();
    let mut parameters = NodesParameters::default();
    args.by_ref().find(|x| x.eq("--params-file"));
    let Some(path) = args.next() else {
        return Ok(parameters);
    };
    let mut file =
        std::fs::File::open(path).map_err(|e| oxidros_core::Error::Other(e.to_string()))?;
    let mut s = String::new();
    file.read_to_string(&mut s)
        .map_err(|e| oxidros_core::Error::Other(e.to_string()))?;
    let docs =
        YamlLoader::load_from_str(&s).map_err(|e| oxidros_core::Error::Other(e.to_string()))?;
    for doc in docs {
        let Yaml::Hash(h) = doc else {
            log::warn!("Unexpected Yaml type");
            continue;
        };
        for v in h {
            let (Yaml::String(name), Yaml::Hash(params)) = v else {
                continue;
            };
            let Some(Yaml::Hash(values)) = params.get(&Yaml::String("ros__parameters".to_string()))
            else {
                continue;
            };
            parameters.insert(name, parse_node_params(values)?);
        }
    }
    Ok(parameters)
}
