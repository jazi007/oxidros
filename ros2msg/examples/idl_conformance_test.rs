#!/usr/bin/env cargo run --example idl_conformance_test --features serde --
//! IDL Parser Conformance Test
//!
//! This example compares the output of our Rust IDL parser against the Python
//! rosidl_parser to ensure compatibility.
//!
//! The Rust parser's serde output should match Python's rosidl_parser output
//! directly - no transformation needed.
//!
//! Usage:
//!   # Compare a single IDL file
//!   cargo run --example idl_conformance_test --features serde -- <idl-file>
//!
//!   # Compare with verbose output
//!   cargo run --example idl_conformance_test --features serde -- <idl-file> --verbose
//!
//!   # Generate reference JSON from Python parser (requires rosidl_parser)
//!   cargo run --example idl_conformance_test --features serde -- <idl-file> --generate-reference
//!
//!   # Compare against an existing reference JSON file
//!   cargo run --example idl_conformance_test --features serde -- <idl-file> --reference <ref.json>
//!
//! Requirements:
//!   - For --generate-reference: Python 3 with rosidl_parser installed
//!   - Feature `serde` must be enabled

use ros2msg::idl::{IdlFile, parse_idl_file};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Result type for conformance tests
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Conformance test configuration
#[derive(Debug)]
struct Config {
    idl_file: PathBuf,
    verbose: bool,
    generate_reference: bool,
    reference_file: Option<PathBuf>,
    python_script: Option<PathBuf>,
}

impl Config {
    fn from_args() -> Result<Self> {
        let args: Vec<String> = env::args().collect();

        if args.len() < 2 {
            print_usage(&args[0]);
            std::process::exit(1);
        }

        let mut idl_file = None;
        let mut verbose = false;
        let mut generate_reference = false;
        let mut reference_file = None;
        let mut python_script = None;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--verbose" | "-v" => verbose = true,
                "--generate-reference" | "-g" => generate_reference = true,
                "--reference" | "-r" => {
                    i += 1;
                    if i < args.len() {
                        reference_file = Some(PathBuf::from(&args[i]));
                    } else {
                        eprintln!("Error: --reference requires a file path");
                        std::process::exit(1);
                    }
                }
                "--python-script" | "-p" => {
                    i += 1;
                    if i < args.len() {
                        python_script = Some(PathBuf::from(&args[i]));
                    } else {
                        eprintln!("Error: --python-script requires a file path");
                        std::process::exit(1);
                    }
                }
                "--help" | "-h" => {
                    print_usage(&args[0]);
                    std::process::exit(0);
                }
                arg if !arg.starts_with('-') => {
                    if idl_file.is_none() {
                        idl_file = Some(PathBuf::from(arg));
                    }
                }
                _ => {
                    eprintln!("Unknown option: {}", args[i]);
                    std::process::exit(1);
                }
            }
            i += 1;
        }

        let idl_file = idl_file.ok_or("IDL file path is required")?;

        Ok(Config {
            idl_file,
            verbose,
            generate_reference,
            reference_file,
            python_script,
        })
    }
}

fn print_usage(program: &str) {
    eprintln!(
        r#"IDL Parser Conformance Test

Usage: {program} <idl-file> [OPTIONS]

Arguments:
  <idl-file>                   Path to the IDL file to parse

Options:
  -v, --verbose                Show detailed output
  -g, --generate-reference     Generate reference JSON from Python parser
  -r, --reference <file>       Compare against an existing reference JSON file
  -p, --python-script <file>   Path to the Python IDL to JSON script
  -h, --help                   Show this help message

Examples:
  # Parse an IDL file and show the JSON output
  {program} path/to/Message.idl --verbose

  # Generate reference JSON from Python rosidl_parser
  {program} path/to/Message.idl --generate-reference

  # Compare our parser output against a reference JSON
  {program} path/to/Message.idl --reference reference.json
"#
    );
}

/// Convert our IdlFile to JSON - should match Python's format directly
fn idl_file_to_json(idl_file: &IdlFile) -> Value {
    serde_json::to_value(idl_file).unwrap_or(Value::Null)
}

/// Run the Python IDL parser and get JSON output
fn run_python_parser(idl_path: &Path, python_script: Option<&Path>) -> Result<Value> {
    let script_path = if let Some(script) = python_script {
        script.to_path_buf()
    } else {
        let possible_paths = vec![
            PathBuf::from("idl_to_json_python.py"),
            PathBuf::from("../../idl_to_json_python.py"),
            PathBuf::from("../../../idl_to_json_python.py"),
            env::current_dir()?.join("idl_to_json_python.py"),
        ];

        possible_paths.into_iter().find(|p| p.exists()).ok_or(
            "Could not find idl_to_json_python.py. Use --python-script to specify the path.",
        )?
    };

    let output = Command::new("python3")
        .arg(&script_path)
        .arg(idl_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Python parser failed: {}", stderr).into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let json: Value = serde_json::from_str(&stdout)?;
    Ok(json)
}

/// Compare two JSON values and report differences
fn compare_json(rust_json: &Value, python_json: &Value, path: &str, _verbose: bool) -> Vec<String> {
    let mut differences = Vec::new();

    match (rust_json, python_json) {
        (Value::Null, Value::Null) => {}
        (Value::Bool(r), Value::Bool(p)) => {
            if r != p {
                differences.push(format!("{}: bool mismatch: rust={}, python={}", path, r, p));
            }
        }
        (Value::Number(r), Value::Number(p)) => {
            if let (Some(rf), Some(pf)) = (r.as_f64(), p.as_f64()) {
                if (rf - pf).abs() > 1e-10 {
                    differences.push(format!(
                        "{}: number mismatch: rust={}, python={}",
                        path, rf, pf
                    ));
                }
            } else if r != p {
                differences.push(format!(
                    "{}: number mismatch: rust={}, python={}",
                    path, r, p
                ));
            }
        }
        (Value::String(r), Value::String(p)) => {
            if r != p {
                differences.push(format!(
                    "{}: string mismatch: rust='{}', python='{}'",
                    path, r, p
                ));
            }
        }
        (Value::Array(r), Value::Array(p)) => {
            if r.len() != p.len() {
                differences.push(format!(
                    "{}: array length mismatch: rust={}, python={}",
                    path,
                    r.len(),
                    p.len()
                ));
            }
            for (i, (rv, pv)) in r.iter().zip(p.iter()).enumerate() {
                let child_path = format!("{}[{}]", path, i);
                differences.extend(compare_json(rv, pv, &child_path, _verbose));
            }
        }
        (Value::Object(r), Value::Object(p)) => {
            for key in r.keys() {
                if !p.contains_key(key) {
                    differences.push(format!("{}.{}: key only in rust output", path, key));
                }
            }
            for key in p.keys() {
                if !r.contains_key(key) {
                    differences.push(format!("{}.{}: key only in python output", path, key));
                }
            }
            for key in r.keys() {
                if let Some(pv) = p.get(key) {
                    let rv = &r[key];
                    let child_path = format!("{}.{}", path, key);
                    differences.extend(compare_json(rv, pv, &child_path, _verbose));
                }
            }
        }
        _ => {
            differences.push(format!(
                "{}: type mismatch: rust={}, python={}",
                path,
                json_type_name(rust_json),
                json_type_name(python_json)
            ));
        }
    }

    differences
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn main() -> Result<()> {
    let config = Config::from_args()?;

    if !config.idl_file.exists() {
        eprintln!("Error: IDL file not found: {:?}", config.idl_file);
        std::process::exit(1);
    }

    println!("=== IDL Parser Conformance Test ===");
    println!("IDL file: {:?}", config.idl_file);
    println!();

    println!("Parsing with Rust IDL parser...");
    let rust_result = parse_idl_file(&config.idl_file);

    match rust_result {
        Ok(idl_file) => {
            let rust_json = idl_file_to_json(&idl_file);

            if config.verbose {
                println!("\n--- Rust Parser JSON Output ---");
                println!("{}", serde_json::to_string_pretty(&rust_json)?);
            }

            if config.generate_reference {
                println!("\nGenerating reference JSON from Python rosidl_parser...");
                match run_python_parser(&config.idl_file, config.python_script.as_deref()) {
                    Ok(python_json) => {
                        println!("\n--- Python Parser JSON Output ---");
                        println!("{}", serde_json::to_string_pretty(&python_json)?);

                        // Try to save reference file, but don't fail if we can't (e.g., read-only dir)
                        let ref_file = config.idl_file.with_extension("reference.json");
                        match fs::write(&ref_file, serde_json::to_string_pretty(&python_json)?) {
                            Ok(_) => println!("\nReference JSON saved to: {:?}", ref_file),
                            Err(e) => eprintln!("{ref_file:?} => fail due to {e}"),
                        }

                        println!("\n--- Comparison ---");
                        let differences =
                            compare_json(&rust_json, &python_json, "root", config.verbose);
                        if differences.is_empty() {
                            println!("✓ No differences found! Parser outputs match.");
                        } else {
                            println!("✗ Found {} differences:", differences.len());
                            for diff in &differences {
                                println!("  - {}", diff);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not run Python parser: {}", e);
                        eprintln!("Make sure rosidl_parser is installed and ROS2 is sourced");
                    }
                }
            } else if let Some(ref_file) = config.reference_file {
                println!("\nComparing against reference: {:?}", ref_file);
                let ref_content = fs::read_to_string(&ref_file)?;
                let python_json: Value = serde_json::from_str(&ref_content)?;

                if config.verbose {
                    println!("\n--- Reference JSON ---");
                    println!("{}", serde_json::to_string_pretty(&python_json)?);
                }

                println!("\n--- Comparison ---");
                let differences = compare_json(&rust_json, &python_json, "root", config.verbose);
                if differences.is_empty() {
                    println!("✓ No differences found! Parser outputs match.");
                } else {
                    println!("✗ Found {} differences:", differences.len());
                    for diff in &differences {
                        println!("  - {}", diff);
                    }
                    std::process::exit(1);
                }
            } else {
                println!("\n--- Rust Parser JSON Output ---");
                println!("{}", serde_json::to_string_pretty(&rust_json)?);
                println!("\nTip: Use --generate-reference to compare with Python rosidl_parser");
                println!("     Use --reference <file.json> to compare with an existing reference");
            }

            println!("\n✓ Rust parser completed successfully");
        }
        Err(e) => {
            eprintln!("✗ Rust parser failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_json_identical() {
        let json1 = serde_json::json!({
            "name": "test",
            "value": 42
        });
        let json2 = json1.clone();

        let differences = compare_json(&json1, &json2, "root", false);
        assert!(differences.is_empty());
    }

    #[test]
    fn test_compare_json_different_values() {
        let json1 = serde_json::json!({
            "name": "test",
            "value": 42
        });
        let json2 = serde_json::json!({
            "name": "test",
            "value": 100
        });

        let differences = compare_json(&json1, &json2, "root", false);
        assert!(!differences.is_empty());
        assert!(differences[0].contains("value"));
    }

    #[test]
    fn test_compare_json_missing_keys() {
        let json1 = serde_json::json!({
            "name": "test",
            "extra": "field"
        });
        let json2 = serde_json::json!({
            "name": "test"
        });

        let differences = compare_json(&json1, &json2, "root", false);
        assert!(!differences.is_empty());
        assert!(differences[0].contains("extra"));
    }
}
