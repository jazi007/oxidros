//! ROS2 naming validation for nodes and topics
//!
//! This module provides validation functions for ROS2 names according to the
//! [ROS2 Topic and Service Names](https://design.ros2.org/articles/topic_and_service_names.html)
//! specification.
//!
//! # ROS2 Naming Rules
//!
//! ## Topic and Service Names
//!
//! - Must not be empty
//! - May contain alphanumeric characters (`[0-9|a-z|A-Z]`), underscores (`_`), or forward slashes (`/`)
//! - May use balanced curly braces (`{}`) for substitutions
//! - May start with a tilde (`~`), the private namespace substitution character
//! - Must not start with a numeric character (`[0-9]`)
//! - Must not end with a forward slash (`/`)
//! - Must not contain repeated forward slashes (`//`)
//! - Must not contain repeated underscores (`__`)
//! - Must separate a tilde (`~`) from the rest of the name with a forward slash (`/`)
//!
//! ## Node Names (Base Names)
//!
//! - Must not be empty
//! - May contain alphanumeric characters (`[0-9|a-z|A-Z]`) and underscores (`_`)
//! - Must not start with a numeric character (`[0-9]`)
//! - Must not contain forward slashes (`/`) or tildes (`~`)
//! - Must not contain repeated underscores (`__`)
//!
//! ## Namespace Names
//!
//! - Must start with a forward slash (`/`) if absolute
//! - Follow similar rules to topic names but cannot contain tilde or substitutions
//!
//! # Examples
//!
//! ```
//! use ros2args::names::{validate_topic_name, validate_node_name, validate_namespace};
//!
//! // Valid topic names
//! assert!(validate_topic_name("foo").is_ok());
//! assert!(validate_topic_name("/foo/bar").is_ok());
//! assert!(validate_topic_name("~/foo").is_ok());
//! assert!(validate_topic_name("foo_bar").is_ok());
//!
//! // Invalid topic names
//! assert!(validate_topic_name("").is_err());        // empty
//! assert!(validate_topic_name("123abc").is_err());  // starts with number
//! assert!(validate_topic_name("foo//bar").is_err()); // double slash
//! assert!(validate_topic_name("foo__bar").is_err()); // double underscore
//!
//! // Valid node names
//! assert!(validate_node_name("my_node").is_ok());
//! assert!(validate_node_name("node123").is_ok());
//!
//! // Invalid node names
//! assert!(validate_node_name("my/node").is_err()); // contains slash
//! assert!(validate_node_name("~node").is_err());   // contains tilde
//! ```

use crate::errors::{Ros2ArgsError, Ros2ArgsResult};

/// Represents what kind of name is being validated
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    /// A topic or service name
    Topic,
    /// A node base name (no namespace)
    Node,
    /// A namespace
    Namespace,
    /// A substitution content (inside curly braces)
    Substitution,
}

impl std::fmt::Display for NameKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Topic => write!(f, "topic"),
            Self::Node => write!(f, "node"),
            Self::Namespace => write!(f, "namespace"),
            Self::Substitution => write!(f, "substitution"),
        }
    }
}

/// Check if a character is valid for ROS2 names
///
/// Valid characters are alphanumeric characters and underscores.
#[inline]
#[must_use]
pub fn is_valid_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Check if a character is valid for topic/service names (includes `/`)
#[inline]
#[must_use]
pub fn is_valid_topic_char(c: char) -> bool {
    is_valid_name_char(c) || c == '/'
}

/// Validate a ROS2 topic or service name
///
/// # Rules
///
/// - Must not be empty
/// - May contain alphanumeric characters, underscores, or forward slashes
/// - May use balanced curly braces for substitutions
/// - May start with a tilde (~) for private namespace substitution
/// - Must not start with a numeric character
/// - Must not end with a forward slash
/// - Must not contain repeated forward slashes
/// - Must not contain repeated underscores
/// - Tilde must be followed by a forward slash if not alone
///
/// # Errors
///
/// Returns `Ros2ArgsError::InvalidName` if the name violates any of the above rules.
///
/// # Examples
///
/// ```
/// use ros2args::names::validate_topic_name;
///
/// assert!(validate_topic_name("foo").is_ok());
/// assert!(validate_topic_name("/foo/bar").is_ok());
/// assert!(validate_topic_name("~/private").is_ok());
/// assert!(validate_topic_name("{node}/topic").is_ok());
///
/// assert!(validate_topic_name("").is_err());
/// assert!(validate_topic_name("123").is_err());
/// assert!(validate_topic_name("foo//bar").is_err());
/// ```
pub fn validate_topic_name(name: &str) -> Ros2ArgsResult<()> {
    validate_name_impl(name, NameKind::Topic)
}

/// Validate a ROS2 node base name
///
/// Node names have stricter rules than topic names:
/// - Must not be empty
/// - May contain alphanumeric characters and underscores only
/// - Must not start with a numeric character
/// - Must not contain forward slashes, tildes, or curly braces
/// - Must not contain repeated underscores
///
/// # Errors
///
/// Returns `Ros2ArgsError::InvalidName` if the name violates any of the above rules.
///
/// # Examples
///
/// ```
/// use ros2args::names::validate_node_name;
///
/// assert!(validate_node_name("my_node").is_ok());
/// assert!(validate_node_name("node123").is_ok());
/// assert!(validate_node_name("MyNode").is_ok());
///
/// assert!(validate_node_name("").is_err());
/// assert!(validate_node_name("my/node").is_err());
/// assert!(validate_node_name("123node").is_err());
/// ```
pub fn validate_node_name(name: &str) -> Ros2ArgsResult<()> {
    validate_name_impl(name, NameKind::Node)
}

/// Validate a ROS2 namespace
///
/// Namespace rules:
/// - Must not be empty
/// - Must start with a forward slash if absolute
/// - May contain alphanumeric characters, underscores, and forward slashes
/// - Must not start with a numeric character (after the leading slash)
/// - Must not end with a forward slash (unless it's just "/")
/// - Must not contain repeated forward slashes
/// - Must not contain repeated underscores
///
/// # Errors
///
/// Returns `Ros2ArgsError::InvalidName` if the namespace violates any of the above rules.
///
/// # Examples
///
/// ```
/// use ros2args::names::validate_namespace;
///
/// assert!(validate_namespace("/").is_ok());
/// assert!(validate_namespace("/foo").is_ok());
/// assert!(validate_namespace("/foo/bar").is_ok());
///
/// assert!(validate_namespace("").is_err());
/// assert!(validate_namespace("/foo/").is_err());
/// assert!(validate_namespace("/foo//bar").is_err());
/// ```
pub fn validate_namespace(namespace: &str) -> Ros2ArgsResult<()> {
    validate_name_impl(namespace, NameKind::Namespace)
}

/// Validate a substitution name (content inside curly braces)
///
/// Substitution rules:
/// - Must not be empty
/// - May contain alphanumeric characters and underscores
/// - Must not start with a numeric character
///
/// # Errors
///
/// Returns `Ros2ArgsError::InvalidName` if the substitution name violates any of the above rules.
///
/// # Examples
///
/// ```
/// use ros2args::names::validate_substitution;
///
/// assert!(validate_substitution("node").is_ok());
/// assert!(validate_substitution("namespace").is_ok());
///
/// assert!(validate_substitution("").is_err());
/// assert!(validate_substitution("123").is_err());
/// ```
pub fn validate_substitution(name: &str) -> Ros2ArgsResult<()> {
    validate_name_impl(name, NameKind::Substitution)
}

/// Internal implementation for name validation
#[allow(clippy::too_many_lines)]
fn validate_name_impl(name: &str, kind: NameKind) -> Ros2ArgsResult<()> {
    // Rule: Must not be empty
    if name.is_empty() {
        return Err(Ros2ArgsError::InvalidName {
            kind,
            name: name.to_string(),
            reason: "name must not be empty".to_string(),
        });
    }

    let chars: Vec<char> = name.chars().collect();
    let mut i = 0;

    // Handle leading characters based on name kind
    match kind {
        NameKind::Topic => {
            // Topics can start with '/', '~', '{', or an alpha/underscore
            if chars[0] == '~' {
                // Tilde must be alone or followed by '/'
                if chars.len() > 1 && chars[1] != '/' {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "tilde (~) must be followed by a forward slash (/)".to_string(),
                    });
                }
                i = 1;
            } else if chars[0] == '/' {
                i = 1;
            } else if chars[0] == '{' {
                // Will be validated in the main loop
            } else if chars[0].is_ascii_digit() {
                return Err(Ros2ArgsError::InvalidName {
                    kind,
                    name: name.to_string(),
                    reason: "name must not start with a numeric character".to_string(),
                });
            } else if !chars[0].is_ascii_alphabetic() && chars[0] != '_' {
                return Err(Ros2ArgsError::InvalidName {
                    kind,
                    name: name.to_string(),
                    reason: format!("invalid character '{}' at position 0", chars[0]),
                });
            }
        }
        NameKind::Namespace => {
            // Namespaces must start with '/'
            if chars[0] != '/' {
                return Err(Ros2ArgsError::InvalidName {
                    kind,
                    name: name.to_string(),
                    reason: "namespace must start with a forward slash (/)".to_string(),
                });
            }
            // Root namespace "/" is valid
            if name == "/" {
                return Ok(());
            }
            i = 1;
            // Check that first character after '/' is not a digit
            if i < chars.len() && chars[i].is_ascii_digit() {
                return Err(Ros2ArgsError::InvalidName {
                    kind,
                    name: name.to_string(),
                    reason: "namespace token must not start with a numeric character".to_string(),
                });
            }
        }
        NameKind::Node | NameKind::Substitution => {
            // Node names and substitutions must start with alpha or underscore
            if chars[0].is_ascii_digit() {
                return Err(Ros2ArgsError::InvalidName {
                    kind,
                    name: name.to_string(),
                    reason: "name must not start with a numeric character".to_string(),
                });
            }
            if !is_valid_name_char(chars[0]) {
                return Err(Ros2ArgsError::InvalidName {
                    kind,
                    name: name.to_string(),
                    reason: format!("invalid character '{}' at position 0", chars[0]),
                });
            }
        }
    }

    // Track curly brace balance for substitutions (only in topic names)
    let mut brace_depth = 0;
    let mut prev_char: Option<char> = if i > 0 { Some(chars[i - 1]) } else { None };

    while i < chars.len() {
        let c = chars[i];

        match kind {
            NameKind::Topic => {
                if c == '{' {
                    brace_depth += 1;
                } else if c == '}' {
                    if brace_depth == 0 {
                        return Err(Ros2ArgsError::InvalidName {
                            kind,
                            name: name.to_string(),
                            reason: "unbalanced curly braces: unexpected '}'".to_string(),
                        });
                    }
                    brace_depth -= 1;
                } else if brace_depth > 0 {
                    // Inside substitution: only alphanumeric and underscore allowed
                    if !is_valid_name_char(c) {
                        return Err(Ros2ArgsError::InvalidName {
                            kind,
                            name: name.to_string(),
                            reason: format!(
                                "invalid character '{c}' inside substitution at position {i}"
                            ),
                        });
                    }
                } else if !is_valid_topic_char(c) {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: format!("invalid character '{c}' at position {i}"),
                    });
                }

                // Check for repeated slashes
                if c == '/' && prev_char == Some('/') {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "name must not contain repeated forward slashes (//)".to_string(),
                    });
                }

                // Check for repeated underscores
                if c == '_' && prev_char == Some('_') {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "name must not contain repeated underscores (__)".to_string(),
                    });
                }

                // Check that tilde only appears at the start
                if c == '~' {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "tilde (~) may only appear at the beginning of a name".to_string(),
                    });
                }

                // Check that tokens after '/' don't start with a digit
                if prev_char == Some('/') && c.is_ascii_digit() {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: format!(
                            "token after '/' must not start with a numeric character at position {i}"
                        ),
                    });
                }
            }
            NameKind::Namespace => {
                if !is_valid_topic_char(c) {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: format!("invalid character '{c}' at position {i}"),
                    });
                }

                // Check for repeated slashes
                if c == '/' && prev_char == Some('/') {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "namespace must not contain repeated forward slashes (//)"
                            .to_string(),
                    });
                }

                // Check for repeated underscores
                if c == '_' && prev_char == Some('_') {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "namespace must not contain repeated underscores (__)".to_string(),
                    });
                }

                // Check that tokens after '/' don't start with a digit
                if prev_char == Some('/') && c.is_ascii_digit() {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: format!(
                            "namespace token after '/' must not start with a numeric character at position {i}"
                        ),
                    });
                }
            }
            NameKind::Node => {
                // Node names cannot contain '/', '~', or '{}'
                if c == '/' {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "node name must not contain forward slash (/)".to_string(),
                    });
                }
                if c == '~' {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "node name must not contain tilde (~)".to_string(),
                    });
                }
                if c == '{' || c == '}' {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "node name must not contain curly braces".to_string(),
                    });
                }
                if !is_valid_name_char(c) {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: format!("invalid character '{c}' at position {i}"),
                    });
                }

                // Check for repeated underscores
                if c == '_' && prev_char == Some('_') {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: "node name must not contain repeated underscores (__)".to_string(),
                    });
                }
            }
            NameKind::Substitution => {
                if !is_valid_name_char(c) {
                    return Err(Ros2ArgsError::InvalidName {
                        kind,
                        name: name.to_string(),
                        reason: format!("invalid character '{c}' at position {i}"),
                    });
                }
            }
        }

        prev_char = Some(c);
        i += 1;
    }

    // Final checks
    match kind {
        NameKind::Topic => {
            // Check for unbalanced braces
            if brace_depth != 0 {
                return Err(Ros2ArgsError::InvalidName {
                    kind,
                    name: name.to_string(),
                    reason: "unbalanced curly braces: missing '}'".to_string(),
                });
            }

            // Check for trailing slash
            if name.ends_with('/') {
                return Err(Ros2ArgsError::InvalidName {
                    kind,
                    name: name.to_string(),
                    reason: "name must not end with a forward slash (/)".to_string(),
                });
            }
        }
        NameKind::Namespace => {
            // Check for trailing slash (except for root namespace)
            if name.len() > 1 && name.ends_with('/') {
                return Err(Ros2ArgsError::InvalidName {
                    kind,
                    name: name.to_string(),
                    reason: "namespace must not end with a forward slash (/)".to_string(),
                });
            }
        }
        NameKind::Node | NameKind::Substitution => {}
    }

    Ok(())
}

/// Validate a fully qualified name (absolute topic or service name)
///
/// Fully qualified names have additional restrictions:
/// - Must start with a forward slash (/)
/// - Must not contain tilde (~) or curly braces ({})
///
/// # Errors
///
/// Returns `Ros2ArgsError::InvalidName` if the name is not a valid fully qualified name.
///
/// # Examples
///
/// ```
/// use ros2args::names::validate_fully_qualified_name;
///
/// assert!(validate_fully_qualified_name("/foo").is_ok());
/// assert!(validate_fully_qualified_name("/foo/bar").is_ok());
///
/// assert!(validate_fully_qualified_name("foo").is_err());      // not absolute
/// assert!(validate_fully_qualified_name("/~foo").is_err());    // contains tilde
/// assert!(validate_fully_qualified_name("/{sub}").is_err());   // contains substitution
/// ```
pub fn validate_fully_qualified_name(name: &str) -> Ros2ArgsResult<()> {
    if name.is_empty() {
        return Err(Ros2ArgsError::InvalidName {
            kind: NameKind::Topic,
            name: name.to_string(),
            reason: "fully qualified name must not be empty".to_string(),
        });
    }

    if !name.starts_with('/') {
        return Err(Ros2ArgsError::InvalidName {
            kind: NameKind::Topic,
            name: name.to_string(),
            reason: "fully qualified name must start with a forward slash (/)".to_string(),
        });
    }

    if name.contains('~') {
        return Err(Ros2ArgsError::InvalidName {
            kind: NameKind::Topic,
            name: name.to_string(),
            reason: "fully qualified name must not contain tilde (~)".to_string(),
        });
    }

    if name.contains('{') || name.contains('}') {
        return Err(Ros2ArgsError::InvalidName {
            kind: NameKind::Topic,
            name: name.to_string(),
            reason: "fully qualified name must not contain curly braces ({})".to_string(),
        });
    }

    // Validate as a topic name
    validate_topic_name(name)
}

/// Check if a name is a relative name (does not start with '/' or '~')
#[inline]
#[must_use]
pub fn is_relative_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('/') && !name.starts_with('~')
}

/// Check if a name is an absolute name (starts with '/')
#[inline]
#[must_use]
pub fn is_absolute_name(name: &str) -> bool {
    name.starts_with('/')
}

/// Check if a name is a private name (starts with '~')
#[inline]
#[must_use]
pub fn is_private_name(name: &str) -> bool {
    name.starts_with('~')
}

/// Check if a name is hidden (contains a token starting with '_')
///
/// Any topic or service name that contains tokens starting with an underscore
/// is considered hidden.
#[must_use]
pub fn is_hidden_name(name: &str) -> bool {
    // Check if any token starts with underscore
    name.starts_with('_')
        || name.contains("/_")
        || name
            .split('/')
            .any(|token| !token.is_empty() && token.starts_with('_'))
}

/// Expand a topic name to its fully qualified form
///
/// This function takes a node's namespace, node name, and a topic name,
/// and returns the fully qualified topic name after expanding special
/// characters and handling relative names.
///
/// # Expansion Rules
///
/// - **Absolute names** (starting with `/`): Returned as-is, ignoring the node's namespace
/// - **Private names** (starting with `~`): The `~` is replaced with the node's FQN
///   (namespace + node name), e.g., `~/foo` becomes `/my_ns/my_node/foo`
/// - **Relative names**: Prefixed with the node's namespace,
///   e.g., `foo` becomes `/my_ns/foo`
///
/// # Arguments
///
/// * `node_namespace` - The node's namespace (must start with `/`)
/// * `node_name` - The node's base name (without namespace)
/// * `topic_name` - The topic name to expand
///
/// # Errors
///
/// Returns an error if:
/// - The node namespace is invalid
/// - The node name is invalid
/// - The resulting topic name is invalid
///
/// # Examples
///
/// ```
/// use ros2args::names::expand_topic_name;
///
/// // Absolute topic - returned as-is
/// let fqn = expand_topic_name("/my_ns", "my_node", "/absolute/topic").unwrap();
/// assert_eq!(fqn, "/absolute/topic");
///
/// // Private topic - ~ replaced with node FQN
/// let fqn = expand_topic_name("/my_ns", "my_node", "~/private").unwrap();
/// assert_eq!(fqn, "/my_ns/my_node/private");
///
/// // Just tilde - expands to node FQN
/// let fqn = expand_topic_name("/my_ns", "my_node", "~").unwrap();
/// assert_eq!(fqn, "/my_ns/my_node");
///
/// // Relative topic - prefixed with namespace
/// let fqn = expand_topic_name("/my_ns", "my_node", "relative/topic").unwrap();
/// assert_eq!(fqn, "/my_ns/relative/topic");
///
/// // Root namespace
/// let fqn = expand_topic_name("/", "my_node", "~/private").unwrap();
/// assert_eq!(fqn, "/my_node/private");
///
/// // Root namespace with relative topic
/// let fqn = expand_topic_name("/", "my_node", "relative").unwrap();
/// assert_eq!(fqn, "/relative");
/// ```
pub fn expand_topic_name(
    node_namespace: &str,
    node_name: &str,
    topic_name: &str,
) -> Ros2ArgsResult<String> {
    // Validate inputs
    validate_namespace(node_namespace)?;
    validate_node_name(node_name)?;
    validate_topic_name(topic_name)?;

    let expanded = if is_absolute_name(topic_name) {
        // Absolute names are returned as-is
        topic_name.to_string()
    } else if is_private_name(topic_name) {
        // Private names: replace ~ with node's FQN
        let node_fqn = build_node_fqn(node_namespace, node_name);
        if topic_name == "~" {
            node_fqn
        } else {
            // topic_name is "~/something", strip the "~" and append
            format!("{}{}", node_fqn, &topic_name[1..])
        }
    } else {
        // Relative names: prefix with namespace
        if node_namespace == "/" {
            format!("/{topic_name}")
        } else {
            format!("{node_namespace}/{topic_name}")
        }
    };

    // Validate the resulting FQN
    validate_fully_qualified_name(&expanded)?;

    Ok(expanded)
}

/// Build the fully qualified node name from namespace and node name
///
/// # Examples
///
/// ```
/// use ros2args::names::build_node_fqn;
///
/// assert_eq!(build_node_fqn("/my_ns", "my_node"), "/my_ns/my_node");
/// assert_eq!(build_node_fqn("/", "my_node"), "/my_node");
/// assert_eq!(build_node_fqn("/foo/bar", "node"), "/foo/bar/node");
/// ```
#[must_use]
pub fn build_node_fqn(namespace: &str, node_name: &str) -> String {
    if namespace == "/" {
        format!("/{node_name}")
    } else {
        format!("{namespace}/{node_name}")
    }
}

/// Expand a topic name using a pre-built node FQN
///
/// This is a convenience function when you already have the node's fully
/// qualified name. It performs the same expansion as [`expand_topic_name`]
/// but takes the node FQN directly.
///
/// # Arguments
///
/// * `node_fqn` - The node's fully qualified name (e.g., `/my_ns/my_node`)
/// * `topic_name` - The topic name to expand
///
/// # Errors
///
/// Returns an error if:
/// - The node FQN is invalid
/// - The topic name is invalid
/// - The resulting topic name is invalid
///
/// # Examples
///
/// ```
/// use ros2args::names::expand_topic_name_with_fqn;
///
/// let fqn = expand_topic_name_with_fqn("/my_ns/my_node", "~/private").unwrap();
/// assert_eq!(fqn, "/my_ns/my_node/private");
///
/// let fqn = expand_topic_name_with_fqn("/my_ns/my_node", "/absolute").unwrap();
/// assert_eq!(fqn, "/absolute");
///
/// let fqn = expand_topic_name_with_fqn("/my_ns/my_node", "relative").unwrap();
/// assert_eq!(fqn, "/my_ns/relative");
/// ```
pub fn expand_topic_name_with_fqn(node_fqn: &str, topic_name: &str) -> Ros2ArgsResult<String> {
    // Validate node FQN
    validate_fully_qualified_name(node_fqn)?;
    validate_topic_name(topic_name)?;

    // Extract namespace from node FQN (everything except the last token)
    let node_namespace = extract_namespace(node_fqn);

    let expanded = if is_absolute_name(topic_name) {
        // Absolute names are returned as-is
        topic_name.to_string()
    } else if is_private_name(topic_name) {
        // Private names: replace ~ with node's FQN
        if topic_name == "~" {
            node_fqn.to_string()
        } else {
            // topic_name is "~/something", strip the "~" and append
            format!("{node_fqn}{}", &topic_name[1..])
        }
    } else {
        // Relative names: prefix with namespace
        if node_namespace == "/" {
            format!("/{topic_name}")
        } else {
            format!("{node_namespace}/{topic_name}")
        }
    };

    // Validate the resulting FQN
    validate_fully_qualified_name(&expanded)?;

    Ok(expanded)
}

/// Extract the namespace from a fully qualified node name
///
/// Returns the namespace portion of a node FQN (everything before the last token).
///
/// # Examples
///
/// ```
/// use ros2args::names::extract_namespace;
///
/// assert_eq!(extract_namespace("/my_ns/my_node"), "/my_ns");
/// assert_eq!(extract_namespace("/my_node"), "/");
/// assert_eq!(extract_namespace("/foo/bar/baz"), "/foo/bar");
/// ```
#[must_use]
pub fn extract_namespace(node_fqn: &str) -> &str {
    if let Some(last_slash_pos) = node_fqn.rfind('/') {
        if last_slash_pos == 0 {
            "/"
        } else {
            &node_fqn[..last_slash_pos]
        }
    } else {
        "/"
    }
}

/// Extract the base name from a fully qualified node name
///
/// Returns the base name portion of a node FQN (the last token after the final `/`).
///
/// # Examples
///
/// ```
/// use ros2args::names::extract_base_name;
///
/// assert_eq!(extract_base_name("/my_ns/my_node"), "my_node");
/// assert_eq!(extract_base_name("/my_node"), "my_node");
/// assert_eq!(extract_base_name("/foo/bar/baz"), "baz");
/// ```
#[must_use]
pub fn extract_base_name(node_fqn: &str) -> &str {
    if let Some(last_slash_pos) = node_fqn.rfind('/') {
        &node_fqn[last_slash_pos + 1..]
    } else {
        node_fqn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Topic Name Tests ====================

    #[test]
    fn test_valid_topic_names() {
        let valid_names = [
            "foo",
            "bar",
            "abc123",
            "_foo",
            "Foo",
            "BAR",
            "foo/bar",
            "/foo",
            "/foo/bar",
            "~",
            "~/foo",
            "~/foo/bar",
            "{foo}_bar",
            "foo/{ping}/bar",
            "foo/_bar",
            "foo_/bar",
            "foo_",
        ];

        for name in &valid_names {
            assert!(
                validate_topic_name(name).is_ok(),
                "Expected '{name}' to be valid",
            );
        }
    }

    #[test]
    fn test_invalid_topic_names_empty() {
        assert!(validate_topic_name("").is_err());
    }

    #[test]
    fn test_invalid_topic_names_start_with_number() {
        assert!(validate_topic_name("123abc").is_err());
        assert!(validate_topic_name("123").is_err());
    }

    #[test]
    fn test_invalid_topic_names_double_slash() {
        assert!(validate_topic_name("foo//bar").is_err());
        assert!(validate_topic_name("//foo").is_err());
    }

    #[test]
    fn test_invalid_topic_names_double_underscore() {
        assert!(validate_topic_name("foo__bar").is_err());
    }

    #[test]
    fn test_invalid_topic_names_tilde_not_at_start() {
        assert!(validate_topic_name("/~").is_err());
        assert!(validate_topic_name("foo~").is_err());
        assert!(validate_topic_name("foo~/bar").is_err());
        assert!(validate_topic_name("foo/~bar").is_err());
        assert!(validate_topic_name("foo/~/bar").is_err());
    }

    #[test]
    fn test_invalid_topic_names_tilde_not_followed_by_slash() {
        assert!(validate_topic_name("~foo").is_err());
    }

    #[test]
    fn test_invalid_topic_names_trailing_slash() {
        assert!(validate_topic_name("foo/").is_err());
        assert!(validate_topic_name("/foo/bar/").is_err());
    }

    #[test]
    fn test_invalid_topic_names_space() {
        assert!(validate_topic_name("foo bar").is_err());
        assert!(validate_topic_name(" ").is_err());
    }

    #[test]
    fn test_invalid_topic_names_unbalanced_braces() {
        assert!(validate_topic_name("{foo").is_err());
        assert!(validate_topic_name("foo}").is_err());
        assert!(validate_topic_name("{foo/bar").is_err());
    }

    // ==================== Node Name Tests ====================

    #[test]
    fn test_valid_node_names() {
        let valid_names = [
            "my_node",
            "node123",
            "MyNode",
            "NODE",
            "_private_node",
            "node_",
            "a",
            "A",
        ];

        for name in &valid_names {
            assert!(
                validate_node_name(name).is_ok(),
                "Expected '{name}' to be valid node name",
            );
        }
    }

    #[test]
    fn test_invalid_node_names_empty() {
        assert!(validate_node_name("").is_err());
    }

    #[test]
    fn test_invalid_node_names_start_with_number() {
        assert!(validate_node_name("123node").is_err());
        assert!(validate_node_name("1").is_err());
    }

    #[test]
    fn test_invalid_node_names_contains_slash() {
        assert!(validate_node_name("my/node").is_err());
        assert!(validate_node_name("/node").is_err());
        assert!(validate_node_name("node/").is_err());
    }

    #[test]
    fn test_invalid_node_names_contains_tilde() {
        assert!(validate_node_name("~node").is_err());
        assert!(validate_node_name("node~").is_err());
        assert!(validate_node_name("my~node").is_err());
    }

    #[test]
    fn test_invalid_node_names_contains_braces() {
        assert!(validate_node_name("{node}").is_err());
        assert!(validate_node_name("node{").is_err());
        assert!(validate_node_name("}node").is_err());
    }

    #[test]
    fn test_invalid_node_names_double_underscore() {
        assert!(validate_node_name("my__node").is_err());
    }

    #[test]
    fn test_invalid_node_names_special_chars() {
        assert!(validate_node_name("my-node").is_err());
        assert!(validate_node_name("my.node").is_err());
        assert!(validate_node_name("my node").is_err());
        assert!(validate_node_name("my@node").is_err());
    }

    // ==================== Namespace Tests ====================

    #[test]
    fn test_valid_namespaces() {
        let valid_namespaces = [
            "/",
            "/foo",
            "/foo/bar",
            "/foo/bar/baz",
            "/my_namespace",
            "/_private",
        ];

        for ns in &valid_namespaces {
            assert!(
                validate_namespace(ns).is_ok(),
                "Expected '{ns}' to be valid namespace",
            );
        }
    }

    #[test]
    fn test_invalid_namespace_empty() {
        assert!(validate_namespace("").is_err());
    }

    #[test]
    fn test_invalid_namespace_not_starting_with_slash() {
        assert!(validate_namespace("foo").is_err());
        assert!(validate_namespace("foo/bar").is_err());
    }

    #[test]
    fn test_invalid_namespace_trailing_slash() {
        assert!(validate_namespace("/foo/").is_err());
        assert!(validate_namespace("/foo/bar/").is_err());
    }

    #[test]
    fn test_invalid_namespace_double_slash() {
        assert!(validate_namespace("//foo").is_err());
        assert!(validate_namespace("/foo//bar").is_err());
    }

    #[test]
    fn test_invalid_namespace_double_underscore() {
        assert!(validate_namespace("/foo__bar").is_err());
    }

    #[test]
    fn test_invalid_namespace_token_starts_with_number() {
        assert!(validate_namespace("/123").is_err());
        assert!(validate_namespace("/foo/123bar").is_err());
    }

    // ==================== Fully Qualified Name Tests ====================

    #[test]
    fn test_valid_fully_qualified_names() {
        let valid_names = [
            "/foo",
            "/bar/baz",
            "/_private/thing",
            "/public_namespace/_private/thing",
        ];

        for name in &valid_names {
            assert!(
                validate_fully_qualified_name(name).is_ok(),
                "Expected '{name}' to be valid FQN",
            );
        }
    }

    #[test]
    fn test_invalid_fqn_not_absolute() {
        assert!(validate_fully_qualified_name("foo").is_err());
        assert!(validate_fully_qualified_name("foo/bar").is_err());
    }

    #[test]
    fn test_invalid_fqn_contains_tilde() {
        assert!(validate_fully_qualified_name("/~").is_err());
        assert!(validate_fully_qualified_name("/~/foo").is_err());
    }

    #[test]
    fn test_invalid_fqn_contains_substitution() {
        assert!(validate_fully_qualified_name("/{sub}").is_err());
        assert!(validate_fully_qualified_name("/foo/{bar}").is_err());
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_is_relative_name() {
        assert!(is_relative_name("foo"));
        assert!(is_relative_name("foo/bar"));
        assert!(!is_relative_name("/foo"));
        assert!(!is_relative_name("~"));
        assert!(!is_relative_name("~/foo"));
        assert!(!is_relative_name(""));
    }

    #[test]
    fn test_is_absolute_name() {
        assert!(is_absolute_name("/foo"));
        assert!(is_absolute_name("/"));
        assert!(!is_absolute_name("foo"));
        assert!(!is_absolute_name("~"));
    }

    #[test]
    fn test_is_private_name() {
        assert!(is_private_name("~"));
        assert!(is_private_name("~/foo"));
        assert!(!is_private_name("/foo"));
        assert!(!is_private_name("foo"));
    }

    #[test]
    fn test_is_hidden_name() {
        assert!(is_hidden_name("_foo"));
        assert!(is_hidden_name("/foo/_bar"));
        assert!(is_hidden_name("/_private/thing"));
        assert!(!is_hidden_name("foo"));
        assert!(!is_hidden_name("/foo/bar"));
        assert!(!is_hidden_name("foo_bar"));
    }

    #[test]
    fn test_is_valid_name_char() {
        assert!(is_valid_name_char('a'));
        assert!(is_valid_name_char('Z'));
        assert!(is_valid_name_char('5'));
        assert!(is_valid_name_char('_'));
        assert!(!is_valid_name_char('/'));
        assert!(!is_valid_name_char('-'));
        assert!(!is_valid_name_char(' '));
    }

    #[test]
    fn test_is_valid_topic_char() {
        assert!(is_valid_topic_char('a'));
        assert!(is_valid_topic_char('Z'));
        assert!(is_valid_topic_char('5'));
        assert!(is_valid_topic_char('_'));
        assert!(is_valid_topic_char('/'));
        assert!(!is_valid_topic_char('-'));
        assert!(!is_valid_topic_char(' '));
    }

    // ==================== Substitution Tests ====================

    #[test]
    fn test_valid_substitutions() {
        assert!(validate_substitution("node").is_ok());
        assert!(validate_substitution("namespace").is_ok());
        assert!(validate_substitution("foo_bar").is_ok());
        assert!(validate_substitution("_private").is_ok());
    }

    #[test]
    fn test_invalid_substitution_empty() {
        assert!(validate_substitution("").is_err());
    }

    #[test]
    fn test_invalid_substitution_starts_with_number() {
        assert!(validate_substitution("123").is_err());
        assert!(validate_substitution("1foo").is_err());
    }

    #[test]
    fn test_invalid_substitution_special_chars() {
        assert!(validate_substitution("foo/bar").is_err());
        assert!(validate_substitution("foo-bar").is_err());
    }

    // ==================== Topic Expansion Tests ====================

    #[test]
    fn test_expand_absolute_topic() {
        // Absolute topics are returned as-is
        let fqn = expand_topic_name("/my_ns", "my_node", "/absolute/topic").unwrap();
        assert_eq!(fqn, "/absolute/topic");

        let fqn = expand_topic_name("/", "node", "/foo").unwrap();
        assert_eq!(fqn, "/foo");

        let fqn = expand_topic_name("/deep/ns", "node", "/other/topic").unwrap();
        assert_eq!(fqn, "/other/topic");
    }

    #[test]
    fn test_expand_private_topic() {
        // Private topics: ~ replaced with node FQN
        let fqn = expand_topic_name("/my_ns", "my_node", "~/private").unwrap();
        assert_eq!(fqn, "/my_ns/my_node/private");

        let fqn = expand_topic_name("/my_ns", "my_node", "~").unwrap();
        assert_eq!(fqn, "/my_ns/my_node");

        let fqn = expand_topic_name("/", "my_node", "~/private").unwrap();
        assert_eq!(fqn, "/my_node/private");

        let fqn = expand_topic_name("/", "my_node", "~").unwrap();
        assert_eq!(fqn, "/my_node");

        let fqn = expand_topic_name("/foo/bar", "node", "~/baz").unwrap();
        assert_eq!(fqn, "/foo/bar/node/baz");
    }

    #[test]
    fn test_expand_relative_topic() {
        // Relative topics: prefixed with namespace
        let fqn = expand_topic_name("/my_ns", "my_node", "relative").unwrap();
        assert_eq!(fqn, "/my_ns/relative");

        let fqn = expand_topic_name("/my_ns", "my_node", "foo/bar").unwrap();
        assert_eq!(fqn, "/my_ns/foo/bar");

        let fqn = expand_topic_name("/", "my_node", "relative").unwrap();
        assert_eq!(fqn, "/relative");

        let fqn = expand_topic_name("/deep/namespace", "node", "topic").unwrap();
        assert_eq!(fqn, "/deep/namespace/topic");
    }

    #[test]
    fn test_expand_topic_with_fqn() {
        let fqn = expand_topic_name_with_fqn("/my_ns/my_node", "~/private").unwrap();
        assert_eq!(fqn, "/my_ns/my_node/private");

        let fqn = expand_topic_name_with_fqn("/my_ns/my_node", "/absolute").unwrap();
        assert_eq!(fqn, "/absolute");

        let fqn = expand_topic_name_with_fqn("/my_ns/my_node", "relative").unwrap();
        assert_eq!(fqn, "/my_ns/relative");

        let fqn = expand_topic_name_with_fqn("/my_node", "~/private").unwrap();
        assert_eq!(fqn, "/my_node/private");

        let fqn = expand_topic_name_with_fqn("/my_node", "relative").unwrap();
        assert_eq!(fqn, "/relative");
    }

    #[test]
    fn test_build_node_fqn() {
        assert_eq!(build_node_fqn("/my_ns", "my_node"), "/my_ns/my_node");
        assert_eq!(build_node_fqn("/", "my_node"), "/my_node");
        assert_eq!(build_node_fqn("/foo/bar", "node"), "/foo/bar/node");
    }

    #[test]
    fn test_extract_namespace() {
        assert_eq!(extract_namespace("/my_ns/my_node"), "/my_ns");
        assert_eq!(extract_namespace("/my_node"), "/");
        assert_eq!(extract_namespace("/foo/bar/baz"), "/foo/bar");
        assert_eq!(extract_namespace("/a/b/c/d"), "/a/b/c");
    }

    #[test]
    fn test_extract_base_name() {
        assert_eq!(extract_base_name("/my_ns/my_node"), "my_node");
        assert_eq!(extract_base_name("/my_node"), "my_node");
        assert_eq!(extract_base_name("/foo/bar/baz"), "baz");
    }

    #[test]
    fn test_expand_topic_invalid_inputs() {
        // Invalid namespace
        assert!(expand_topic_name("invalid", "node", "topic").is_err());

        // Invalid node name
        assert!(expand_topic_name("/ns", "invalid/node", "topic").is_err());

        // Invalid topic
        assert!(expand_topic_name("/ns", "node", "invalid//topic").is_err());
    }
}
