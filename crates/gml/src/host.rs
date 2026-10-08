//! The embedding interface between the interpreter and the game world.

use crate::builtins::{FunctionId, VariableId};
use crate::error::Error;
use crate::value::Value;
use crate::vars::VarTable;
use crate::vm::Vm;

/// Special instance values (spec §10.1).
pub mod special {
    /// The current instance.
    pub const SELF: i32 = -1;
    /// The other instance.
    pub const OTHER: i32 = -2;
    /// All instances.
    pub const ALL: i32 = -3;
    /// No instance.
    pub const NOONE: i32 = -4;
    /// The global variable scope.
    pub const GLOBAL: i32 = -5;
    /// The current program's `var` locals.
    pub const LOCAL: i32 = -7;
    /// First instance id; values `0..FIRST_INSTANCE_ID` are object indices.
    pub const FIRST_INSTANCE_ID: i32 = 100_000;
}

/// The execution context of running code: who `self` and `other` are.
///
/// `self_id` / `other_id` are always concrete instance ids (≥ 100000) or
/// [`special::NOONE`] when there is no such instance (e.g. code run outside any
/// instance, or no collision partner).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ctx {
    /// The instance executing the code.
    pub self_id: i32,
    /// The "other" instance.
    pub other_id: i32,
    /// Value of `argument_relative` for this invocation (true only for scripts run
    /// from an action whose "Relative" box is checked).
    pub argument_relative: bool,
}

impl Ctx {
    /// A context with `self` and `other` set and `argument_relative = false`.
    pub fn new(self_id: i32, other_id: i32) -> Self {
        Ctx {
            self_id,
            other_id,
            argument_relative: false,
        }
    }
}

/// Callbacks through which the interpreter reaches the game world.
///
/// The runner implements this trait. The interpreter ([`Vm`]) owns only what is
/// independent of the world: global user variables, the `globalvar` set, the random
/// generator and data structures. Everything else — instances and their variables,
/// built-in variables, world-related functions, and script code — lives with the
/// host.
///
/// # Re-entrancy
/// [`call_function`](Host::call_function) and [`call_script`](Host::call_script)
/// receive the `&mut Vm`, so a host may run further GML from inside them (to call a
/// script, run a Create event from `instance_create`, `event_perform`, ...) by calling
/// [`Vm::run`] with `self` as the host. Keep compiled programs in `Rc` and clone the
/// `Rc` before calling `run` so no borrow of the host is held.
///
/// # Instance numbering
/// Values `0..100000` passed as instance targets are object indices; values
/// `>= 100000` are instance ids (spec §10.1).
pub trait Host {
    // ------------------------------------------------------------ compile time

    /// Resolves an identifier that is not a keyword, `var` local, built-in variable or
    /// built-in constant: user-defined constants and resource names (sprites, sounds,
    /// backgrounds, paths, scripts, fonts, timelines, objects, rooms) → their value.
    /// Called while compiling. Return `None` for "not a constant" (it then becomes a
    /// user variable).
    fn resolve_constant(&self, name: &str) -> Option<Value> {
        let _ = name;
        None
    }

    /// Resolves the name in a call `name(...)` that is not a built-in function to a
    /// script index. Called while compiling. `None` makes the call a compile error
    /// ("Unknown function or script").
    fn resolve_script(&self, name: &str) -> Option<i32> {
        let _ = name;
        None
    }

    // ------------------------------------------------------------ instances

    /// The user-variable table of instance `id`, or `None` if no such instance
    /// exists. Destroyed instances may stay accessible until the end of the event
    /// (spec §10.3); that is the host's choice.
    fn instance_vars(&mut self, id: i32) -> Option<&mut VarTable>;

    /// Direct access to a plain real built-in instance variable (one whose write
    /// has no effect beyond storing the value, such as `x`), for `x += 1` style
    /// updates. `None` (the default) makes the interpreter use
    /// [`get_builtin`](Host::get_builtin) / [`set_builtin`](Host::set_builtin).
    fn builtin_real_mut(&mut self, _inst: i32, _var: VariableId) -> Option<&mut f64> {
        None
    }

    /// The value of a real, non-array built-in instance variable of `inst` (such
    /// as `x`) when the host can read it directly; `None` (the default) makes the
    /// interpreter use [`get_builtin`](Host::get_builtin).
    fn builtin_real(&mut self, _inst: i32, _var: VariableId) -> Option<f64> {
        None
    }

    /// The instance that a read through instance id `id` (`id.x`) addresses. The
    /// default is `id` itself; a host may redirect it (gm-runner: the new record
    /// made by `instance_change`, which `with (id)` does not find yet).
    fn read_target(&mut self, id: i32) -> i32 {
        id
    }

    /// Whether instance `id` still exists and is active (used to skip instances
    /// destroyed or deactivated during a `with` loop).
    fn instance_exists(&mut self, id: i32) -> bool;

    /// Appends the ids of the active instances designated by `target`, in instance
    /// order:
    /// * an object index (`0..100000`): instances of that object and its descendants;
    /// * [`special::ALL`]: every active instance;
    /// * an instance id (`>= 100000`): that instance if it exists and is active.
    ///
    /// Other values are never passed.
    fn select_instances(&mut self, target: i32, out: &mut Vec<i32>);

    /// The first instance designated by `target` (same meaning as in
    /// [`select_instances`](Host::select_instances)). The default implementation uses
    /// `select_instances`; override it for speed.
    fn first_instance(&mut self, target: i32) -> Option<i32> {
        let mut v = Vec::new();
        self.select_instances(target, &mut v);
        v.first().copied()
    }

    // ------------------------------------------------------------ built-in variables

    /// Reads a built-in variable of scope `instance` (of instance `inst`) or `global`
    /// (`inst` is then the current `self` and may be ignored). `index` is the array
    /// index (0 for non-array variables); declared array bounds have already been
    /// checked, except dynamic upper bounds (`instance_id`).
    fn get_builtin(
        &mut self,
        ctx: &Ctx,
        inst: i32,
        var: VariableId,
        index: i32,
    ) -> Result<Value, Error>;

    /// Writes a built-in variable (see [`get_builtin`](Host::get_builtin)). Read-only
    /// variables are rejected at compile time and never reach this method. The host
    /// checks the value type.
    fn set_builtin(
        &mut self,
        ctx: &Ctx,
        inst: i32,
        var: VariableId,
        index: i32,
        value: Value,
    ) -> Result<(), Error>;

    // ------------------------------------------------------------ calls

    /// Calls a built-in function that the interpreter does not implement itself
    /// (see [`builtins::is_native`](crate::is_native)). Argument counts have been
    /// checked at compile time. Functions without a meaningful result return real 0.
    fn call_function(
        &mut self,
        vm: &mut Vm,
        ctx: &Ctx,
        func: FunctionId,
        args: &[Value],
    ) -> Result<Value, Error>;

    /// Calls script number `script` with up to 16 arguments; used for direct calls
    /// `scr(...)` and for `script_execute`. The implementation normally looks up the
    /// compiled program and calls [`Vm::run`] with the given context (which has
    /// `argument_relative = false`). Unknown indices should return an error.
    fn call_script(
        &mut self,
        vm: &mut Vm,
        ctx: &Ctx,
        script: i32,
        args: &[Value],
    ) -> Result<Value, Error>;

    /// Reports an error that does not abandon the running code: GM 8.1 continues
    /// after errors raised by built-in functions (the function returns 0) and after
    /// errors inside `execute_string` (VERIFIED black-box, bb_lang math.*, real.*,
    /// type.readonly_id). The host records/shows the error like any other; it
    /// returns `Err` only to stop all GML (e.g. the user chose to abort the game).
    /// The default returns the error, which abandons the code as an ordinary error.
    fn soft_error(&mut self, e: Error) -> Result<(), Error> {
        Err(e)
    }

    /// Reads a file for `execute_file`. The default reads it from the file system
    /// relative to the current directory.
    fn read_file(&mut self, path: &str) -> Result<Vec<u8>, Error> {
        std::fs::read(path).map_err(|e| Error::new(format!("Cannot read file {}: {}", path, e)))
    }
}
