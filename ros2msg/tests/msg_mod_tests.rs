// Tests for msg::mod to improve coverage

use ros2msg::msg::*;
use ros2msg::{parse_action_string, parse_message_string, parse_service_string};

#[test]
fn test_interface_specification_message_package_name() {
    let msg = parse_message_string("test_pkg", "TestMsg", "int32 x\n").unwrap();
    let iface = InterfaceSpecification::Message(msg);
    assert_eq!(iface.package_name(), "test_pkg");
}

#[test]
fn test_interface_specification_message_interface_name() {
    let msg = parse_message_string("pkg", "MsgName", "int32 x\n").unwrap();
    let iface = InterfaceSpecification::Message(msg);
    assert_eq!(iface.interface_name(), "MsgName");
}

#[test]
fn test_interface_specification_service_package_name() {
    let srv = parse_service_string("srv_pkg", "SrvName", "int32 a\n---\nint32 b\n").unwrap();
    let iface = InterfaceSpecification::Service(srv);
    assert_eq!(iface.package_name(), "srv_pkg");
}

#[test]
fn test_interface_specification_service_interface_name() {
    let srv = parse_service_string("pkg", "Service", "int32 a\n---\nint32 b\n").unwrap();
    let iface = InterfaceSpecification::Service(srv);
    assert_eq!(iface.interface_name(), "Service");
}

#[test]
fn test_interface_specification_action_package_name() {
    let action = parse_action_string(
        "action_pkg",
        "ActionName",
        "int32 x\n---\nint32 y\n---\nint32 z\n",
    )
    .unwrap();
    let iface = InterfaceSpecification::Action(action);
    assert_eq!(iface.package_name(), "action_pkg");
}

#[test]
fn test_interface_specification_action_interface_name() {
    let action =
        parse_action_string("pkg", "MyAction", "int32 x\n---\nint32 y\n---\nint32 z\n").unwrap();
    let iface = InterfaceSpecification::Action(action);
    assert_eq!(iface.interface_name(), "MyAction");
}

#[test]
fn test_parse_interface_file_msg() {
    let temp_dir = std::env::temp_dir();
    let msg_path = temp_dir.join("TestMsg.msg");
    std::fs::write(&msg_path, "int32 x\nint32 y\n").unwrap();

    let result = parse_interface_file("test_pkg", &msg_path);
    assert!(result.is_ok());
    if let Ok(InterfaceSpecification::Message(msg)) = result {
        assert_eq!(msg.pkg_name, "test_pkg");
        assert_eq!(msg.fields.len(), 2);
    }
    std::fs::remove_file(&msg_path).ok();
}

#[test]
fn test_parse_interface_file_srv() {
    let temp_dir = std::env::temp_dir();
    let srv_path = temp_dir.join("TestSrv.srv");
    std::fs::write(&srv_path, "int32 a\n---\nint32 b\n").unwrap();

    let result = parse_interface_file("srv_pkg", &srv_path);
    assert!(result.is_ok());
    if let Ok(InterfaceSpecification::Service(_)) = result {
        // Success
    } else {
        panic!("Expected Service");
    }
    std::fs::remove_file(&srv_path).ok();
}

#[test]
fn test_parse_interface_file_action() {
    let temp_dir = std::env::temp_dir();
    let action_path = temp_dir.join("TestAction.action");
    std::fs::write(&action_path, "int32 x\n---\nint32 y\n---\nint32 z\n").unwrap();

    let result = parse_interface_file("action_pkg", &action_path);
    assert!(result.is_ok());
    if let Ok(InterfaceSpecification::Action(_)) = result {
        // Success
    } else {
        panic!("Expected Action");
    }
    std::fs::remove_file(&action_path).ok();
}

#[test]
fn test_parse_interface_file_invalid_extension() {
    let temp_dir = std::env::temp_dir();
    let invalid_path = temp_dir.join("test.txt");
    std::fs::write(&invalid_path, "int32 x\n").unwrap();

    let result = parse_interface_file("pkg", &invalid_path);
    assert!(result.is_err());
    std::fs::remove_file(&invalid_path).ok();
}

#[test]
fn test_parse_interface_file_no_extension() {
    let temp_dir = std::env::temp_dir();
    let no_ext_path = temp_dir.join("noextension");
    std::fs::write(&no_ext_path, "int32 x\n").unwrap();

    let result = parse_interface_file("pkg", &no_ext_path);
    assert!(result.is_err());
    std::fs::remove_file(&no_ext_path).ok();
}

#[test]
fn test_parse_interface_file_not_found() {
    let result = parse_interface_file("pkg", std::path::Path::new("/nonexistent/file.msg"));
    assert!(result.is_err());
}
