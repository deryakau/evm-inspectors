use alloy_primitives::{Address, Log, B256, U256};
use revm::{
    inspectors::CustomPrintTracer,
    interpreter::{CallInputs, CallOutcome, CreateInputs, CreateOutcome, Interpreter},
    primitives::Env,
    Database, EvmContext, Inspector,
};
use std::fmt::Debug;

/// Enum representing different hooks for inspecting blockchain transactions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hook {
    #[default]
    /// No hook.
    None,
    /// Hook on a specific block number.
    Block(u64),
    /// Hook on a specific transaction hash.
    Transaction(B256),
    /// Hook on every transaction in a block.
    All,
}

/// An inspector that manages a stack of multiple inspectors and executes them in sequence.
#[derive(Clone, Default)]
pub struct InspectorStack {
    /// An optional inspector that prints opcode traces to the console.
    pub custom_print_tracer: Option<CustomPrintTracer>,
    /// The hook configuration for the inspector stack.
    pub hook: Hook,
}

impl Debug for InspectorStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InspectorStack")
            .field("custom_print_tracer", &self.custom_print_tracer.is_some())
            .field("hook", &self.hook)
            .finish()
    }
}

impl InspectorStack {
    /// Creates a new `InspectorStack` instance based on the provided configuration.
    pub fn new(config: InspectorStackConfig) -> Self {
        let mut stack = Self {
            hook: config.hook,
            ..Default::default()
        };

        if config.use_printer_tracer {
            stack.custom_print_tracer = Some(CustomPrintTracer::default());
        }

        stack
    }

    /// Determines if the inspector should be used based on the environment and transaction hash.
    pub fn should_inspect(&self, env: &Env, tx_hash: B256) -> bool {
        match self.hook {
            Hook::None => false,
            Hook::Block(block) => env.block.number.to::<u64>() == block,
            Hook::Transaction(hash) => hash == tx_hash,
            Hook::All => true,
        }
    }
}

/// Configuration struct for the `InspectorStack`.
#[derive(Clone, Copy, Debug, Default)]
pub struct InspectorStackConfig {
    /// Enables the opcode trace printer in the inspector.
    pub use_printer_tracer: bool,

    /// Hook configuration for the inspector stack.
    pub hook: Hook,
}

/// Macro for calling a method on multiple inspectors without dynamic dispatch.
#[macro_export]
macro_rules! call_inspectors {
    ([$($inspector:expr),+ $(,)?], |$id:ident $(,)?| $call:expr $(,)?) => {
        $(
            if let Some($id) = $inspector {
                $call
            }
        )+
    };
}

impl<DB> Inspector<DB> for InspectorStack
where
    DB: Database,
{
    fn initialize_interp(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        call_inspectors!([&mut self.custom_print_tracer], |inspector| {
            inspector.initialize_interp(interp, context);
        });
    }

    fn step(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        call_inspectors!([&mut self.custom_print_tracer], |inspector| {
            inspector.step(interp, context);
        });
    }

    fn step_end(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        call_inspectors!([&mut self.custom_print_tracer], |inspector| {
            inspector.step_end(interp, context);
        });
    }

    fn log(&mut self, context: &mut EvmContext<DB>, log: &Log) {
        call_inspectors!([&mut self.custom_print_tracer], |inspector| {
            inspector.log(context, log);
        });
    }

    fn call(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &mut CallInputs,
    ) -> Option<CallOutcome> {
        call_inspectors!([&mut self.custom_print_tracer], |inspector| {
            inspector.call(context, inputs)
        }).flatten()
    }

    fn call_end(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &CallInputs,
        outcome: CallOutcome,
    ) -> CallOutcome {
        call_inspectors!([&mut self.custom_print_tracer], |inspector| {
            let new_ret = inspector.call_end(context, inputs, outcome.clone());

            // If the inspector returns a different result or a revert with a non-empty message,
            // we assume it wants to provide additional information.
            if new_ret != outcome {
                return new_ret;
            }
        });

        outcome
    }

    fn create(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &mut CreateInputs,
    ) -> Option<CreateOutcome> {
        call_inspectors!([&mut self.custom_print_tracer], |inspector| {
            inspector.create(context, inputs)
        }).flatten()
    }

    fn create_end(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &CreateInputs,
        outcome: CreateOutcome,
    ) -> CreateOutcome {
        call_inspectors!([&mut self.custom_print_tracer], |inspector| {
            let new_ret = inspector.create_end(context, inputs, outcome.clone());

            // If the inspector returns a different result or a revert with a non-empty message,
            // we assume it wants to provide additional information.
            if new_ret != outcome {
                return new_ret;
            }
        });

        outcome
    }

    fn selfdestruct(&mut self, contract: Address, target: Address, value: U256) {
        call_inspectors!([&mut self.custom_print_tracer], |inspector| {
            inspector.selfdestruct(contract, target, value);
        });
    }
}
