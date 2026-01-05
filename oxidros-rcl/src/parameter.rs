//! Parameter server.
//!
//! # Examples
//!
//! ## Wait update by callback
//!
//! ```
//! use oxidros_rcl::{
//!     context::Context,
//!     logger::Logger,
//!     parameter::{ParameterServer, Value, Parameter, Descriptor},
//!     pr_info,
//! };
//!
//! // Create a context and a node.
//! let ctx = Context::new().unwrap();
//! let node = ctx.create_node("param_server", None, Default::default()).unwrap();
//!
//! // Create a parameter server.
//! let param_server = node.create_parameter_server().unwrap();
//! {
//!     // Set parameters.
//!     let mut params = param_server.params.write(); // Write lock
//!
//!     // Statically typed parameter.
//!     params.set_parameter(
//!         "my_flag".to_string(),                     // parameter name
//!         Value::Bool(false),                        // value
//!         false,                                     // read only?
//!         Some("my flag's description".to_string()), // description
//!     ).unwrap();
//!
//!     // Dynamically typed parameter.
//!     params.set_dynamically_typed_parameter(
//!         "my_dynamic_type_flag".to_string(), // parameter name
//!         Value::Bool(false),                 // value
//!         false,                              // read only?
//!         Some("my dynamic type flag's description".to_string()), // description
//!     ).unwrap();
//!
//!     // Add Directly from Parameter struct
//!     let parameter_to_set = Parameter {
//!         descriptor: Descriptor {
//!             description: "my parameter description".to_string(),                       // parameter description
//!             additional_constraints: "my parameter addutional_constraints".to_string(), // parameter additional constraints
//!             read_only: false,                                                          // read only ?
//!             dynamic_typing: false,                                                     // static or Dynamic
//!             floating_point_range: None,                                                // floating point range
//!             integer_range: None,                                                       // integer point range
//!         },
//!         value: Value::Bool(false),                                                     // value
//!     };
//!
//!     let _= params.add_parameter(
//!         ("my parameter").to_string(), // name
//!         parameter_to_set,             // parameter
//!     );
//! }
//!
//! // Create a logger and a selector.
//! let logger = Logger::new("param_server");
//! let mut selector = ctx.create_selector().unwrap();
//!
//! // Add a callback function to the parameter server.
//! selector.add_parameter_server(
//!     param_server,
//!     Box::new(move |params, updated| {
//!         // Print updated parameters.
//!         let mut keys = String::new();
//!         for key in updated.iter() {
//!             let value = &params.get_parameter(key).unwrap().value;
//!             keys = format!("{keys}{key} = {}, ", value);
//!         }
//!         pr_info!(logger, "updated parameters: {keys}");
//!     }),
//! );
//!
//! // Do spin to wait update.
//! // loop {
//! //    selector.wait()?;
//! // }
//! ```
//!
//! ## Asynchronous wait
//!
//! ```
//! use oxidros_rcl::{
//!     context::Context,
//!     error::DynError,
//!     logger::Logger,
//!     parameter::{ParameterServer, Value, Parameter, Descriptor},
//!     pr_info,
//! };
//!
//! // Create a context and a node.
//! let ctx = Context::new().unwrap();
//! let node = ctx.create_node("async_param_server", None, Default::default()).unwrap();
//!
//! // Create a parameter server.
//! let mut param_server = node.create_parameter_server().unwrap();
//! {
//!     // Set parameters.
//!     let mut params = param_server.params.write(); // Write lock
//!
//!     // Statically typed parameter.
//!     params.set_parameter(
//!         "my_flag".to_string(),                     // parameter name
//!         Value::Bool(false),                        // value
//!         false,                                     // read only?
//!         Some("my flag's description".to_string()), // description
//!     ).unwrap();
//!
//!     // Dynamically typed parameter.
//!     params.set_dynamically_typed_parameter(
//!         "my_dynamic_type_flag".to_string(), // parameter name
//!         Value::Bool(false),                 // value
//!         false,                              // read only?
//!         Some("my dynamic type flag's description".to_string()), // description
//!     ).unwrap();
//!
//!     // Add Directly from Parameter struct
//!     let parameter_to_set = Parameter {
//!         descriptor: Descriptor {
//!             description: "my parameter description".to_string(),                       // parameter description
//!             additional_constraints: "my parameter addutional_constraints".to_string(), // parameter additional constraints
//!             read_only: false,                                                          // read only ?
//!             dynamic_typing: false,                                                     // static or Dynamic
//!             floating_point_range: None,                                                // floating point range
//!             integer_range: None,                                                       // integer point range
//!         },
//!         value: Value::Bool(false),                                                     // value
//!     };
//!
//!     let _ = params.add_parameter(
//!         ("my parameter").to_string(), // name
//!         parameter_to_set,             // parameter
//!     );
//! }
//!
//! async fn run_wait(mut param_server: ParameterServer) {
//!     loop {
//!         // Create a logger.
//!         let logger = Logger::new("async_param_server");
//!
//!         // Wait update asynchronously.
//!         let updated = param_server.wait().await.unwrap();
//!
//!         let params = param_server.params.read(); // Read lock
//!
//!         // Print updated parameters.
//!         let mut keys = String::new();
//!         for key in updated.iter() {
//!             let value = &params.get_parameter(key).unwrap().value;
//!             keys = format!("{keys}{key} = {}, ", value);
//!         }
//!         pr_info!(logger, "updated parameters: {keys}");
//!     }
//! }
//!
//! // let rt = tokio::runtime::Runtime::new().unwrap(); --- IGNORE ---
//! // rt.block_on(run_wait(param_server)); // Spawn an asynchronous task.
//! ```

use crate::{
    error::{DynError, OResult},
    is_halt,
    logger::{Logger, pr_error_in, pr_fatal_in},
    msg::{
        RosString, RosStringSeq, U8Seq,
        interfaces::rcl_interfaces::{
            self,
            msg::{
                ParameterDescriptor, ParameterDescriptorSeq, ParameterValue, ParameterValueSeq,
                SetParametersResult, SetParametersResultSeq,
            },
            srv::{
                describe_parameters::{DescribeParameters, DescribeParameters_Response},
                get_parameter_types::{GetParameterTypes, GetParameterTypes_Response},
                get_parameters::{GetParameters, GetParameters_Response},
                list_parameters::{ListParameters, ListParameters_Response},
                set_parameters::{SetParameters, SetParameters_Response},
                set_parameters_atomically::{
                    SetParametersAtomically, SetParametersAtomically_Response,
                },
            },
        },
    },
    node::Node,
    qos::Profile,
    selector::{
        Selector,
        async_selector::{Command, SELECTOR},
        guard_condition::GuardCondition,
    },
    signal_handler::Signaled,
};
pub use oxidros_core::parameter::*;
use oxidros_core::selector::CallbackResult;
use parking_lot::RwLock;
use std::{cell::Cell, collections::BTreeSet, future::Future, rc::Rc, sync::Arc, task::Poll};

/// Parameter server.
///
/// # Example
///
/// ```
/// use oxidros_rcl::{
///     context::Context,
///     parameter::{ParameterServer, Value, Parameter, Descriptor},
/// };
///
/// // Create a context and a node.
/// let ctx = Context::new().unwrap();
/// let node = ctx.create_node("param_server_ex", None, Default::default()).unwrap();
///
/// // Create a parameter server.
/// let param_server = node.create_parameter_server().unwrap();
/// {
///     // Set parameters.
///     let mut params = param_server.params.write(); // Write lock
///
///     // Statically typed parameter.
///     params.set_parameter(
///         "my_flag".to_string(),                     // parameter name
///         Value::Bool(false),                        // value
///         false,                                     // read only?
///         Some("my flag's description".to_string()), // description
///     ).unwrap();
///
///     // Dynamically typed parameter.
///     params.set_dynamically_typed_parameter(
///         "my_dynamic_type_flag".to_string(), // parameter name
///         Value::Bool(false),                 // value
///         false,                              // read only?
///         Some("my dynamic type flag's description".to_string()), // description
///     ).unwrap();
///
///     // Add Directly from Parameter struct
///     let parameter_to_set = Parameter {
///         descriptor: Descriptor {
///             description: "my parameter description".to_string(),                       // parameter description
///             additional_constraints: "my parameter addutional_constraints".to_string(), // parameter additional constraints
///             read_only: false,                                                          // read only ?
///             dynamic_typing: false,                                                     // static or Dynamic
///             floating_point_range: None,                                                // floating point range
///             integer_range: None,                                                       // integer point range
///         },
///         value: Value::Bool(false),                                                     // value
///     };
///
///     let _ = params.add_parameter(
///         ("my parameter").to_string(), // name
///         parameter_to_set,             // parameter
///     );
/// }
/// ```
pub struct ParameterServer {
    pub params: Arc<RwLock<Parameters>>,
    handler: Option<std::thread::JoinHandle<Result<(), DynError>>>,
    cond_halt: GuardCondition,
    pub(crate) cond_callback: GuardCondition,
    node: Arc<Node>,
}

impl ParameterServer {
    pub(crate) fn new(node: Arc<Node>) -> Result<Self, DynError> {
        let params_value = {
            let mut guard = crate::rcl::MT_UNSAFE_FN.lock();
            let fqn = node.get_fully_qualified_name()?;
            let arguments = unsafe { &mut (*node.context.as_ptr_mut()).global_arguments };
            guard.parameter_map(fqn.as_str(), arguments)?
        };
        let mut params = Parameters::new();
        for (k, v) in params_value.into_iter() {
            let _ = params.set_parameter(k, v, false, None);
        }
        let params = Arc::new(RwLock::new(params));
        let ps = params.clone();
        let n = node.clone();

        let cond_halt = GuardCondition::new(node.context.clone())?;
        let cond_halt_cloned = cond_halt.clone();

        let cond_callback = GuardCondition::new(node.context.clone())?;
        let cond_callback_cloned = cond_callback.clone();

        let handler =
            std::thread::spawn(move || param_server(n, ps, cond_halt_cloned, cond_callback_cloned));

        Ok(Self {
            params,
            handler: Some(handler),
            cond_halt,
            cond_callback,
            node,
        })
    }

    pub fn wait(&mut self) -> AsyncWait<'_> {
        AsyncWait {
            param_server: self,
            state: WaitState::Init,
        }
    }
}

impl Drop for ParameterServer {
    fn drop(&mut self) {
        if self.cond_halt.trigger().is_ok()
            && let Some(handler) = self.handler.take()
        {
            let _ = handler.join();
        }
    }
}

fn param_server(
    node: Arc<Node>,
    params: Arc<RwLock<Parameters>>,
    cond_halt: GuardCondition,
    cond_callback: GuardCondition,
) -> Result<(), DynError> {
    if let Ok(mut selector) = node.context.create_selector() {
        add_srv_list(&node, &mut selector, params.clone())?;
        add_srv_set(
            &node,
            &mut selector,
            params.clone(),
            "set_parameters",
            cond_callback.clone(),
        )?;
        add_srv_set_atomic(
            &node,
            &mut selector,
            params.clone(),
            "set_parameters_atomically",
            cond_callback,
        )?;
        add_srv_get(&node, &mut selector, params.clone())?;
        add_srv_get_types(&node, &mut selector, params.clone())?;
        add_srv_describe(&node, &mut selector, params)?;

        let is_halt = Rc::new(Cell::new(false));
        let is_halt_cloned = is_halt.clone();

        selector.add_guard_condition(
            &cond_halt,
            Some(Box::new(move || {
                is_halt_cloned.set(true);
                CallbackResult::Remove
            })),
            false,
        );

        while !is_halt.get() {
            selector.wait()?;
        }
    } else {
        let logger = Logger::new("oxidros");
        pr_error_in!(logger, "failed to start a parameter server");
    }

    Ok(())
}

fn add_srv_set(
    node: &Arc<Node>,
    selector: &mut Selector,
    params: Arc<RwLock<Parameters>>,
    service_name: &str,
    cond_callback: GuardCondition,
) -> OResult<()> {
    let name = node.get_name()?;
    let srv_set = node.create_server::<SetParameters>(
        &format!("{name}/{service_name}"),
        Some(Profile::default()),
    )?;

    selector.add_server(
        srv_set,
        Box::new(move |req, _| {
            let mut results = if let Some(seq) = SetParametersResultSeq::new(req.parameters.len()) {
                seq
            } else {
                let response = SetParameters_Response::new().unwrap();
                return response;
            };

            let slice = results.as_mut_slice();

            let mut updated = 0;
            {
                let mut guard = params.write();
                for (i, param) in req.parameters.iter().enumerate() {
                    let key = param.name.to_string();
                    let val: Value = (&param.value).into();

                    if let Some(original) = guard.params.get_mut(&key) {
                        if original.descriptor.read_only {
                            let reason = format!("{} is read only", key);
                            slice[i].reason.assign(&reason);
                            slice[i].successful = false;
                            continue;
                        }

                        if !original.check_range(&val) {
                            let reason = format!("{} is not in the range", key);
                            slice[i].reason.assign(&reason);
                            slice[i].successful = false;
                            continue;
                        }

                        if original.descriptor.dynamic_typing || original.value.type_check(&val) {
                            original.value = val;
                            slice[i].successful = true;
                            updated += 1;
                            guard.updated.insert(key);
                        } else {
                            let reason = format!(
                                "failed type checking: dst = {}, src = {}",
                                original.value.type_name(),
                                val.type_name()
                            );
                            slice[i].reason.assign(&reason);
                            slice[i].successful = false;
                        }
                    } else {
                        let reason = format!("no such parameter: name = {}", key);
                        slice[i].reason.assign(&reason);
                        slice[i].successful = false;
                    }
                }
            }

            if updated > 0 && cond_callback.trigger().is_err() {
                let logger = Logger::new("oxidros");
                pr_fatal_in!(
                    logger,
                    "{}:{}: failed to trigger a condition variable",
                    file!(),
                    line!()
                );
            }

            let mut response = SetParameters_Response::new().unwrap();
            response.results = results;

            response
        }),
    );

    Ok(())
}

fn add_srv_set_atomic(
    node: &Arc<Node>,
    selector: &mut Selector,
    params: Arc<RwLock<Parameters>>,
    service_name: &str,
    cond_callback: GuardCondition,
) -> OResult<()> {
    let name = node.get_name()?;
    let srv_set = node.create_server::<SetParametersAtomically>(
        &format!("{name}/{service_name}"),
        Some(Profile::default()),
    )?;

    selector.add_server(
        srv_set,
        Box::new(move |req, _| {
            let mut results = if let Some(seq) = SetParametersResult::new() {
                seq
            } else {
                let response = SetParametersAtomically_Response::new().unwrap();
                return response;
            };

            let mut updated = 0;
            {
                let mut guard = params.write();
                for param in req.parameters.iter() {
                    let key = param.name.to_string();
                    let val: Value = (&param.value).into();

                    if let Some(original) = guard.params.get_mut(&key) {
                        if original.descriptor.read_only {
                            let reason = format!("{} is read only", key);
                            results.reason.assign(&reason);
                            results.successful = false;
                            break;
                        }

                        if !original.check_range(&val) {
                            let reason = format!("{} is not in the range", key);
                            results.reason.assign(&reason);
                            results.successful = false;
                            break;
                        }

                        if original.descriptor.dynamic_typing || original.value.type_check(&val) {
                            original.value = val;
                            results.successful = true;
                            updated += 1;
                            guard.updated.insert(key);
                        } else {
                            let reason = format!(
                                "failed type checking: dst = {}, src = {}",
                                original.value.type_name(),
                                val.type_name()
                            );
                            results.reason.assign(&reason);
                            results.successful = false;
                            break;
                        }
                    } else {
                        let reason = format!("no such parameter: name = {}", key);
                        results.reason.assign(&reason);
                        results.successful = false;
                        break;
                    }
                }
            }

            if updated > 0 && cond_callback.trigger().is_err() {
                let logger = Logger::new("oxidros");
                pr_fatal_in!(
                    logger,
                    "{}:{}: failed to trigger a condition variable",
                    file!(),
                    line!()
                );
            }

            let mut response = SetParametersAtomically_Response::new().unwrap();
            response.result = results;

            response
        }),
    );

    Ok(())
}

fn add_srv_get(
    node: &Arc<Node>,
    selector: &mut Selector,
    params: Arc<RwLock<Parameters>>,
) -> OResult<()> {
    let name = node.get_name()?;
    let srv_get = node.create_server::<GetParameters>(
        &format!("{name}/get_parameters"),
        Some(Profile::default()),
    )?;
    selector.add_server(
        srv_get,
        Box::new(move |req, _| {
            let mut result = Vec::new();

            let gurad = params.read();
            for name in req.names.iter() {
                let key = name.to_string();
                if let Some(param) = gurad.params.get(&key) {
                    result.push(&param.value);
                }
            }

            let mut response = GetParameters_Response::new().unwrap();

            if let Some(mut seq) = ParameterValueSeq::new(result.len()) {
                seq.iter_mut()
                    .zip(result.iter())
                    .for_each(|(dst, src)| *dst = (*src).into());
                response.values = seq;
            }

            response
        }),
    );

    Ok(())
}

macro_rules! unwrap_or_continue {
    ($e:expr) => {
        if let Some(x) = $e {
            x
        } else {
            continue;
        }
    };
}

fn add_srv_describe(
    node: &Arc<Node>,
    selector: &mut Selector,
    params: Arc<RwLock<Parameters>>,
) -> OResult<()> {
    let name = node.get_name()?;
    let srv_describe = node.create_server::<DescribeParameters>(
        &format!("{name}/describe_parameters"),
        Some(Profile::default()),
    )?;
    selector.add_server(
        srv_describe,
        Box::new(move |req, _| {
            let gurad = params.read();

            let mut results = Vec::new();
            for name in req.names.iter() {
                let key = name.to_string();
                if let Some(param) = gurad.params.get(&key) {
                    let value: ParameterValue = (&param.value).into();
                    let description =
                        unwrap_or_continue!(RosString::new(&param.descriptor.description));
                    let additional_constraints = unwrap_or_continue!(RosString::new(
                        &param.descriptor.additional_constraints
                    ));

                    let integer_range = if let Some(range) = &param.descriptor.integer_range {
                        let mut int_range =
                            unwrap_or_continue!(rcl_interfaces::msg::IntegerRangeSeq::new(1));
                        int_range.as_mut_slice()[0] = range.into();
                        int_range
                    } else {
                        unwrap_or_continue!(rcl_interfaces::msg::IntegerRangeSeq::new(0))
                    };

                    let floating_point_range = if let Some(range) =
                        &param.descriptor.floating_point_range
                    {
                        let mut f64_range =
                            unwrap_or_continue!(rcl_interfaces::msg::FloatingPointRangeSeq::new(1));
                        f64_range.as_mut_slice()[0] = range.into();
                        f64_range
                    } else {
                        unwrap_or_continue!(rcl_interfaces::msg::FloatingPointRangeSeq::new(0))
                    };

                    let result = ParameterDescriptor {
                        name: unwrap_or_continue!(RosString::new(&key)),
                        r#type: value.r#type,
                        description,
                        additional_constraints,
                        read_only: param.descriptor.read_only,
                        dynamic_typing: param.descriptor.dynamic_typing,
                        integer_range,
                        floating_point_range,
                    };
                    results.push(result);
                }
            }

            let mut response = DescribeParameters_Response::new().unwrap();
            if let Some(mut seq) = ParameterDescriptorSeq::new(results.len()) {
                seq.iter_mut()
                    .zip(results)
                    .for_each(|(dst, src)| *dst = src);
                response.descriptors = seq;
            };

            response
        }),
    );

    Ok(())
}

fn add_srv_get_types(
    node: &Arc<Node>,
    selector: &mut Selector,
    params: Arc<RwLock<Parameters>>,
) -> OResult<()> {
    let name = node.get_name()?;
    let srv_get_types = node.create_server::<GetParameterTypes>(
        &format!("{name}/get_parameter_types"),
        Some(Profile::default()),
    )?;
    selector.add_server(
        srv_get_types,
        Box::new(move |req, _| {
            let mut types = Vec::new();

            let gurad = params.read();
            for name in req.names.iter() {
                let key = name.to_string();
                if let Some(param) = gurad.params.get(&key) {
                    let v: ParameterValue = (&param.value).into();
                    types.push(v.r#type);
                } else {
                    types.push(0);
                }
            }

            let mut response = GetParameterTypes_Response::new().unwrap();
            if let Some(mut seq) = U8Seq::new(types.len()) {
                seq.iter_mut()
                    .zip(types.iter())
                    .for_each(|(dst, src)| *dst = *src);
                response.types = seq;
            }

            response
        }),
    );

    Ok(())
}

fn add_srv_list(
    node: &Arc<Node>,
    selector: &mut Selector,
    params: Arc<RwLock<Parameters>>,
) -> OResult<()> {
    let name = node.get_name()?;
    let srv_list = node.create_server::<ListParameters>(
        &format!("{name}/list_parameters"),
        Some(Profile::default()),
    )?;
    selector.add_server(
        srv_list,
        Box::new(move |req, _| {
            let separator = b'.';

            let mut result = Vec::new();
            let mut result_prefix = Vec::new();
            let prefixes: Vec<String> = req
                .prefixes
                .iter()
                .map(|prefix| prefix.get_string())
                .collect();

            let guard = params.write();

            for (k, _v) in guard.params.iter() {
                let cnt = k.as_bytes().iter().filter(|c| **c == separator).count();
                let get_all = prefixes.is_empty() && req.depth == 0 || cnt < req.depth as usize;

                let matches = prefixes.iter().find(|prefix| {
                    if k == *prefix {
                        true
                    } else {
                        let mut prefix_sep = (*prefix).clone();
                        prefix_sep.push(separator as char);

                        if k.starts_with(&prefix_sep) {
                            if req.depth == 0 {
                                true
                            } else {
                                let cnt = k.as_bytes()[..prefix.len()]
                                    .iter()
                                    .filter(|c| **c == separator)
                                    .count();
                                req.depth == 0 || cnt < req.depth as usize
                            }
                        } else {
                            false
                        }
                    }
                });

                if get_all || matches.is_some() {
                    result.push(k);
                    let separated: Vec<_> = k.split(separator as char).collect();
                    if separated.len() > 1 {
                        let prefix = separated[..separated.len() - 1].iter().fold(
                            String::new(),
                            |mut s, item| {
                                s.push_str(item);
                                s
                            },
                        );
                        if !result_prefix.contains(&prefix) {
                            result_prefix.push(prefix);
                        }
                    }
                }
            }

            let mut response = ListParameters_Response::new().unwrap();
            if let (Some(mut seq_names), Some(mut seq_prefixes)) = (
                RosStringSeq::<0, 0>::new(result.len()),
                RosStringSeq::<0, 0>::new(result_prefix.len()),
            ) {
                seq_names
                    .iter_mut()
                    .zip(result.iter())
                    .for_each(|(dst, src)| {
                        dst.assign(src);
                    });

                seq_prefixes
                    .iter_mut()
                    .zip(result_prefix.iter())
                    .for_each(|(dst, src)| {
                        dst.assign(src);
                    });

                response.result.names = seq_names;
                response.result.prefixes = seq_prefixes;
            }

            response
        }),
    );

    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum WaitState {
    Init,
    Waiting,
}

pub struct AsyncWait<'a> {
    param_server: &'a ParameterServer,
    state: WaitState,
}

impl<'a> Future for AsyncWait<'a> {
    type Output = Result<BTreeSet<String>, DynError>;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if is_halt() {
            return Poll::Ready(Err(Signaled.into()));
        }

        match self.state {
            WaitState::Init => {
                let mut waker = Some(cx.waker().clone());
                let mut guard = SELECTOR.lock();

                if let Err(e) = guard.send_command(
                    &self.param_server.node.context,
                    Command::ConditionVar(
                        self.param_server.cond_callback.clone(),
                        Box::new(move || {
                            let w = waker.take().unwrap();
                            w.wake();
                            CallbackResult::Remove
                        }),
                    ),
                ) {
                    Poll::Ready(Err(e))
                } else {
                    self.get_mut().state = WaitState::Waiting;
                    Poll::Pending
                }
            }
            WaitState::Waiting => {
                let mut guard = self.param_server.params.write();
                let updated = guard.take_updated();
                Poll::Ready(Ok(updated))
            }
        }
    }
}

impl<'a> Drop for AsyncWait<'a> {
    fn drop(&mut self) {
        let mut guard = SELECTOR.lock();
        if guard
            .send_command(
                &self.param_server.node.context,
                Command::RemoveConditionVar(self.param_server.cond_callback.clone()),
            )
            .is_err()
        {}
    }
}
