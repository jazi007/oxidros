#!/usr/bin/env python3
"""
Generate API documentation comparing oxidros-rcl and oxidros-zenoh.

This script uses `cargo public-api` to extract public APIs from both backends
and generates a Markdown file showing common APIs and differences.

Requirements:
- cargo-public-api (install with: cargo install cargo-public-api)
- ROS2 environment sourced (for oxidros-msg build)

Usage:
    python scripts/generate_api_docs.py [--output docs/API_REFERENCE.md]
"""

import subprocess
import re
import argparse
from pathlib import Path
from collections import defaultdict
from dataclasses import dataclass
from typing import Optional


@dataclass
class ApiItem:
    """Represents a public API item."""
    kind: str  # fn, struct, enum, type, const, mod, trait
    path: str  # full path like oxidros_zenoh::Context
    signature: str  # full signature
    name: str  # short name like "new" or "Context"
    parent: str  # parent module/struct


def run_cargo_public_api(package: str, features: Optional[str] = None) -> str:
    """Run cargo public-api and return output."""
    cmd = ["cargo", "public-api", "-p", package]
    if features:
        cmd.extend(["--features", features])
    
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        cwd=Path(__file__).parent.parent
    )
    
    if result.returncode != 0:
        print(f"Warning: cargo public-api failed for {package}")
        print(result.stderr)
        return ""
    
    return result.stdout


def parse_api_line(line: str, crate_name: str) -> Optional[ApiItem]:
    """Parse a single API line into an ApiItem."""
    # Match patterns like: pub fn oxidros_zenoh::Context::new() -> ...
    # or: pub struct oxidros_zenoh::Context
    
    patterns = [
        (r"^pub (fn) (\S+::\S+)\((.*)", "fn"),
        (r"^pub (struct) (\S+)", "struct"),
        (r"^pub (enum) (\S+)", "enum"),
        (r"^pub (type) (\S+)", "type"),
        (r"^pub (const) (\S+)", "const"),
        (r"^pub (mod) (\S+)", "mod"),
        (r"^pub (trait) (\S+)", "trait"),
    ]
    
    for pattern, kind in patterns:
        match = re.match(pattern, line)
        if match:
            path = match.group(2)
            
            # Only include items from our crate
            if not path.startswith(crate_name):
                return None
            
            # Skip impl blocks from dependencies
            if any(dep in path for dep in [
                "stabby_abi", "ppv_lite86", "crossbeam_epoch", 
                "zenoh_keyexpr", "asn1_rs", "typenum", "either",
                "tracing::", "core::", "alloc::", "std::"
            ]):
                return None
            
            # Skip auto-derived trait methods
            auto_derived_methods = [
                "::borrow(", "::borrow_mut(", "::from(", "::into(",
                "::try_from(", "::try_into(", "::type_id(",
                "::clone(", "::clone_from(", "::default(",
                "::eq(", "::ne(", "::partial_cmp(", "::cmp(",
                "::hash(", "::fmt(",
                # stabby_abi and other internal trait impls
                "::guard_mut_inner(", "::guard_ref_inner(",
                "::mut_as<", "::ref_as<", "::as_node(", "::as_node_mut(",
                "::vzip(", "::to_owned(", "::clone_into(",
                "::__clone_box(", "::deref(", "::deref_mut(",
            ]
            if any(m in line for m in auto_derived_methods):
                return None
            
            # Skip type aliases for Error, Output, Guard, Init, Owned (from traits)
            if kind == "type" and any(x in path for x in [
                "::Error", "::Output", "::Guard<", "::GuardMut<", 
                "::Init", "::Owned", "::Request"
            ]):
                return None
            
            # Skip const ALIGN (from Pointable trait)
            if kind == "const" and "::ALIGN" in path:
                return None
            
            # Extract name and parent
            parts = path.split("::")
            name = parts[-1] if parts else path
            parent = "::".join(parts[:-1]) if len(parts) > 1 else ""
            
            return ApiItem(
                kind=kind,
                path=path,
                signature=line,
                name=name,
                parent=parent
            )
    
    return None


def extract_apis(output: str, crate_name: str) -> dict[str, ApiItem]:
    """Extract API items from cargo public-api output."""
    apis = {}
    
    for line in output.splitlines():
        line = line.strip()
        if not line.startswith("pub "):
            continue
        
        item = parse_api_line(line, crate_name)
        if item:
            # Use a normalized key for comparison
            # Replace crate name with generic prefix
            normalized_path = item.path.replace(crate_name, "oxidros")
            apis[normalized_path] = item
    
    return apis


def categorize_by_module(apis: dict[str, ApiItem]) -> dict[str, list[ApiItem]]:
    """Group API items by their parent module."""
    by_module = defaultdict(list)
    
    for item in apis.values():
        # Get the main module (e.g., Context, Node, topic::publisher)
        parts = item.path.split("::")
        if len(parts) >= 2:
            # Skip crate name, get first module
            module = parts[1] if len(parts) > 1 else "root"
            by_module[module].append(item)
    
    return by_module


def generate_markdown(
    zenoh_apis: dict[str, ApiItem],
    rcl_apis: dict[str, ApiItem],
    output_path: Path
) -> None:
    """Generate Markdown documentation."""
    
    # Find common and unique APIs
    zenoh_keys = set(zenoh_apis.keys())
    rcl_keys = set(rcl_apis.keys())
    
    common = zenoh_keys & rcl_keys
    zenoh_only = zenoh_keys - rcl_keys
    rcl_only = rcl_keys - zenoh_keys
    
    lines = [
        "# Oxidros API Reference",
        "",
        "This document is auto-generated by `scripts/generate_api_docs.py`.",
        "",
        "## Summary",
        "",
        f"| Category | Count |",
        f"|----------|-------|",
        f"| Common APIs | {len(common)} |",
        f"| Zenoh-only APIs | {len(zenoh_only)} |",
        f"| RCL-only APIs | {len(rcl_only)} |",
        "",
        "---",
        "",
    ]
    
    # Common APIs by category
    lines.extend([
        "## Common APIs (Both Backends)",
        "",
        "These APIs are available in both `oxidros-rcl` and `oxidros-zenoh` with the same signature.",
        "",
    ])
    
    common_by_module = defaultdict(list)
    for key in sorted(common):
        item = zenoh_apis[key]
        parts = key.split("::")
        module = parts[1] if len(parts) > 1 else "root"
        common_by_module[module].append(item)
    
    for module in sorted(common_by_module.keys()):
        items = common_by_module[module]
        lines.append(f"### {module}")
        lines.append("")
        lines.append("```rust")
        for item in sorted(items, key=lambda x: x.name):
            # Show simplified signature
            sig = item.signature.replace("oxidros_zenoh::", "")
            lines.append(sig)
        lines.append("```")
        lines.append("")
    
    # Zenoh-only APIs
    lines.extend([
        "---",
        "",
        "## Zenoh-Only APIs",
        "",
        "These APIs are specific to `oxidros-zenoh`.",
        "",
    ])
    
    zenoh_by_module = defaultdict(list)
    for key in sorted(zenoh_only):
        item = zenoh_apis[key]
        parts = key.split("::")
        module = parts[1] if len(parts) > 1 else "root"
        zenoh_by_module[module].append(item)
    
    for module in sorted(zenoh_by_module.keys()):
        items = zenoh_by_module[module]
        lines.append(f"### {module}")
        lines.append("")
        lines.append("```rust")
        for item in sorted(items, key=lambda x: x.name):
            sig = item.signature.replace("oxidros_zenoh::", "")
            lines.append(sig)
        lines.append("```")
        lines.append("")
    
    # RCL-only APIs
    lines.extend([
        "---",
        "",
        "## RCL-Only APIs",
        "",
        "These APIs are specific to `oxidros-rcl`.",
        "",
    ])
    
    rcl_by_module = defaultdict(list)
    for key in sorted(rcl_only):
        item = rcl_apis[key]
        parts = key.split("::")
        module = parts[1] if len(parts) > 1 else "root"
        rcl_by_module[module].append(item)
    
    for module in sorted(rcl_by_module.keys()):
        items = rcl_by_module[module]
        lines.append(f"### {module}")
        lines.append("")
        lines.append("```rust")
        for item in sorted(items, key=lambda x: x.name):
            sig = item.signature.replace("oxidros_rcl::", "")
            lines.append(sig)
        lines.append("```")
        lines.append("")
    
    # Write output
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text("\n".join(lines))
    print(f"Generated: {output_path}")


def main():
    parser = argparse.ArgumentParser(description="Generate API documentation")
    parser.add_argument(
        "--output", "-o",
        default="docs/API_REFERENCE.md",
        help="Output Markdown file path"
    )
    parser.add_argument(
        "--rcl-features",
        default="jazzy",
        help="Features for oxidros-rcl (default: jazzy)"
    )
    args = parser.parse_args()
    
    print("Extracting oxidros-zenoh API...")
    zenoh_output = run_cargo_public_api("oxidros-zenoh")
    zenoh_apis = extract_apis(zenoh_output, "oxidros_zenoh")
    print(f"  Found {len(zenoh_apis)} API items")
    
    print("Extracting oxidros-rcl API...")
    rcl_output = run_cargo_public_api("oxidros-rcl", args.rcl_features)
    rcl_apis = extract_apis(rcl_output, "oxidros_rcl")
    print(f"  Found {len(rcl_apis)} API items")
    
    output_path = Path(__file__).parent.parent / args.output
    generate_markdown(zenoh_apis, rcl_apis, output_path)


if __name__ == "__main__":
    main()
