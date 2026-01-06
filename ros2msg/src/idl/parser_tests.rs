#[cfg(test)]
mod tests {
    use crate::idl::grammar::parse_idl_string;

    /// Helper function to get a temporary base path for tests
    fn get_test_base_path() -> std::path::PathBuf {
        std::env::temp_dir()
    }

    #[test]
    fn test_parse_simple_const() {
        let input = "const int32 MY_CONST = 42;";
        let result = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("test.idl"),
        );
        if let Err(ref e) = result {
            println!("Parse error: {e:?}");
        }
        assert!(result.is_ok());

        let idl_file = result.unwrap();
        assert_eq!(
            idl_file.locator.relative_path,
            std::path::PathBuf::from("test.idl")
        );
    }

    #[test]
    fn test_parse_struct() {
        let input = r"
        struct Point {
            int32 x;
            int32 y;
        };
        ";
        let result = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("point.idl"),
        );
        if let Err(ref e) = result {
            println!("Parse error: {e:?}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_module() {
        let input = r"module geometry {
    const int32 MAX_POINTS = 100;
}";
        let result = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("geometry.idl"),
        );
        if let Err(ref e) = result {
            println!("Parse error: {e:?}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_module_with_struct_int32() {
        let input = r"module geometry {
    struct Point {
        int32 x;
    };
}";
        let result = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("geometry.idl"),
        );
        if let Err(ref e) = result {
            println!("Parse error: {e:?}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_module_with_struct_double() {
        let input = r"module geometry {
    struct Point {
        double x;
    };
}";
        let result = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("geometry.idl"),
        );
        if let Err(ref e) = result {
            println!("Parse error: {e:?}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_const_long() {
        let input = "const long MAX_SIZE = 1000;";
        let result = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("test.idl"),
        );
        if let Err(ref e) = result {
            println!("Parse error: {e:?}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_struct_double_toplevel() {
        let input = r"struct Point {
    double x;
};";
        let result = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("geometry.idl"),
        );
        if let Err(ref e) = result {
            println!("Parse error: {e:?}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_enum() {
        let input = r"
        enum Color {
            RED,
            GREEN,
            BLUE
        };
        ";
        let result = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("color.idl"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_nested_modules() {
        let input = r"module geometry_msgs {
    module msg {
        struct Point {
            double x;
        };
    };
}";
        let result = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("nested.idl"),
        );
        if let Err(ref e) = result {
            println!("Parse error: {e:?}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_service_promoted_from_messages() {
        let input = r"module example_pkg {
    module srv {
        struct DoThing_Request {
            int32 value;
        };
        struct DoThing_Response {
            int32 result;
        };
        struct DoThing_Event {
            int32 info;
        };
    };
};";

        let idl_file = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("service.idl"),
        )
        .expect("service idl should parse");

        let services = idl_file.content.get_services();
        assert_eq!(services.len(), 1);
        let service = services[0];
        assert_eq!(service.namespaced_type.name, "DoThing");
        assert_eq!(service.request_message.structure.members.len(), 1);
        assert_eq!(service.response_message.structure.members.len(), 1);
        assert_eq!(service.event_message.structure.members.len(), 1);
        assert!(idl_file.content.get_messages().is_empty());
    }

    #[test]
    fn test_action_promoted_from_messages() {
        let input = r"module example_pkg {
    module action {
        struct DoAction_Goal {
            int32 target;
        };
        struct DoAction_Result {
            boolean success;
        };
        struct DoAction_Feedback {
            float progress;
        };
        struct DoAction_FeedbackMessage {
            float status;
        };
        struct DoAction_SendGoal_Request {
            int32 id;
        };
        struct DoAction_SendGoal_Response {
            boolean accepted;
        };
        struct DoAction_GetResult_Request {
            int32 seq;
        };
        struct DoAction_GetResult_Response {
            boolean ready;
        };
    };
};";

        let idl_file = parse_idl_string(
            input,
            get_test_base_path(),
            std::path::PathBuf::from("action.idl"),
        )
        .expect("action idl should parse");

        let actions = idl_file.content.get_actions();
        assert_eq!(actions.len(), 1);
        let action = actions[0];
        assert_eq!(action.namespaced_type.name, "DoAction");
        assert_eq!(action.goal.structure.members.len(), 1);
        assert_eq!(action.result.structure.members.len(), 1);
        assert_eq!(action.feedback.structure.members.len(), 1);
        assert_eq!(action.feedback_message.structure.members.len(), 1);
        assert_eq!(
            action
                .send_goal_service
                .request_message
                .structure
                .members
                .len(),
            1
        );
        assert_eq!(
            action
                .get_result_service
                .response_message
                .structure
                .members
                .len(),
            1
        );

        // Services that are part of the action (SendGoal, GetResult) are consumed by the action
        // and not returned as separate services
        let services = idl_file.content.get_services();
        assert_eq!(services.len(), 0);
        assert!(idl_file.content.get_messages().is_empty());
    }
}
