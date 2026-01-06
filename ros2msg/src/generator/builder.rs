//! Builder interface for the code generator
//!
//! NOTE: This generator processes files sequentially (not in parallel) to avoid
//! race conditions when resolving dependencies. While parallel processing could
//! improve performance, it introduces complexity around deduplication and could
//! cause the same dependency files to be generated multiple times concurrently.
//! The sequential approach is simpler, safer, and fast enough for most use cases.
use super::{GeneratedCode, GeneratorResult, codegen::CodeGenerator, config::GeneratorConfig};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Main generator builder (bindgen-style API)
///
/// # Example
///
/// ```no_run
/// use ros2msg::generator::Generator;
///
/// Generator::new()
///     .header("// Auto-generated - do not edit")
///     .derive_debug(true)
///     .derive_clone(true)
///     .include("/opt/ros/jazzy/share/std_msgs/msg/Header.msg")
///     .include("/opt/ros/jazzy/share/geometry_msgs/msg/Pose.msg")
///     .output_dir("src/generated")
///     .emit_rerun_if_changed(true)
///     .generate()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct Generator {
    config: GeneratorConfig,
    input_files: Vec<PathBuf>,
}

impl Generator {
    /// Create a new generator with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: GeneratorConfig::new(),
            input_files: Vec::new(),
        }
    }

    /// Add a header comment to generated files
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new()
    ///     .header("// Auto-generated code - do not edit!");
    /// ```
    #[must_use]
    pub fn header<S: AsRef<str>>(mut self, header: S) -> Self {
        self.config.header = Some(header.as_ref().to_string());
        self
    }

    /// Generate `Debug` implementations
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new().derive_debug(true);
    /// ```
    #[must_use]
    pub fn derive_debug(mut self, enable: bool) -> Self {
        if enable && !self.config.derives.contains(&"Debug".to_string()) {
            self.config.derives.push("Debug".to_string());
        }
        self
    }

    /// Generate `Clone` implementations
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new().derive_clone(true);
    /// ```
    #[must_use]
    pub fn derive_clone(mut self, enable: bool) -> Self {
        if enable && !self.config.derives.contains(&"Clone".to_string()) {
            self.config.derives.push("Clone".to_string());
        }
        self
    }

    /// Generate `Copy` implementations
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new().derive_copy(true);
    /// ```
    #[must_use]
    pub fn derive_copy(mut self, enable: bool) -> Self {
        if enable && !self.config.derives.contains(&"Copy".to_string()) {
            self.config.derives.push("Copy".to_string());
        }
        self
    }

    /// Generate `Default` implementations
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new().derive_default(true);
    /// ```
    #[must_use]
    pub fn derive_default(mut self, enable: bool) -> Self {
        if enable && !self.config.derives.contains(&"Default".to_string()) {
            self.config.derives.push("Default".to_string());
        }
        self
    }

    /// Generate `PartialEq` and `Eq` implementations
    ///
    /// Note: This will fail for types containing f32/f64 fields. Use `derive_partialeq()` instead.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new().derive_eq(true);
    /// ```
    #[must_use]
    pub fn derive_eq(mut self, enable: bool) -> Self {
        if enable {
            if !self.config.derives.contains(&"PartialEq".to_string()) {
                self.config.derives.push("PartialEq".to_string());
            }
            if !self.config.derives.contains(&"Eq".to_string()) {
                self.config.derives.push("Eq".to_string());
            }
        }
        self
    }

    /// Generate `PartialEq` implementation only (without `Eq`)
    ///
    /// Use this for types containing f32/f64 fields.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new().derive_partialeq(true);
    /// ```
    #[must_use]
    pub fn derive_partialeq(mut self, enable: bool) -> Self {
        if enable && !self.config.derives.contains(&"PartialEq".to_string()) {
            self.config.derives.push("PartialEq".to_string());
        }
        self
    }

    /// Generate `PartialOrd` and `Ord` implementations
    ///
    /// Note: This will fail for types containing f32/f64 fields. Use `derive_partialord()` instead.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new().derive_ord(true);
    /// ```
    #[must_use]
    pub fn derive_ord(mut self, enable: bool) -> Self {
        if enable {
            if !self.config.derives.contains(&"PartialOrd".to_string()) {
                self.config.derives.push("PartialOrd".to_string());
            }
            if !self.config.derives.contains(&"Ord".to_string()) {
                self.config.derives.push("Ord".to_string());
            }
        }
        self
    }

    /// Generate `PartialOrd` implementation only (without `Ord`)
    ///
    /// Use this for types containing f32/f64 fields.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new().derive_partialord(true);
    /// ```
    #[must_use]
    pub fn derive_partialord(mut self, enable: bool) -> Self {
        if enable && !self.config.derives.contains(&"PartialOrd".to_string()) {
            self.config.derives.push("PartialOrd".to_string());
        }
        self
    }

    /// Generate `Hash` implementations
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new().derive_hash(true);
    /// ```
    #[must_use]
    pub fn derive_hash(mut self, enable: bool) -> Self {
        if enable && !self.config.derives.contains(&"Hash".to_string()) {
            self.config.derives.push("Hash".to_string());
        }
        self
    }

    /// Add a raw line at the top of generated files (after header)
    ///
    /// Useful for adding imports or other declarations.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new()
    ///     .raw_line("use serde::{Serialize, Deserialize};")
    ///     .raw_line("use custom::types::*;");
    /// ```
    #[must_use]
    pub fn raw_line<S: AsRef<str>>(mut self, line: S) -> Self {
        self.config.raw_lines.push(line.as_ref().to_string());
        self
    }

    /// Set the prefix for C types (default: `std::os::raw`)
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new()
    ///     .ctypes_prefix("libc");
    /// ```
    #[must_use]
    pub fn ctypes_prefix<S: AsRef<str>>(mut self, prefix: S) -> Self {
        self.config.ctypes_prefix = Some(prefix.as_ref().to_string());
        self
    }

    /// Enable/disable emitting cargo:rerun-if-changed directives
    ///
    /// When enabled, prints cargo directives that can be used in build.rs
    /// to trigger rebuilds when source files change.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new()
    ///     .emit_rerun_if_changed(true);
    /// ```
    #[must_use]
    pub fn emit_rerun_if_changed(mut self, enable: bool) -> Self {
        self.config.emit_rerun_if_changed = enable;
        self
    }

    /// Add an input file to generate bindings for
    ///
    /// Can be called multiple times to add multiple files.
    /// Automatically detects file type from extension (.msg, .srv, .action, .idl).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ros2msg::generator::Generator;
    ///
    /// Generator::new()
    ///     .include("/opt/ros/jazzy/share/std_msgs/msg/Header.msg")
    ///     .include("/opt/ros/jazzy/share/std_msgs/msg/String.msg")
    ///     .output_dir("src/generated")
    ///     .generate()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn include<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.input_files.push(path.as_ref().to_path_buf());
        self
    }

    /// Add multiple input files to generate bindings for
    ///
    /// Accepts any iterator of paths. Automatically detects file types.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ros2msg::generator::Generator;
    ///
    /// let files = vec![
    ///     "/opt/ros/jazzy/share/std_msgs/msg/Header.msg",
    ///     "/opt/ros/jazzy/share/std_msgs/msg/String.msg",
    ///     "/opt/ros/jazzy/share/geometry_msgs/msg/Pose.msg",
    /// ];
    ///
    /// Generator::new()
    ///     .includes(files)
    ///     .output_dir("src/generated")
    ///     .generate()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn includes<I, P>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        self.input_files
            .extend(paths.into_iter().map(|p| p.as_ref().to_path_buf()));
        self
    }

    /// Set output directory for generated files
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new()
    ///     .output_dir("generated/ros2_msgs");
    /// ```
    #[must_use]
    pub fn output_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.config.output_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Set parse callbacks for customization
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::{Generator, ParseCallbacks, ItemInfo};
    ///
    /// struct MyCallbacks;
    /// impl ParseCallbacks for MyCallbacks {
    ///     fn item_name(&self, info: &ItemInfo) -> Option<String> {
    ///         Some(format!("Ros{}", info.name()))
    ///     }
    /// }
    ///
    /// let generator = Generator::new()
    ///     .parse_callbacks(Box::new(MyCallbacks));
    /// ```
    #[must_use]
    pub fn parse_callbacks<C: super::ParseCallbacks + 'static>(
        mut self,
        callbacks: Box<C>,
    ) -> Self {
        self.config.parse_callbacks = Some(Arc::from(callbacks as Box<dyn super::ParseCallbacks>));
        self
    }

    /// Add an item to the allowlist
    ///
    /// Only items matching allowlist patterns will be generated.
    /// If allowlist is empty, all items are included (except blocklisted).
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new()
    ///     .allowlist_item("Header")
    ///     .allowlist_item("String");
    /// ```
    #[must_use]
    pub fn allowlist_item<S: AsRef<str>>(mut self, item: S) -> Self {
        self.config.allowlist.push(item.as_ref().to_string());
        self
    }

    /// Add an item to the blocklist
    ///
    /// Items matching blocklist patterns will be excluded from generation.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new()
    ///     .blocklist_item("Internal")
    ///     .blocklist_item("Private");
    /// ```
    #[must_use]
    pub fn blocklist_item<S: AsRef<str>>(mut self, item: S) -> Self {
        self.config.blocklist.push(item.as_ref().to_string());
        self
    }

    /// Enable recursive inclusion of dependencies
    ///
    /// When enabled, dependencies of allowlisted items will also be generated.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new()
    ///     .allowlist_recursively(true);
    /// ```
    #[must_use]
    pub fn allowlist_recursively(mut self, enable: bool) -> Self {
        self.config.allowlist_recursively = enable;
        self
    }

    /// Add a directory to search for ROS2 packages when resolving dependencies
    ///
    /// When `allowlist_recursively(true)` is enabled, the generator will search
    /// these paths to find message/service/action files for dependencies.
    ///
    /// Typically, you would add paths like `/opt/ros/jazzy/share` or custom workspace
    /// install directories.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let generator = Generator::new()
    ///     .package_search_path("/opt/ros/jazzy/share")
    ///     .package_search_path("/home/user/ros2_ws/install/share");
    /// ```
    #[must_use]
    pub fn package_search_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config
            .package_search_paths
            .push(path.as_ref().to_path_buf());
        self
    }

    /// Add multiple directories to search for ROS2 packages
    ///
    /// Convenience method to add multiple package search paths at once.
    ///
    /// # Example
    ///
    /// ```
    /// use ros2msg::generator::Generator;
    ///
    /// let paths = vec![
    ///     "/opt/ros/jazzy/share",
    ///     "/home/user/ros2_ws/install/share",
    /// ];
    ///
    /// let generator = Generator::new()
    ///     .package_search_paths(paths);
    /// ```
    #[must_use]
    pub fn package_search_paths<I, P>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        self.config
            .package_search_paths
            .extend(paths.into_iter().map(|p| p.as_ref().to_path_buf()));
        self
    }

    /// Generate bindings and write to output directory
    ///
    /// This is the main entry point. It generates Rust code from all included
    /// ROS2 interface files and writes them to the output directory.
    /// File types are automatically detected from extensions.
    ///
    /// # Errors
    ///
    /// Returns an error if generation fails or output directory is not set
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ros2msg::generator::Generator;
    ///
    /// Generator::new()
    ///     .header("// Auto-generated")
    ///     .derive_debug(true)
    ///     .derive_clone(true)
    ///     .include("/opt/ros/jazzy/share/std_msgs/msg/Header.msg")
    ///     .include("/opt/ros/jazzy/share/std_msgs/msg/String.msg")
    ///     .output_dir("src/generated")
    ///     .generate()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn generate(self) -> GeneratorResult<()> {
        let output_dir = self
            .config
            .output_dir
            .as_ref()
            .ok_or(super::ConfigError::OutputDirectoryRequired)?;

        if self.input_files.is_empty() {
            return Err(super::ConfigError::NoInputFiles.into());
        }

        // Create output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)?;

        // Generate all code
        let all_generated = self.generate_all_files()?;

        // Write generated files to disk
        self.write_generated_files(output_dir, &all_generated)?;

        Ok(())
    }

    /// Generate all files including dependencies
    fn generate_all_files(&self) -> GeneratorResult<Vec<GeneratedCode>> {
        use std::collections::HashSet;

        // Generate initial files sequentially
        let results: GeneratorResult<Vec<_>> = self
            .input_files
            .iter()
            .map(|path| self.generate_single_file(path))
            .collect();

        let mut all_generated = results?;

        // If recursive mode is enabled, find and generate dependencies
        if self.config.allowlist_recursively {
            let mut processed_packages: HashSet<String> = HashSet::new();
            let mut packages_to_process: Vec<String> = Vec::new();

            // Collect initial dependencies
            for code in &all_generated {
                processed_packages.insert(code.package_name.clone());
                for dep in &code.dependencies {
                    if !processed_packages.contains(dep) && !packages_to_process.contains(dep) {
                        packages_to_process.push(dep.clone());
                    }
                }
            }

            // Process dependencies recursively
            while let Some(package_name) = packages_to_process.pop() {
                if processed_packages.contains(&package_name) {
                    continue;
                }

                // Find all files for this package
                let dep_files = self.find_package_files(&package_name);

                // Generate files for this dependency package sequentially
                let dep_results: GeneratorResult<Vec<_>> = dep_files
                    .iter()
                    .map(|path| self.generate_single_file(path))
                    .collect();

                let dep_generated = dep_results?;

                // Add new dependencies to process
                for code in &dep_generated {
                    for dep in &code.dependencies {
                        if !processed_packages.contains(dep) && !packages_to_process.contains(dep) {
                            packages_to_process.push(dep.clone());
                        }
                    }
                }

                all_generated.extend(dep_generated);
                processed_packages.insert(package_name);
            }
        }

        Ok(all_generated)
    }

    /// Write generated files to disk with proper package structure
    fn write_generated_files(
        &self,
        output_dir: &Path,
        all_generated: &[GeneratedCode],
    ) -> GeneratorResult<()> {
        use std::collections::{HashMap, HashSet};

        // Group generated files by package and deduplicate by file path
        // This prevents the same file from being written multiple times if:
        // 1. A file appears in both input_files and as a dependency
        let mut packages: HashMap<String, Vec<&GeneratedCode>> = HashMap::new();
        let mut all_packages: HashSet<String> = HashSet::new();
        let mut seen_paths: HashSet<PathBuf> = HashSet::new();

        for code in all_generated {
            // Deduplicate: skip if we've already processed this file path
            if !seen_paths.insert(code.source_file.clone()) {
                continue;
            }

            if self.config.should_include_item(&code.module_name) {
                all_packages.insert(code.package_name.clone());
                packages
                    .entry(code.package_name.clone())
                    .or_default()
                    .push(code);
            }
        }

        // Write files in package subdirectories with msg/srv/action organization
        for (package_name, codes) in &packages {
            let package_dir = output_dir.join(package_name);
            std::fs::create_dir_all(&package_dir)?;

            // Group codes by interface kind (msg, srv, action)
            // This uses the semantic type from the content, not the file extension.
            // IDL files are placed in msg/srv/action based on what they contain.
            let mut type_groups: HashMap<String, Vec<&GeneratedCode>> = HashMap::new();
            for code in codes {
                type_groups
                    .entry(code.interface_kind.dir_name().to_string())
                    .or_default()
                    .push(code);
            }

            let mut submodule_names = Vec::new();

            // Write files organized by interface kind
            for (interface_dir, type_codes) in type_groups {
                let type_dir = package_dir.join(&interface_dir);
                std::fs::create_dir_all(&type_dir)?;

                // Get interface kind from the first code in the group
                // (all codes in a group have the same interface_kind)
                let interface_kind = type_codes
                    .first()
                    .map_or(super::InterfaceKind::Message, |c| c.interface_kind);

                let mut module_names = Vec::new();
                for code in type_codes {
                    let output_path = type_dir.join(format!("{}.rs", code.module_name));
                    code.write_to_file(output_path)?;
                    module_names.push(code.module_name.clone());
                }

                // Generate type-specific mod.rs (e.g., msg/mod.rs)
                if !module_names.is_empty() {
                    let type_mod_rs =
                        self.generate_type_mod_rs(package_name, interface_kind, &module_names);
                    std::fs::write(type_dir.join("mod.rs"), type_mod_rs)?;
                    submodule_names.push(interface_dir);
                }
            }

            // Generate package mod.rs that includes msg, srv, action submodules
            if !submodule_names.is_empty() {
                let package_mod_rs = self.generate_interface_mod_rs(package_name, &submodule_names);
                std::fs::write(package_dir.join("mod.rs"), package_mod_rs)?;
            }
        }

        // Generate root mod.rs
        if !all_packages.is_empty() {
            let mut package_list: Vec<_> = all_packages.into_iter().collect();
            package_list.sort();
            let root_mod_rs = self.generate_root_mod_rs(&package_list);
            std::fs::write(output_dir.join("mod.rs"), root_mod_rs)?;
        }

        Ok(())
    }

    /// Find all message/service/action/idl files for a given package
    fn find_package_files(&self, package_name: &str) -> Vec<PathBuf> {
        let mut files = Vec::new();

        // Search in configured package search paths
        for search_path in &self.config.package_search_paths {
            // Check if search_path already ends with package name or is a share directory
            let package_path = if search_path.ends_with(package_name) {
                search_path.clone()
            } else {
                search_path.join(package_name)
            };

            if !package_path.exists() {
                continue;
            }

            // Check msg, srv, action, and idl directories
            for subdir in &["msg", "srv", "action", "idl"] {
                let dir_path = package_path.join(subdir);
                if dir_path.exists()
                    && dir_path.is_dir()
                    && let Ok(entries) = std::fs::read_dir(&dir_path)
                {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file()
                            && let Some(ext) = path.extension()
                            && ext == *subdir
                        {
                            files.push(path);
                        }
                    }
                }
            }

            // If we found files in this search path, don't need to continue
            if !files.is_empty() {
                break;
            }
        }

        files
    }

    /// Generate code from a single file (internal helper)
    fn generate_single_file(&self, path: &Path) -> GeneratorResult<GeneratedCode> {
        if self.config.emit_rerun_if_changed {
            println!("cargo:rerun-if-changed={}", path.display());
        }

        CodeGenerator::new(self.config.clone()).generate_from_file(path)
    }

    /// Generate root mod.rs content (lists all packages)
    fn generate_root_mod_rs(&self, package_names: &[String]) -> String {
        use super::callbacks::{ModuleInfo, ModuleLevel};
        use std::fmt::Write;

        let mut content = String::new();

        if let Some(header) = &self.config.header {
            content.push_str(header);
            content.push_str("\n\n");
        }

        for raw in &self.config.raw_lines {
            content.push_str(raw);
            content.push('\n');
        }

        if !self.config.raw_lines.is_empty() {
            content.push('\n');
        }

        for package in package_names {
            let info = ModuleInfo::new(
                package.clone(),
                String::new(), // root has no parent
                package.clone(),
                ModuleLevel::Package,
            );

            // Pre-module callback
            if let Some(cb) = &self.config.parse_callbacks {
                if let Some(pre) = cb.pre_module(&info) {
                    content.push_str(&pre);
                    if !pre.ends_with('\n') {
                        content.push('\n');
                    }
                }
                if let Some(pre_tokens) = cb.pre_module_tokens(&info) {
                    content.push_str(&pre_tokens.to_string());
                    content.push('\n');
                }
            }

            let _ = writeln!(content, "pub mod {package};");

            // Post-module callback
            if let Some(cb) = &self.config.parse_callbacks {
                if let Some(post) = cb.post_module(&info) {
                    content.push_str(&post);
                    if !post.ends_with('\n') {
                        content.push('\n');
                    }
                }
                if let Some(post_tokens) = cb.post_module_tokens(&info) {
                    content.push_str(&post_tokens.to_string());
                    content.push('\n');
                }
            }
        }

        content
    }

    /// Generate package mod.rs content (lists interface kind modules like msg, srv, action)
    fn generate_interface_mod_rs(&self, package_name: &str, interface_names: &[String]) -> String {
        use super::callbacks::{ModuleInfo, ModuleLevel};
        use std::fmt::Write;

        let mut content = String::new();

        if let Some(header) = &self.config.header {
            content.push_str(header);
            content.push_str("\n\n");
        }

        for interface in interface_names {
            let interface_kind = match interface.as_str() {
                "srv" => super::InterfaceKind::Service,
                "action" => super::InterfaceKind::Action,
                _ => super::InterfaceKind::Message, // Default to message for unknown
            };

            let info = ModuleInfo::new(
                interface.clone(),
                package_name.to_string(),
                package_name.to_string(),
                ModuleLevel::InterfaceKind(interface_kind),
            );

            // Pre-module callback
            if let Some(cb) = &self.config.parse_callbacks {
                if let Some(pre) = cb.pre_module(&info) {
                    content.push_str(&pre);
                    if !pre.ends_with('\n') {
                        content.push('\n');
                    }
                }
                if let Some(pre_tokens) = cb.pre_module_tokens(&info) {
                    content.push_str(&pre_tokens.to_string());
                    content.push('\n');
                }
            }

            let _ = writeln!(content, "pub mod {interface};");

            // Post-module callback
            if let Some(cb) = &self.config.parse_callbacks {
                if let Some(post) = cb.post_module(&info) {
                    content.push_str(&post);
                    if !post.ends_with('\n') {
                        content.push('\n');
                    }
                }
                if let Some(post_tokens) = cb.post_module_tokens(&info) {
                    content.push_str(&post_tokens.to_string());
                    content.push('\n');
                }
            }
        }

        content
    }

    /// Generate type mod.rs content (lists individual message/service/action type modules)
    fn generate_type_mod_rs(
        &self,
        package_name: &str,
        interface_kind: super::InterfaceKind,
        module_names: &[String],
    ) -> String {
        use super::callbacks::{ModuleInfo, ModuleLevel};
        use std::fmt::Write;

        let mut content = String::new();

        if let Some(header) = &self.config.header {
            content.push_str(header);
            content.push_str("\n\n");
        }

        let parent_path = format!("{package_name}::{interface_kind}");

        for module in module_names {
            let info = ModuleInfo::new(
                module.clone(),
                parent_path.clone(),
                package_name.to_string(),
                ModuleLevel::Type(interface_kind),
            );

            // Pre-module callback
            if let Some(cb) = &self.config.parse_callbacks {
                if let Some(pre) = cb.pre_module(&info) {
                    content.push_str(&pre);
                    if !pre.ends_with('\n') {
                        content.push('\n');
                    }
                }
                if let Some(pre_tokens) = cb.pre_module_tokens(&info) {
                    content.push_str(&pre_tokens.to_string());
                    content.push('\n');
                }
            }

            let _ = writeln!(content, "pub mod {module};");

            // Post-module callback
            if let Some(cb) = &self.config.parse_callbacks {
                if let Some(post) = cb.post_module(&info) {
                    content.push_str(&post);
                    if !post.ends_with('\n') {
                        content.push('\n');
                    }
                }
                if let Some(post_tokens) = cb.post_module_tokens(&info) {
                    content.push_str(&post_tokens.to_string());
                    content.push('\n');
                }
            }
        }

        content
    }
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}
