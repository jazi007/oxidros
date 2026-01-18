#!/usr/bin/env cargo run --example validate_ros_idl_files --
//! ROS2 IDL Parser Validation Tool
//!
//! This binary scans a ROS2 distribution directory for all IDL files and attempts to parse them.
//! It reports any parsing failures, helping identify parser limitations and compatibility issues.
//!
//! Usage:
//!   cargo run --example validate_ros_idl_files -- /opt/ros/humble
//!   cargo run --example validate_ros_idl_files -- /opt/ros/iron
//!   cargo run --example validate_ros_idl_files -- ~/ros2_ws/install

use ros2msg::idl::grammar::parse_idl_file;
use ros2msg::idl::types::IdlLocator;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Statistics for parsing results
#[derive(Debug, Default)]
struct ParseStats {
    total_files: usize,
    successful_parses: usize,
    failed_parses: usize,
    parse_errors: Vec<ParseFailure>,
}

/// Information about a parsing failure
#[derive(Debug, Clone)]
struct ParseFailure {
    file_path: PathBuf,
    error_message: String,
    package_name: Option<String>,
}

impl ParseStats {
    fn success_rate(&self) -> f64 {
        if self.total_files == 0 {
            0.0
        } else {
            (self.successful_parses as f64 / self.total_files as f64) * 100.0
        }
    }

    fn add_success(&mut self, _file_path: &Path) {
        self.total_files += 1;
        self.successful_parses += 1;
    }

    fn add_failure(&mut self, file_path: &Path, error: String) {
        self.total_files += 1;
        self.failed_parses += 1;

        let package_name = extract_package_name(file_path);
        self.parse_errors.push(ParseFailure {
            file_path: file_path.to_path_buf(),
            error_message: error,
            package_name,
        });
    }
}

/// Extract package name from IDL file path
fn extract_package_name(file_path: &Path) -> Option<String> {
    let path_str = file_path.to_string_lossy();

    // Look for patterns like: /opt/ros/humble/share/package_name/
    if let Some(share_idx) = path_str.find("/share/") {
        let after_share = &path_str[share_idx + 7..];
        if let Some(slash_idx) = after_share.find('/') {
            return Some(after_share[..slash_idx].to_string());
        }
    }

    // Only look for _msgs/_interfaces patterns if the path contains /share/ or /install/
    // This ensures we only extract from proper ROS installations, not relative paths
    if (path_str.contains("/share/") || path_str.contains("/install/"))
        && let Some(pkg_match) = file_path.components().find_map(|comp| {
            let comp_str = comp.as_os_str().to_string_lossy();
            if comp_str.ends_with("_msgs") || comp_str.ends_with("_interfaces") {
                Some(comp_str.to_string())
            } else {
                None
            }
        })
    {
        return Some(pkg_match);
    }

    None
}

/// Recursively find all .idl files in a directory
fn find_idl_files(root_path: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut idl_files = Vec::new();

    fn visit_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip certain directories that are unlikely to contain IDL files
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if !["build", "devel", "log", ".git", "target", "__pycache__"].contains(&dir_name) {
                    visit_dir(&path, files)?;
                }
            } else if let Some(extension) = path.extension()
                && extension == "idl"
            {
                files.push(path);
            }
        }
        Ok(())
    }

    visit_dir(root_path, &mut idl_files)?;
    Ok(idl_files)
}

/// Parse a single IDL file and return the result
fn parse_single_file(file_path: &Path, base_path: &Path) -> Result<(), String> {
    let locator = IdlLocator::new(
        base_path.to_path_buf(),
        file_path
            .strip_prefix(base_path)
            .unwrap_or(file_path)
            .to_path_buf(),
    );

    parse_idl_file(&locator)
        .map(|_| ())
        .map_err(|e| format!("{:?}", e))
}

/// Analyze error patterns to identify common parser issues
fn analyze_error_patterns(stats: &ParseStats) -> HashMap<String, usize> {
    let mut error_patterns = HashMap::new();

    for failure in &stats.parse_errors {
        // Categorize errors by type
        let category = if failure.error_message.contains("include") {
            "Include directives"
        } else if failure.error_message.contains("annotation") {
            "Annotation parsing"
        } else if failure.error_message.contains("expected") {
            "Grammar/Syntax issues"
        } else if failure.error_message.contains("sequence") {
            "Sequence types"
        } else if failure.error_message.contains("string") {
            "String constants"
        } else {
            "Other"
        };

        *error_patterns.entry(category.to_string()).or_insert(0) += 1;
    }

    error_patterns
}

/// Display detailed parsing results
fn display_results(stats: &ParseStats, verbose: bool) {
    println!("\n🔍 ROS2 IDL Parser Validation Results");
    println!("====================================");
    println!("📊 Total IDL files found: {}", stats.total_files);
    println!("✅ Successfully parsed: {}", stats.successful_parses);
    println!("❌ Failed to parse: {}", stats.failed_parses);
    println!("📈 Success rate: {:.1}%", stats.success_rate());

    if stats.failed_parses > 0 {
        println!("\n🔍 Error Pattern Analysis:");
        let error_patterns = analyze_error_patterns(stats);
        for (pattern, count) in error_patterns.iter() {
            println!("  • {}: {} files", pattern, count);
        }

        if verbose {
            println!("\n❌ Detailed Parse Failures:");
            for (i, failure) in stats.parse_errors.iter().enumerate().take(20) {
                println!("\n{}. File: {}", i + 1, failure.file_path.display());
                if let Some(pkg) = &failure.package_name {
                    println!("   Package: {}", pkg);
                }
                println!("   Error: {}", failure.error_message);
            }

            if stats.parse_errors.len() > 20 {
                println!("\n... and {} more failures", stats.parse_errors.len() - 20);
            }
        } else {
            println!("\n💡 Use --verbose flag to see detailed error information");
        }
    }

    if stats.success_rate() == 100.0 {
        println!("\n🎉 Perfect! All IDL files parsed successfully!");
        println!("🚀 Your parser is fully compatible with this ROS distribution!");
    } else if stats.success_rate() >= 90.0 {
        println!("\n✨ Excellent! Over 90% success rate.");
        println!("🔧 Minor parser improvements needed for full compatibility.");
    } else if stats.success_rate() >= 75.0 {
        println!("\n👍 Good progress! Most files parse successfully.");
        println!("🛠️  Some parser features need enhancement.");
    } else {
        println!("\n⚠️  Significant parser improvements needed.");
        println!("🔨 Focus on the most common error patterns above.");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <ros_distro_path> [--verbose]", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} /opt/ros/humble", args[0]);
        eprintln!("  {} /opt/ros/iron --verbose", args[0]);
        eprintln!("  {} ~/ros2_ws/install", args[0]);
        std::process::exit(1);
    }

    let ros_path = PathBuf::from(&args[1]);
    let verbose = args.contains(&"--verbose".to_string());

    if !ros_path.exists() {
        eprintln!("❌ Error: Path '{}' does not exist", ros_path.display());
        std::process::exit(1);
    }

    if !ros_path.is_dir() {
        eprintln!("❌ Error: '{}' is not a directory", ros_path.display());
        std::process::exit(1);
    }

    println!("🔍 Scanning for IDL files in: {}", ros_path.display());
    let start_scan = Instant::now();

    let idl_files = find_idl_files(&ros_path)?;
    let scan_duration = start_scan.elapsed();

    if idl_files.is_empty() {
        println!("⚠️  No IDL files found in the specified directory.");
        println!("💡 Make sure you're pointing to a ROS2 installation or workspace.");
        return Ok(());
    }

    println!(
        "📁 Found {} IDL files in {:.2?}",
        idl_files.len(),
        scan_duration
    );
    println!("🧪 Starting parse validation...\n");

    let mut stats = ParseStats::default();
    let start_parse = Instant::now();

    for (i, file_path) in idl_files.iter().enumerate() {
        // Show progress for large numbers of files
        if idl_files.len() > 50 && i % 10 == 0 {
            print!("Progress: {}/{} files\r", i, idl_files.len());
        }

        match parse_single_file(file_path, &ros_path) {
            Ok(()) => {
                stats.add_success(file_path);
                if verbose {
                    println!("✅ {}", file_path.display());
                }
            }
            Err(error) => {
                stats.add_failure(file_path, error);
                if verbose {
                    println!("❌ {}: Parse failed", file_path.display());
                }
            }
        }
    }

    let parse_duration = start_parse.elapsed();
    println!("\n⏱️  Parsing completed in {:.2?}", parse_duration);

    display_results(&stats, verbose);

    // Exit with appropriate code
    if stats.success_rate() == 100.0 {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_find_idl_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create some test IDL files
        let msg_dir = temp_path.join("share/test_msgs/msg");
        std::fs::create_dir_all(&msg_dir).unwrap();

        let srv_dir = temp_path.join("share/test_msgs/srv");
        std::fs::create_dir_all(&srv_dir).unwrap();

        // Create IDL files
        File::create(msg_dir.join("TestMessage.idl"))
            .unwrap()
            .write_all(b"struct TestMessage { int32 data; };")
            .unwrap();

        File::create(srv_dir.join("TestService.idl"))
            .unwrap()
            .write_all(
                b"struct TestRequest { string query; }; struct TestResponse { bool success; };",
            )
            .unwrap();

        // Create a non-IDL file (should be ignored)
        File::create(temp_path.join("README.md"))
            .unwrap()
            .write_all(b"# Test")
            .unwrap();

        let idl_files = find_idl_files(temp_path).unwrap();
        assert_eq!(idl_files.len(), 2);

        let file_names: Vec<String> = idl_files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(file_names.contains(&"TestMessage.idl".to_string()));
        assert!(file_names.contains(&"TestService.idl".to_string()));
    }

    #[test]
    fn test_extract_package_name() {
        let test_cases = vec![
            (
                "/opt/ros/humble/share/geometry_msgs/msg/Point.idl",
                Some("geometry_msgs"),
            ),
            (
                "/home/user/ws/install/my_interfaces/share/my_interfaces/srv/Custom.idl",
                Some("my_interfaces"),
            ),
            ("./test_msgs/msg/Test.idl", None),
        ];

        for (path_str, expected) in test_cases {
            let path = PathBuf::from(path_str);
            let result = extract_package_name(&path);
            assert_eq!(
                result,
                expected.map(|s| s.to_string()),
                "Failed for path: {}",
                path_str
            );
        }
    }

    #[test]
    fn test_parse_stats() {
        let mut stats = ParseStats::default();

        assert_eq!(stats.success_rate(), 0.0);

        stats.add_success(&PathBuf::from("test1.idl"));
        stats.add_success(&PathBuf::from("test2.idl"));
        stats.add_failure(&PathBuf::from("test3.idl"), "Parse error".to_string());

        assert_eq!(stats.total_files, 3);
        assert_eq!(stats.successful_parses, 2);
        assert_eq!(stats.failed_parses, 1);
        // Use approximate comparison for floating point
        let success_rate = stats.success_rate();
        assert!((success_rate - 66.66666666666667).abs() < 1e-10);
    }
}
