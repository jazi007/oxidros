#[cfg(test)]
mod tests {
    use oxidros_rcl::context::Context;
    use oxidros_rcl::msg::common_interfaces::std_msgs;

    #[test]
    fn test_publisher_topic_names() {
        let ctx = Context::new().unwrap();

        // Test with namespace - relative topic
        let node = ctx.create_node("test_node", Some("/my_ns")).unwrap();
        let pub1 = node
            .create_publisher::<std_msgs::msg::String>("my_topic", None)
            .unwrap();

        assert_eq!(
            pub1.fully_qualified_topic_name().unwrap().as_str(),
            "/my_ns/my_topic"
        );
        assert_eq!(pub1.topic_name().unwrap().as_str(), "my_topic");

        // Test with namespace - absolute topic (should ignore namespace)
        let pub2 = node
            .create_publisher::<std_msgs::msg::String>("/absolute", None)
            .unwrap();

        assert_eq!(
            pub2.fully_qualified_topic_name().unwrap().as_str(),
            "/absolute"
        );
        assert_eq!(pub2.topic_name().unwrap().as_str(), "absolute");

        // Test without namespace
        let node2 = ctx.create_node("test_node2", None).unwrap();
        let pub3 = node2
            .create_publisher::<std_msgs::msg::String>("simple", None)
            .unwrap();

        assert_eq!(
            pub3.fully_qualified_topic_name().unwrap().as_str(),
            "/simple"
        );
        assert_eq!(pub3.topic_name().unwrap().as_str(), "simple");
    }

    #[test]
    fn test_subscriber_topic_names() {
        let ctx = Context::new().unwrap();

        // Test with namespace - relative topic
        let node = ctx.create_node("test_node_sub", Some("/my_ns")).unwrap();
        let sub1 = node
            .create_subscriber::<std_msgs::msg::String>("my_topic", None)
            .unwrap();

        assert_eq!(
            sub1.fully_qualified_topic_name().unwrap().as_str(),
            "/my_ns/my_topic"
        );
        assert_eq!(sub1.topic_name().unwrap().as_str(), "my_topic");

        // Test with namespace - absolute topic (should ignore namespace)
        let sub2 = node
            .create_subscriber::<std_msgs::msg::String>("/absolute", None)
            .unwrap();

        assert_eq!(
            sub2.fully_qualified_topic_name().unwrap().as_str(),
            "/absolute"
        );
        assert_eq!(sub2.topic_name().unwrap().as_str(), "absolute");

        // Test without namespace
        let node2 = ctx.create_node("test_node2_sub", None).unwrap();
        let sub3 = node2
            .create_subscriber::<std_msgs::msg::String>("simple", None)
            .unwrap();

        assert_eq!(
            sub3.fully_qualified_topic_name().unwrap().as_str(),
            "/simple"
        );
        assert_eq!(sub3.topic_name().unwrap().as_str(), "simple");
    }
}
