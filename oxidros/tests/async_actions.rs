pub mod common;

use futures::Future;
use oxidros::{
    self,
    action::{
        client::{Client, ClientGoalRecv, ClientResultRecv},
        handle::GoalHandle,
        server::{AsyncServer, Server, ServerCancelSend, ServerGoalSend, ServerQosOption},
        GoalStatus,
    },
    context::Context,
    error::DynError,
    msg::{
        common_interfaces::example_interfaces::action::{
            Fibonacci, Fibonacci_Feedback, Fibonacci_GetResult_Request, Fibonacci_Goal,
            Fibonacci_Result,
        },
        interfaces::action_msgs::{msg::GoalInfo, srv::CancelGoal_Request},
        unique_identifier_msgs::msg::UUID,
    },
};
use std::{pin::Pin, sync::Arc, thread, time::Duration};

fn create_server(
    ctx: &Arc<Context>,
    node: &str,
    action: &str,
    qos: Option<ServerQosOption>,
) -> Result<Server<Fibonacci>, DynError> {
    let node_server = ctx.create_node(node, None, Default::default()).unwrap();

    Server::new(node_server, action, qos).map_err(|e| e.into())
}

fn create_client(
    ctx: &Arc<Context>,
    node: &str,
    action: &str,
) -> Result<Client<Fibonacci>, DynError> {
    let node_client = ctx.create_node(node, None, Default::default())?;
    Client::new(node_client, action, None).map_err(|e| e.into())
}

async fn assert_status(client: Client<Fibonacci>, expected: GoalStatus) -> Client<Fibonacci> {
    let recv = client.recv_status();
    match tokio::time::timeout(Duration::from_secs(3), recv).await {
        Ok(Ok((c, status_array))) => {
            let list = status_array.status_list.as_slice();
            assert!(!list.is_empty());
            let status = list.last().unwrap().status;
            assert_eq!(status, expected as i8);
            c
        }
        Ok(Err(e)) => panic!("{e:?}"),
        Err(_) => panic!("timed out"),
    }
}

fn spawn_worker(handle: GoalHandle<Fibonacci>) {
    std::thread::Builder::new()
        .name("worker".into())
        .spawn(move || {
            let mut sequence = vec![0, 1];
            for c in 0..=5 {
                if handle.is_canceling().unwrap() {
                    println!("server worker: canceling the goal");
                    handle
                        .canceled(Fibonacci_Result {
                            sequence: sequence.as_slice().try_into().unwrap(),
                        })
                        .unwrap();
                    return;
                }

                println!("server worker: sending feedback {c}");
                if c > 1 {
                    let next = sequence[sequence.len() - 1] + sequence[sequence.len() - 2];
                    sequence.push(next);
                }
                let feedback = Fibonacci_Feedback {
                    sequence: sequence.as_slice().try_into().unwrap(),
                };
                handle.feedback(feedback).unwrap();
                std::thread::sleep(Duration::from_secs(1));
            }

            println!("server worker: result is now available");
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

fn spawn_worker_abort(handle: GoalHandle<Fibonacci>) {
    std::thread::Builder::new()
        .name("worker".into())
        .spawn(move || {
            std::thread::sleep(Duration::from_secs(2));

            println!("server worker: aborting the goal");
            handle.abort().unwrap();
        })
        .unwrap();
}

async fn run_server(server: Server<Fibonacci>, abort: bool) -> Result<(), DynError> {
    let mut server = AsyncServer::new(server);

    let goal = move |sender: ServerGoalSend<Fibonacci>, req| {
        println!("server: goal received: {:?}", req);

        let s = sender
            .accept(|handle| {
                if abort {
                    spawn_worker_abort(handle)
                } else {
                    spawn_worker(handle)
                }
            })
            .map_err(|(_sender, err)| err)
            .expect("could not accept");
        // let s = sender.reject().map_err(|(_sender, err)| err)?;

        println!("server: goal response sent");

        s
    };

    let cancel = move |sender: ServerCancelSend<Fibonacci>, candidates| {
        println!("server: received cancel request for: {:?}", candidates);

        let accepted = candidates; // filter requests here if needed

        // return cancel response
        let s = sender
            .send(accepted)
            .map_err(|(_s, err)| err)
            .expect("could not send cancel response");

        // perform shutdown operations for the goals here if needed

        println!("server: cancel response sent");

        s
    };

    server.listen(goal, cancel).await
}

async fn receive_goal_response(receiver: ClientGoalRecv<Fibonacci>) -> Client<Fibonacci> {
    let recv = receiver.recv();
    match tokio::time::timeout(Duration::from_secs(3), recv).await {
        Ok(Ok((c, response, _header))) => {
            println!("client: goal response received: {:?}", response);
            c
        }
        Ok(Err(e)) => panic!("{e:?}"),
        Err(_) => panic!("timed out"),
    }
}

async fn receive_result_response(receiver: ClientResultRecv<Fibonacci>) -> Client<Fibonacci> {
    let recv = receiver.recv();
    match tokio::time::timeout(Duration::from_secs(3), recv).await {
        Ok(Ok((c, response, _header))) => {
            println!("client: result response received: {:?}", response);
            c
        }
        Ok(Err(e)) => panic!("{e:?}"),
        Err(_) => panic!("timed out"),
    }
}

async fn run_client(client: Client<Fibonacci>) -> Result<(), DynError> {
    let uuid: [u8; 16] = rand::random();
    let goal = Fibonacci_Goal { order: 10 };
    let receiver = client.send_goal_with_uuid(goal, uuid)?;

    let mut client = receive_goal_response(receiver).await;

    // receive feedback
    loop {
        let recv = client.recv_feedback();
        client = match tokio::time::timeout(Duration::from_secs(3), recv).await {
            Ok(Ok((c, feedback))) => {
                println!("client: feedback received: {:?}", feedback);

                if feedback.feedback.sequence.len() >= 6 {
                    client = c;
                    break;
                }
                c
            }
            Ok(Err(e)) => panic!("{e:?}"),
            Err(_) => panic!("timed out"),
        };
    }

    thread::sleep(Duration::from_secs(4));

    // send a result request
    println!("sending result request...");
    let receiver = client.send_result_request(&Fibonacci_GetResult_Request {
        goal_id: UUID { uuid },
    })?;

    let _ = receive_result_response(receiver).await;

    Ok(())
}

async fn run_client_cancel(client: Client<Fibonacci>) -> Result<(), DynError> {
    let uuid: [u8; 16] = rand::random();
    let goal = Fibonacci_Goal { order: 10 };
    let receiver = client.send_goal_with_uuid(goal, uuid)?;

    let client = receive_goal_response(receiver).await;
    thread::sleep(Duration::from_secs(1));

    // send a cancel request
    let receiver = client.send_cancel_request(&CancelGoal_Request {
        goal_info: GoalInfo {
            goal_id: oxidros_msg::interfaces::unique_identifier_msgs::msg::UUID { uuid },
            stamp: oxidros_msg::interfaces::builtin_interfaces::msg::Time { sec: 0, nanosec: 0 },
        },
    })?;
    println!("client: cancel request sent");

    let client = match receiver.recv().await {
        Ok((c, resp, _header)) => {
            println!("client: cancel response received: {:?}", resp);
            c
        }
        Err(e) => panic!("client: could not cancel the goal: {e:?}"),
    };

    std::thread::sleep(Duration::from_secs(2));

    let _ = assert_status(client, GoalStatus::Canceled).await;

    Ok(())
}

async fn run_client_status(client: Client<Fibonacci>) -> Result<(), DynError> {
    let uuid: [u8; 16] = rand::random();
    let goal = Fibonacci_Goal { order: 10 };
    let receiver = client.send_goal_with_uuid(goal, uuid)?;

    let client = receive_goal_response(receiver).await;
    std::thread::sleep(Duration::from_secs(1));

    // wait for the task to finish
    let client = assert_status(client, GoalStatus::Executing).await;
    std::thread::sleep(Duration::from_secs(10));

    let _ = assert_status(client, GoalStatus::Succeeded).await;

    Ok(())
}

async fn run_client_abort(client: Client<Fibonacci>) -> Result<(), DynError> {
    let uuid: [u8; 16] = rand::random();
    let goal = Fibonacci_Goal { order: 10 };
    let receiver = client.send_goal_with_uuid(goal, uuid)?;

    let client = receive_goal_response(receiver).await;
    std::thread::sleep(Duration::from_secs(1));

    let client = assert_status(client, GoalStatus::Executing).await;
    std::thread::sleep(Duration::from_secs(3));

    let _ = assert_status(client, GoalStatus::Aborted).await;

    Ok(())
}

async fn start_server_client<G>(
    action: &str,
    client_node: &str,
    server_node: &str,
    run_client_fn: G,
    server_abort: bool,
) -> Result<(), DynError>
where
    G: FnOnce(Client<Fibonacci>) -> Pin<Box<dyn Future<Output = Result<(), DynError>> + Send>>
        + Send
        + 'static,
{
    let ctx = Context::new().unwrap();
    let client = create_client(&ctx, client_node, action).unwrap();
    let server = create_server(&ctx, server_node, action, None).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();

    tokio::task::spawn({
        let server = server.clone();
        run_server(server, server_abort)
    });
    tokio::task::spawn(async move {
        let ret = run_client_fn(client).await;
        let _ = tx.send(());
        ret
    });

    let _ = rx.recv();

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_async_action() -> Result<(), DynError> {
    start_server_client(
        "test_async_action",
        "test_async_action_client",
        "test_async_action_server",
        |client| Box::pin(run_client(client)),
        false,
    )
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_async_action_cancel() -> Result<(), DynError> {
    start_server_client(
        "test_async_action_cancel",
        "test_async_action_client_cancel",
        "test_async_action_server_cancel",
        |client| Box::pin(run_client_cancel(client)),
        false,
    )
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_async_action_status() -> Result<(), DynError> {
    start_server_client(
        "test_async_action_status",
        "test_async_action_client_status",
        "test_async_action_server_status",
        |client| Box::pin(run_client_status(client)),
        false,
    )
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_async_action_abort() -> Result<(), DynError> {
    start_server_client(
        "test_async_action_abort",
        "test_async_action_client_abort",
        "test_async_action_server_abort",
        |client| Box::pin(run_client_abort(client)),
        true,
    )
    .await?;
    Ok(())
}
