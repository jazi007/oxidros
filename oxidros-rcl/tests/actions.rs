#![cfg(feature = "rcl")]

pub mod common;

use oxidros_rcl::{
    action::{
        GoalStatus,
        client::Client,
        handle::GoalHandle,
        server::{Server, ServerQosOption},
    },
    context::Context,
    error::Result,
    msg::{
        common_interfaces::example_interfaces::action::{
            Fibonacci, Fibonacci_Feedback, Fibonacci_GetResult_Request, Fibonacci_Goal,
            Fibonacci_Result,
        },
        interfaces::action_msgs::{msg::GoalInfo, srv::CancelGoal_Request},
        unique_identifier_msgs::msg::UUID,
    },
};
use std::{sync::Arc, thread, time::Duration};

fn create_server(
    ctx: &Arc<Context>,
    node: &str,
    action: &str,
    qos: Option<ServerQosOption>,
) -> Result<Server<Fibonacci>> {
    let node_server = ctx.create_node(node, None, Default::default()).unwrap();

    Server::new(node_server, action, qos)
}

fn create_client(ctx: &Arc<Context>, node: &str, action: &str) -> Result<Client<Fibonacci>> {
    let options = oxidros_rcl::node::NodeOptions::default();
    let node_client = ctx.create_node(node, None, options)?;
    Client::new(node_client, action, None)
}

fn accept_handler(handle: GoalHandle<Fibonacci>) {
    std::thread::Builder::new()
        .name("worker".into())
        .spawn(move || {
            let mut sequence = vec![0, 1];
            for i in 0..=5 {
                std::thread::sleep(Duration::from_secs(2));
                println!("server worker: sending feedback {i}");
                if i > 1 {
                    let next = sequence[sequence.len() - 1] + sequence[sequence.len() - 2];
                    sequence.push(next);
                }
                let feedback = Fibonacci_Feedback {
                    sequence: sequence.as_slice().try_into().unwrap(),
                };
                handle.feedback(feedback).unwrap();
            }

            println!("server worker: sending result");
            handle
                .finish(Fibonacci_Result {
                    sequence: sequence.as_slice().try_into().unwrap(),
                })
                .unwrap();

            loop {
                std::thread::sleep(Duration::from_secs(5));
            }
        })
        .unwrap();
}

#[test]
fn test_action() -> Result<()> {
    let ctx = Context::new()?;

    let mut client = create_client(&ctx, "test_action_client", "test_action")?;

    let mut selector = ctx.create_selector()?;
    let server = create_server(&ctx, "test_action_server", "test_action", None)?;

    // send goal request
    let uuid: [u8; 16] = rand::random();
    let uuid_ = uuid;
    let goal = Fibonacci_Goal { order: 10 };
    let recv = client.send_goal_with_uuid(goal, uuid)?;

    thread::sleep(Duration::from_millis(100));

    // You don't have to set handlers for incoming result requests since they are processed
    // automatically.
    selector.add_action_server(
        server.clone(),
        move |_| true,
        accept_handler,
        move |_goal| true,
    );
    selector.wait()?;

    loop {
        match recv.recv_timeout(Duration::from_secs(3), &mut selector) {
            Ok(Some((data, header))) => {
                println!(
                    "received goal response: accepted = {:?}, seq = {}",
                    data.accepted, header.sequence_number
                );
                break;
            }
            Ok(None) => {
                println!("did not receive goal response, retrying");
            }
            Err(e) => panic!("{}", e),
        }
    }

    // wait for five feedback messages
    let mut received = 0;
    while received <= 5 {
        match client.recv_feedback_timeout(Duration::from_secs(3), &mut selector) {
            Ok(Some(feedback)) => {
                println!("received feedback: {:?}", feedback);
                received += 1;
            }
            Ok(None) => {}
            Err(e) => panic!("{}", e),
        }
    }

    let mut goal_id = UUID::new().unwrap();
    goal_id.uuid = uuid_;
    let result_req = Fibonacci_GetResult_Request { goal_id };
    let recv = client.send_result_request(&result_req)?;

    selector.wait()?;

    loop {
        match recv.recv_timeout(Duration::from_secs(3), &mut selector) {
            Ok(Some((data, header))) => {
                println!(
                    "received result: result = {:?} status = {:?}, seq = {}",
                    data.result, data.status, header.sequence_number
                );
                break;
            }
            Ok(None) => {}
            Err(e) => panic!("{}", e),
        };
    }

    Ok(())
}

#[test]
fn test_action_cancel() -> Result<()> {
    let ctx = Context::new()?;

    let mut client = create_client(&ctx, "test_action_cancel_client", "test_action_cancel")?;

    let mut selector = ctx.create_selector()?;
    let server = create_server(
        &ctx,
        "test_action_cancel_server",
        "test_action_cancel",
        None,
    )?;

    // send goal request
    let uuid: [u8; 16] = rand::random();
    let goal = Fibonacci_Goal { order: 10 };
    let recv = client.send_goal_with_uuid(goal, uuid)?;

    thread::sleep(Duration::from_millis(100));

    selector.add_action_server(
        server,
        |_| true,
        accept_handler,
        move |goal| {
            println!("Cancel request received for goal {:?}", goal);
            true
        },
    );
    selector.wait()?;

    loop {
        match recv.recv_timeout(Duration::from_secs(3), &mut selector) {
            Ok(Some((data, header))) => {
                println!(
                    "received goal response: accepted = {:?}, seq = {}",
                    data.accepted, header.sequence_number
                );
                break;
            }
            Ok(None) => {}
            Err(e) => panic!("{}", e),
        }
    }

    let request = CancelGoal_Request {
        goal_info: GoalInfo {
            goal_id: oxidros_msg::interfaces::unique_identifier_msgs::msg::UUID { uuid },
            stamp: oxidros_msg::interfaces::builtin_interfaces::msg::Time { sec: 0, nanosec: 0 },
        },
    };
    let recv = client.send_cancel_request(&request)?;

    loop {
        match recv.recv_timeout(Duration::from_secs(3), &mut selector) {
            Ok(Some((data, header))) => {
                println!(
                    "received cancel goal response: data = {:?}, seq = {}",
                    data, header.sequence_number
                );
                break;
            }
            Ok(None) => {
                println!("retrying");
            }
            Err(e) => panic!("{}", e),
        }
    }

    Ok(())
}

#[test]
fn test_action_status() -> Result<()> {
    let ctx = Context::new()?;

    let mut client = create_client(&ctx, "test_action_status_client", "test_action_status")?;

    let mut selector = ctx.create_selector()?;
    let server = create_server(
        &ctx,
        "test_action_status_server",
        "test_action_status",
        None,
    )?;

    // send goal request
    let uuid: [u8; 16] = rand::random();
    let goal = Fibonacci_Goal { order: 0 };
    let recv = client.send_goal_with_uuid(goal, uuid)?;

    thread::sleep(Duration::from_millis(100));

    selector.add_action_server(server, |_| true, accept_handler, move |_goal| true);
    selector.wait()?;

    loop {
        match recv.recv_timeout(Duration::from_secs(3), &mut selector) {
            Ok(Some((data, header))) => {
                println!(
                    "received goal response: accepted = {:?}, seq = {}",
                    data.accepted, header.sequence_number
                );
                break;
            }
            Ok(None) => {}
            Err(e) => panic!("{}", e),
        }
    }

    // get status
    loop {
        match client.recv_status_timeout(Duration::from_secs(3), &mut selector) {
            Ok(Some(statuses)) => {
                for stat in statuses.status_list.iter() {
                    if stat.goal_info.goal_id.uuid == uuid {
                        let status: GoalStatus = stat.status.into();
                        println!("received status = {:?}", status);

                        if status == GoalStatus::Succeeded {
                            return Ok(());
                        }
                    }
                }
            }
            Ok(None) => {}
            Err(e) => panic!("{}", e),
        }
    }
}
