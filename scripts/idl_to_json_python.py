#!/usr/bin/env python3
"""
Convert ROS2 IDL files to JSON format using rosidl_parser.
This script parses IDL files and outputs a JSON representation.
"""

import sys
import json
from pathlib import Path
from rosidl_parser.parser import parse_idl_file
from rosidl_parser.definition import IdlLocator


def get_all_slots(cls):
    """Get all __slots__ from class and all parent classes."""
    slots = []
    for klass in cls.__mro__:
        if hasattr(klass, "__slots__"):
            s = klass.__slots__
            # Handle both tuple/list and single string cases
            if isinstance(s, str):
                slots.append(s)
            else:
                slots.extend(s)
    return slots


def object_to_dict(obj):
    """Convert a Python object to a dictionary using __slots__ (including inherited)."""
    if obj is None:
        return None
    elif isinstance(obj, (str, int, float, bool)):
        return obj
    elif isinstance(obj, Path):
        return str(obj)
    elif isinstance(obj, (list, tuple)):
        return [object_to_dict(item) for item in obj]
    elif isinstance(obj, dict):
        return {k: object_to_dict(v) for k, v in obj.items()}
    elif hasattr(obj, "__slots__"):
        result = {}
        # Include slots from all parent classes
        for slot in get_all_slots(type(obj)):
            if hasattr(obj, slot):
                value = getattr(obj, slot)
                # Skip empty annotations lists
                if slot == "annotations" and isinstance(value, (list, tuple)) and len(value) == 0:
                    continue
                result[slot] = object_to_dict(value)
        return result
    else:
        return str(obj)


def idl_to_json(idl_path):
    """Parse IDL file and return JSON representation."""
    path = Path(idl_path)
    basepath = path.parent
    relative_path = Path(path.name)
    locator = IdlLocator(basepath, relative_path)
    idl_tree = parse_idl_file(locator)
    return object_to_dict(idl_tree)


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <idl-file>")
        sys.exit(1)

    idl_file = sys.argv[1]

    try:
        json_data = idl_to_json(idl_file)
        print(json.dumps(json_data, indent=2))
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        import traceback

        traceback.print_exc(file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
