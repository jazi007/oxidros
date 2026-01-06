#!/usr/bin/env cargo run --example validate_ros_msg_srv_action_files --
//! ROS2 Message/Service/Action Parser Validation Tool
//!
//! This binary scans a ROS2 distribution directory for all .msg, .srv, and .action files
//! and attempts to parse them. It reports any parsing failures, helping identify parser
//! limitations and compatibility issues.
//!
//! Usage:
//!   cargo run --example validate_ros_msg_srv_action_files -- /opt/ros/humble
//!   cargo run --example validate_ros_msg_srv_action_files -- /opt/ros/jazzy
//!   cargo run --example validate_ros_msg_srv_action_files -- ~/ros2_ws/install

use ros2msg::{parse_action_file, parse_message_file, parse_service_file};
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
    msg_files: usize,
    srv_files: usize,
    action_files: usize,
}

/// Information about a parsing failure
#[derive(Debug, Clone)]
struct ParseFailure {
    file_path: PathBuf,
    file_type: FileType,
    error_message: String,
    package_name: Option<String>,
}

/// Type of interface file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileType {
    Message,
    Service,
    Action,
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::Message => write!(f, "msg"),
            FileType::Service => write!(f, "srv"),
            FileType::Action => write!(f, "action"),
        }
    }
}

impl ParseStats {
    fn success_rate(&self) -> f64 {
        if self.total_files == 0 {
            0.0
        } else {
            (self.successful_parses as f64 / self.total_files as f64) * 100.0
        }
    }

    fn add_success(&mut self, _file_path: &Path, file_type: FileType) {
        self.total_files += 1;
        self.successful_parses += 1;
        match file_type {
            FileType::Message => self.msg_files += 1,
            FileType::Service => self.srv_files += 1,
            FileType::Action => self.action_files += 1,
        }
    }

    fn add_failure(&mut self, file_path: &Path, file_type: FileType, error: String) {
        self.total_files += 1;
        self.failed_parses += 1;

        let package_name = extract_package_name(file_path);
        self.parse_errors.push(ParseFailure {
            file_path: file_path.to_path_buf(),
            file_type,
            error_message: error,
            package_name,
        });
    }
}

/// Find all .msg, .srv, and .action files in a directory recursively
fn find_interface_files(root_path: &Path) -> Vec<(PathBuf, FileType)> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(root_path) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                files.extend(find_interface_files(&path));
            } else if let Some(ext) = path.extension() {
                let file_type = match ext.to_str() {
                    Some("msg") => Some(FileType::Message),
                    Some("srv") => Some(FileType::Service),
                    Some("action") => Some(FileType::Action),
                    _ => None,
                };

                if let Some(ft) = file_type {
                    files.push((path, ft));
                }
            }
        }
    }

    files
}

/// Extract package name from file path
fn extract_package_name(file_path: &Path) -> Option<String> {
    // Try to find a pattern like: .../share/package_name/msg|srv|action/...
    let components: Vec<_> = file_path.components().collect();

    for (i, component) in components.iter().enumerate() {
        if let Some("share") = component.as_os_str().to_str()
            && i + 1 < components.len()
        {
            return components[i + 1].as_os_str().to_str().map(String::from);
        }
    }

    None
}

/// Parse a single interface file
fn parse_interface_file(
    file_path: &Path,
    file_type: FileType,
    stats: &mut ParseStats,
    verbose: bool,
) {
    let package = extract_package_name(file_path).unwrap_or_else(|| "unknown".to_string());

    let result = match file_type {
        FileType::Message => parse_message_file(&package, file_path)
            .map(|_| ())
            .map_err(|e| format!("{e}")),
        FileType::Service => parse_service_file(&package, file_path)
            .map(|_| ())
            .map_err(|e| format!("{e}")),
        FileType::Action => parse_action_file(&package, file_path)
            .map(|_| ())
            .map_err(|e| format!("{e}")),
    };

    match result {
        Ok(_) => {
            stats.add_success(file_path, file_type);
            if verbose {
                println!("✓ {}", file_path.display());
            }
        }
        Err(error) => {
            stats.add_failure(file_path, file_type, error.to_string());
            if verbose {
                println!("✗ {} - Error: {}", file_path.display(), error);
            }
        }
    }
}

/// Analyze error patterns in failed parses
fn analyze_error_patterns(stats: &ParseStats) -> HashMap<String, usize> {
    let mut error_patterns: HashMap<String, usize> = HashMap::new();

    for failure in &stats.parse_errors {
        // Extract the main error category (simplified pattern matching)
        let category = if failure.error_message.contains("Parse error") {
            "Parse error"
        } else if failure.error_message.contains("Invalid") {
            "Validation error"
        } else if failure.error_message.contains("Expected") {
            "Syntax error"
        } else if failure.error_message.contains("type") {
            "Type error"
        } else {
            "Other error"
        };

        *error_patterns.entry(category.to_string()).or_insert(0) += 1;
    }

    error_patterns
}

/// Display parsing results
fn display_results(stats: &ParseStats, verbose: bool) {
    println!("\n🔍 ROS2 Message/Service/Action Parser Validation Results");
    println!("====================================");
    println!("📊 Total files found: {}", stats.total_files);
    println!("  • Message files (.msg): {}", stats.msg_files);
    println!("  • Service files (.srv): {}", stats.srv_files);
    println!("  • Action files (.action): {}", stats.action_files);
    println!("✅ Successfully parsed: {}", stats.successful_parses);
    println!("❌ Failed to parse: {}", stats.failed_parses);
    println!("📈 Success rate: {:.1}%", stats.success_rate());

    if !stats.parse_errors.is_empty() {
        let error_patterns = analyze_error_patterns(stats);

        println!("\n🔍 Error Pattern Analysis:");
        for (pattern, count) in error_patterns.iter() {
            println!("  • {}: {} files", pattern, count);
        }

        if verbose {
            println!("\n❌ Failed Files:");
            for (idx, failure) in stats.parse_errors.iter().enumerate() {
                println!("\n{}. File: {}", idx + 1, failure.file_path.display());
                println!("   Type: {}", failure.file_type);
                if let Some(pkg) = &failure.package_name {
                    println!("   Package: {}", pkg);
                }
                println!("   Error: {}", failure.error_message);
            }
        } else {
            println!("\n💡 Use --verbose flag to see detailed error information");
        }
    }

    // Display summary message
    if stats.success_rate() == 100.0 {
        println!("\n🎉 Perfect! All files parsed successfully!");
        println!("🚀 Your parser is fully compatible with this ROS distribution!");
    } else if stats.success_rate() >= 90.0 {
        println!("\n✨ Excellent! Over 90% success rate.");
        println!("🔧 Minor parser improvements needed for full compatibility.");
    } else if stats.success_rate() >= 75.0 {
        println!("\n👍 Good progress! Over 75% success rate.");
        println!("🔨 Some parser enhancements needed.");
    } else {
        println!("\n⚠️  Significant parser improvements needed.");
        println!("🛠️  Review error patterns above for guidance.");
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <ros2_distro_path> [--verbose]", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} /opt/ros/jazzy", args[0]);
        eprintln!("  {} /opt/ros/humble --verbose", args[0]);
        std::process::exit(1);
    }

    let ros_path = Path::new(&args[1]);
    let verbose = args
        .get(2)
        .is_some_and(|arg| arg == "--verbose" || arg == "-v");

    if !ros_path.exists() {
        eprintln!("❌ Error: Path does not exist: {}", ros_path.display());
        std::process::exit(1);
    }

    println!(
        "🔍 Scanning for message/service/action files in: {}",
        ros_path.display()
    );

    // Find all interface files
    let start_find = Instant::now();
    let files = find_interface_files(ros_path);
    let find_duration = start_find.elapsed();

    println!(
        "📁 Found {} interface files in {:.2}ms",
        files.len(),
        find_duration.as_secs_f64() * 1000.0
    );
    println!("🧪 Starting parse validation...\n");

    // Parse all files
    let mut stats = ParseStats::default();
    let start_parse = Instant::now();

    for (i, (file_path, file_type)) in files.iter().enumerate() {
        // Progress indicator every 50 files (if not verbose)
        if !verbose && i % 50 == 0 && i > 0 {
            print!("\rProgress: {}/{} files", i, files.len());
        }

        parse_interface_file(file_path, *file_type, &mut stats, verbose);
    }

    let parse_duration = start_parse.elapsed();

    if !verbose {
        println!("\rProgress: {}/{} files", files.len(), files.len());
    }

    println!(
        "⏱️  Parsing completed in {:.2}ms",
        parse_duration.as_secs_f64() * 1000.0
    );

    // Display results
    display_results(&stats, verbose);
}
