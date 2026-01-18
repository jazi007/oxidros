use super::guard_condition::GuardCondition;
use crate::{
    action,
    context::Context,
    error::Result,
    service::{client::ClientData, server::ServerData},
    topic::subscriber::RCLSubscription,
};
use crossbeam_channel::{Receiver, Sender};
use oxidros_core::selector::CallbackResult;
use parking_lot::Mutex;
use std::{
    sync::{Arc, OnceLock},
    thread::{self, JoinHandle, yield_now},
};

static SELECTOR_DATA: OnceLock<SelectorData> = OnceLock::new();

pub(crate) enum Command {
    Subscription(
        Arc<RCLSubscription>,
        Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
    ),
    RemoveSubscription(Arc<RCLSubscription>),
    Server(
        Arc<ServerData>,
        Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
    ),
    RemoveServer(Arc<ServerData>),
    Client(
        Arc<ClientData>,
        Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
    ),
    RemoveClient(Arc<ClientData>),
    ActionClient {
        data: Arc<action::client::ClientData>,
        feedback: Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
        status: Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
        goal: Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
        cancel: Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
        result: Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
    },
    RemoveActionClient(Arc<action::client::ClientData>),
    ActionServer {
        data: Arc<action::server::ServerData>,
        goal: Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
        cancel: Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
        result: Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
    },
    RemoveActionServer(Arc<action::server::ServerData>),
    ConditionVar(
        GuardCondition,
        Box<dyn FnMut() -> CallbackResult + Send + Sync + 'static>,
    ),
    RemoveConditionVar(GuardCondition),
    Halt,
}

struct SelectorData {
    tx: Sender<Command>,
    th: Mutex<Option<JoinHandle<Result<()>>>>,
    cond: GuardCondition,
}

pub(crate) fn halt() -> Result<()> {
    if let Some(data) = SELECTOR_DATA.get() {
        data.tx
            .send(Command::Halt)
            .map_err(|_| crate::error::Error::ChannelClosed)?;
        data.cond.trigger()?;

        yield_now();

        let id = data
            .th
            .lock()
            .as_ref()
            .map(|th| th.thread().id())
            .unwrap_or(std::thread::current().id());
        // Don't join if we're calling from inside the selector thread itself
        // (this happens when Context drops during selector cleanup)
        if id != std::thread::current().id()
            && let Some(th) = data.th.lock().take()
        {
            let _ = th.join();
        }
    }

    Ok(())
}

pub(crate) fn send_command(context: &Arc<Context>, cmd: Command) -> Result<()> {
    if let Command::Halt = cmd {
        return halt();
    }

    let data = SELECTOR_DATA.get_or_init(|| {
        let (tx, rx) = crossbeam_channel::unbounded();
        let guard =
            super::guard_condition::GuardCondition::new(context.clone()).expect("guard cond");
        let ctx = context.clone();
        let guard2 = guard.clone();
        let th = thread::spawn(move || select(ctx, guard2, rx));
        SelectorData {
            tx,
            th: Mutex::new(Some(th)),
            cond: guard,
        }
    });

    data.tx
        .send(cmd)
        .map_err(|_| crate::error::Error::ChannelClosed)?;
    data.cond.trigger()
}

fn select(context: Arc<Context>, guard: GuardCondition, rx: Receiver<Command>) -> Result<()> {
    let mut selector = super::Selector::new(context)?;

    selector.add_guard_condition(&guard, None, false);

    loop {
        for cmd in rx.try_iter() {
            match cmd {
                Command::Subscription(s, h) => selector.add_rcl_subscription(s, Some(h), true),
                Command::RemoveSubscription(s) => selector.remove_rcl_subscription(&s),
                Command::Server(s, h) => selector.add_server_data(s, Some(h), true),
                Command::RemoveServer(s) => selector.remove_server_data(&s),
                Command::Client(c, h) => selector.add_client_data(c, Some(h)),
                Command::RemoveClient(c) => selector.remove_client_data(&c),
                Command::ActionClient {
                    data,
                    feedback,
                    status,
                    goal,
                    cancel,
                    result,
                } => selector.add_action_client_data(
                    data,
                    Some(feedback),
                    Some(status),
                    Some(goal),
                    Some(cancel),
                    Some(result),
                ),
                Command::RemoveActionClient(c) => selector.remove_action_client_data(&c),
                Command::ActionServer {
                    data,
                    goal,
                    cancel,
                    result,
                } => selector.add_action_server_data(data, Some(goal), Some(cancel), Some(result)),
                Command::RemoveActionServer(s) => selector.remove_action_server_data(&s),
                Command::ConditionVar(c, h) => selector.add_guard_condition(&c, Some(h), true),
                Command::RemoveConditionVar(c) => selector.remove_guard_condition(&c),
                Command::Halt => return Ok(()),
            }
        }
        if selector
            .wait_timeout(std::time::Duration::from_secs(1))
            .is_err()
        {
            for (_, h) in selector.subscriptions.iter_mut() {
                if let Some(handler) = &mut h.handler {
                    (*handler)();
                }
            }
            for (_, h) in selector.services.iter_mut() {
                if let Some(handler) = &mut h.handler {
                    (*handler)();
                }
            }
            for (_, h) in selector.clients.iter_mut() {
                if let Some(handler) = &mut h.handler {
                    (*handler)();
                }
            }
            for (_, h) in selector.cond.iter_mut() {
                if let Some(handler) = &mut h.handler {
                    (*handler)();
                }
            }
            return Ok(());
        }
    }
}
