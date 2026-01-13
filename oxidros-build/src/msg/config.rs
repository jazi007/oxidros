//! Configuration for ROS2 message code generation.
//!
//! This module provides the [`Config`] struct and [`ConfigBuilder`] for configuring
//! how ROS2 interface files are discovered and processed during code generation.

use std::env;
use std::path::PathBuf;

/// Supported ROS2 distributions for automatic path detection.
const SUPPORTED_DISTROS: &[&str] = &["humble", "jazzy", "kilted"];

/// Configuration for ROS2 message generation.
///
/// This struct holds all configuration options for finding and processing
/// ROS2 interface files (`.msg`, `.srv`, `.action`, `.idl`).
///
/// # Example
///
/// ```rust,ignore
/// use oxidros_build::msg::Config;
///
/// let config = Config::builder()
///     .packages(&["std_msgs", "geometry_msgs"])
///     .uuid_path("my_crate::unique_identifier_msgs")
///     .primitive_path("oxidros_msg")
///     .extra_search_path("/custom/ros2/share")
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Packages to generate types for. If empty, all packages are processed.
    pub(crate) packages: Vec<String>,
    /// Path prefix for unique_identifier_msgs (used for action types).
    pub(crate) uuid_path: Option<String>,
    /// Path prefix for primitive types (default: "oxidros_msg").
    pub(crate) primitive_path: Option<String>,
    /// Additional search paths for msg/srv/idl files.
    pub(crate) extra_search_paths: Vec<PathBuf>,
}

impl Config {
    /// Creates a new [`ConfigBuilder`] for constructing a [`Config`].
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Returns the packages to generate types for.
    pub fn packages(&self) -> &[String] {
        &self.packages
    }

    /// Returns the UUID path prefix for action types.
    pub fn uuid_path(&self) -> Option<&str> {
        self.uuid_path.as_deref()
    }

    /// Returns the primitive path prefix.
    pub fn primitive_path(&self) -> &str {
        self.primitive_path.as_deref().unwrap_or("oxidros_msg")
    }

    /// Returns all search paths for ROS2 interface files.
    ///
    /// This method combines paths from multiple sources in the following priority:
    ///
    /// 1. **AMENT_PREFIX_PATH environment variable** - If set and non-empty, these
    ///    paths are used first (standard ROS2 sourced environment).
    ///
    /// 2. **Common ROS2 installation paths** - If AMENT_PREFIX_PATH is not set,
    ///    the method looks for commonly used installation paths for supported
    ///    distributions (humble, jazzy, kilted) on both Linux and Windows.
    ///
    /// 3. **User-provided extra paths** - Additional paths specified via
    ///    [`ConfigBuilder::extra_search_path`] are always appended.
    ///
    /// # Returns
    ///
    /// A vector of existing paths where ROS2 packages may be found. Each path
    /// should contain a `share/` subdirectory with package interface files.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = Config::builder()
    ///     .extra_search_path("/my/custom/ros2")
    ///     .build();
    ///
    /// for path in config.get_search_paths() {
    ///     println!("Searching in: {}", path.display());
    /// }
    /// ```
    pub fn get_search_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Try AMENT_PREFIX_PATH first
        if let Some(ament_paths) = Self::get_ament_prefix_paths() {
            paths.extend(ament_paths);
        } else {
            // Fall back to common installation paths
            paths.extend(Self::get_default_ros2_paths());
        }

        // Add user-provided extra paths
        for extra in &self.extra_search_paths {
            if extra.exists() && !paths.contains(extra) {
                paths.push(extra.clone());
            }
        }

        paths
    }

    /// Parses the AMENT_PREFIX_PATH environment variable.
    ///
    /// Returns `None` if the variable is not set or is empty.
    fn get_ament_prefix_paths() -> Option<Vec<PathBuf>> {
        let path = env::var("AMENT_PREFIX_PATH").ok()?;
        if path.is_empty() {
            return None;
        }

        let mut paths: Vec<PathBuf> = env::split_paths(&path).filter(|p| p.exists()).collect();
        paths.sort();
        paths.dedup();

        if paths.is_empty() { None } else { Some(paths) }
    }

    /// Returns the first found common ROS2 installation path.
    ///
    /// Checks for the existence of standard installation directories on both
    /// Linux and Windows platforms, returning only the first one found.
    fn get_default_ros2_paths() -> Vec<PathBuf> {
        if cfg!(target_os = "linux") {
            // Linux: /opt/ros/<distro> - return first found
            for distro in SUPPORTED_DISTROS {
                let path = PathBuf::from(format!("/opt/ros/{}", distro));
                if path.exists() {
                    return vec![path];
                }
            }
        } else if cfg!(target_os = "windows") {
            // Windows: Common installation locations - return first found
            let windows_paths = [
                // Pixi workspace (jazzy)
                r"C:\pixi_ws\ros2-windows",
                // Common dev locations
                r"C:\dev\ros2_humble",
                r"C:\dev\ros2_jazzy",
                r"C:\dev\ros2_kilted",
                r"C:\ros2_humble",
                r"C:\ros2_jazzy",
                r"C:\ros2_kilted",
                // Program Files locations
                r"C:\Program Files\ros2_humble",
                r"C:\Program Files\ros2_jazzy",
                r"C:\Program Files\ros2_kilted",
            ];

            for path_str in windows_paths {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    return vec![path];
                }
            }
        }

        Vec::new()
    }

    /// Returns the share paths (paths with /share appended) for package discovery.
    ///
    /// This filters the search paths to only include those that have a valid
    /// `share/` subdirectory containing ROS2 packages.
    pub fn get_share_paths(&self) -> Vec<PathBuf> {
        self.get_search_paths()
            .into_iter()
            .filter_map(|p| {
                let share_path = p.join("share");
                if share_path.exists() {
                    Some(share_path)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the library search paths for linking ROS2 libraries.
    ///
    /// This method returns paths to `lib/` directories where ROS2 shared
    /// libraries are located. It combines:
    ///
    /// 1. **AMENT_PREFIX_PATH** - If set, uses `<path>/lib` (Unix) or `<path>/Lib` (Windows)
    /// 2. **CMAKE_PREFIX_PATH** (Windows only) - Uses `<path>/lib`
    /// 3. **Fallback paths** - If env vars not set, uses common installation paths
    /// 4. **Extra paths** - User-provided paths via config
    ///
    /// # Returns
    ///
    /// A vector of existing library directories suitable for `cargo:rustc-link-search`.
    pub fn get_lib_paths(&self) -> Vec<PathBuf> {
        let mut lib_paths = Vec::new();

        // Use AMENT_PREFIX_PATH and CMAKE_PREFIX_PATH (for Windows)
        if let Some(ament_paths) = Self::get_ament_prefix_paths() {
            for path in ament_paths {
                let lib_path = if cfg!(target_os = "windows") {
                    path.join("Lib")
                } else {
                    path.join("lib")
                };
                if lib_path.exists() && !lib_paths.contains(&lib_path) {
                    lib_paths.push(lib_path);
                }
            }
            // On Windows, also check CMAKE_PREFIX_PATH
            if cfg!(target_os = "windows")
                && let Some(cmake_paths) = Self::get_cmake_prefix_paths()
            {
                for path in cmake_paths {
                    let lib_path = path.join("lib");
                    if lib_path.exists() && !lib_paths.contains(&lib_path) {
                        lib_paths.push(lib_path);
                    }
                }
            }
        } else {
            // Use fallback common paths
            for path in Self::get_default_ros2_paths() {
                let lib_path = if cfg!(target_os = "windows") {
                    path.join("Lib")
                } else {
                    path.join("lib")
                };
                if lib_path.exists() && !lib_paths.contains(&lib_path) {
                    lib_paths.push(lib_path);
                }
            }
        }

        // Add user-provided extra paths
        for extra in &self.extra_search_paths {
            let lib_path = if cfg!(target_os = "windows") {
                extra.join("Lib")
            } else {
                extra.join("lib")
            };
            if lib_path.exists() && !lib_paths.contains(&lib_path) {
                lib_paths.push(lib_path);
            }
            // Also try lowercase lib on Windows as fallback
            if cfg!(target_os = "windows") {
                let lib_path_lower = extra.join("lib");
                if lib_path_lower.exists() && !lib_paths.contains(&lib_path_lower) {
                    lib_paths.push(lib_path_lower);
                }
            }
        }

        lib_paths
    }

    /// Parses the CMAKE_PREFIX_PATH environment variable (Windows only).
    ///
    /// Returns `None` if the variable is not set or is empty.
    fn get_cmake_prefix_paths() -> Option<Vec<PathBuf>> {
        let path = env::var("CMAKE_PREFIX_PATH").ok()?;
        if path.is_empty() {
            return None;
        }

        let mut paths: Vec<PathBuf> = env::split_paths(&path).filter(|p| p.exists()).collect();
        paths.sort();
        paths.dedup();

        if paths.is_empty() { None } else { Some(paths) }
    }

    pub(crate) fn print_packages_search_pathes(&self) {
        for lib_path in self.get_lib_paths() {
            println!("cargo:rustc-link-search={}", lib_path.display());
        }
    }
}

/// Builder for constructing a [`Config`] with a fluent API.
///
/// # Example
///
/// ```rust,ignore
/// use oxidros_build::msg::ConfigBuilder;
///
/// let config = ConfigBuilder::new()
///     .packages(&["std_msgs"])
///     .uuid_path("crate::unique_identifier_msgs")
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    packages: Vec<String>,
    uuid_path: Option<String>,
    primitive_path: Option<String>,
    extra_search_paths: Vec<PathBuf>,
}

impl ConfigBuilder {
    /// Creates a new [`ConfigBuilder`] with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the packages to generate types for.
    ///
    /// If not called or called with an empty slice, all packages found in
    /// the search paths will be processed.
    ///
    /// # Arguments
    ///
    /// * `packages` - Package names (e.g., `["std_msgs", "geometry_msgs"]`)
    pub fn packages(mut self, packages: &[&str]) -> Self {
        self.packages = packages.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Sets the path prefix for unique_identifier_msgs.
    ///
    /// This is used when generating action types that reference UUID types.
    ///
    /// # Arguments
    ///
    /// * `path` - The Rust path prefix (e.g., `"crate::unique_identifier_msgs"`)
    pub fn uuid_path(mut self, path: impl Into<String>) -> Self {
        self.uuid_path = Some(path.into());
        self
    }

    /// Sets the path prefix for primitive types.
    ///
    /// Default is `"oxidros_msg"` if not specified.
    ///
    /// # Arguments
    ///
    /// * `path` - The Rust path prefix for primitives
    pub fn primitive_path(mut self, path: impl Into<String>) -> Self {
        self.primitive_path = Some(path.into());
        self
    }

    /// Adds an extra search path for ROS2 interface files.
    ///
    /// This path will be searched in addition to AMENT_PREFIX_PATH or
    /// the default ROS2 installation paths.
    ///
    /// # Arguments
    ///
    /// * `path` - Additional path to search for packages
    pub fn extra_search_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.extra_search_paths.push(path.into());
        self
    }

    /// Adds multiple extra search paths for ROS2 interface files.
    ///
    /// # Arguments
    ///
    /// * `paths` - Additional paths to search for packages
    pub fn extra_search_paths(mut self, paths: impl IntoIterator<Item = PathBuf>) -> Self {
        self.extra_search_paths.extend(paths);
        self
    }

    /// Builds the [`Config`] with the specified options.
    pub fn build(self) -> Config {
        Config {
            packages: self.packages,
            uuid_path: self.uuid_path,
            primitive_path: self.primitive_path,
            extra_search_paths: self.extra_search_paths,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder_default() {
        let config = Config::builder().build();
        assert!(config.packages.is_empty());
        assert!(config.uuid_path.is_none());
        assert_eq!(config.primitive_path(), "oxidros_msg");
        assert!(config.extra_search_paths.is_empty());
    }

    #[test]
    fn test_config_builder_with_packages() {
        let config = Config::builder()
            .packages(&["std_msgs", "geometry_msgs"])
            .build();
        assert_eq!(config.packages, vec!["std_msgs", "geometry_msgs"]);
    }

    #[test]
    fn test_config_builder_with_paths() {
        let config = Config::builder()
            .uuid_path("my::uuid::path")
            .primitive_path("my::primitive")
            .build();
        assert_eq!(config.uuid_path(), Some("my::uuid::path"));
        assert_eq!(config.primitive_path(), "my::primitive");
    }

    #[test]
    fn test_config_builder_with_extra_paths() {
        let config = Config::builder()
            .extra_search_path("/custom/path1")
            .extra_search_path("/custom/path2")
            .build();
        assert_eq!(config.extra_search_paths.len(), 2);
    }
}
