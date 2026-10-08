//! The virtual machine: interpreter state and public entry points.

use crate::builtins::FunctionId;
use crate::compile::{Compiler, St};
use crate::ds::DsStore;
use crate::error::{CompileError, Error};
use crate::host::{Ctx, Host};
use crate::parser;
use crate::stdlib::{self as natives, Args};
use crate::value::Value;
use crate::vars::{VarId, VarTable};
use crate::Program;
use std::collections::HashMap;
use std::rc::Rc;

/// Maps user variable names to [`VarId`]s.
#[derive(Default, Debug)]
pub struct Interner {
    map: HashMap<Rc<str>, VarId>,
    names: Vec<Rc<str>>,
}

impl Interner {
    /// Interns a name.
    pub fn intern(&mut self, name: &str) -> VarId {
        if let Some(&id) = self.map.get(name) {
            return id;
        }
        let id = VarId(self.names.len() as u32);
        let rc: Rc<str> = Rc::from(name);
        self.names.push(rc.clone());
        self.map.insert(rc, id);
        id
    }
    /// Looks up a name without interning it.
    pub fn get(&self, name: &str) -> Option<VarId> {
        self.map.get(name).copied()
    }
    /// The name of an id.
    pub fn name(&self, id: VarId) -> &str {
        &self.names[id.0 as usize]
    }
    /// Number of interned names.
    pub fn len(&self) -> usize {
        self.names.len()
    }
    /// `true` if nothing is interned.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Interpreter settings.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Global Game Settings → "Treat uninitialized variables as 0".
    pub uninitialized_as_zero: bool,
    /// Maximum nesting of program executions (script calls, `execute_string`, host
    /// re-entry). Exceeding it is a runtime error instead of a crash. Each level uses
    /// native stack (roughly 2 KB in release builds, much more in debug builds), so a
    /// host that raises this should run the interpreter on a thread with a large
    /// stack (`std::thread::Builder::stack_size`). Default: 256.
    pub max_call_depth: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            uninitialized_as_zero: false,
            max_call_depth: 256,
        }
    }
}

/// A source of the current local date-time (as a GML date value: days since
/// 1899-12-30, fraction = time of day).
pub type Clock = Box<dyn Fn() -> f64>;

/// The GML virtual machine.
///
/// Owns interned variable names, global user variables, the `globalvar` set, the
/// random generator, data structures, and settings. Game-world state is reached
/// through a [`Host`] passed to each call.
pub struct Vm {
    pub(crate) interner: Interner,
    pub(crate) globals: VarTable,
    pub(crate) globalvar: Vec<bool>,
    /// Interpreter settings.
    pub settings: Settings,
    /// Data structures (`ds_*`).
    pub ds: DsStore,
    pub(crate) rng_state: u64,
    pub(crate) rng_seed: f64,
    pub(crate) depth: u32,
    pub(crate) arg_pool: Vec<Vec<Value>>,
    pub(crate) select_pool: Vec<Vec<i32>>,
    pub(crate) locals_pool: Vec<Vec<Option<crate::vars::Var>>>,
    pub(crate) exec_cache: HashMap<Vec<u8>, Rc<Program>>,
    pub(crate) clock: Clock,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Vm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vm")
            .field("globals", &self.globals)
            .field("settings", &self.settings)
            .finish_non_exhaustive()
    }
}

const EXEC_CACHE_LIMIT: usize = 256;

impl Vm {
    /// Creates a VM with default settings and random seed 0 (call
    /// [`randomize`](Vm::randomize) for a clock-based seed).
    pub fn new() -> Self {
        let mut vm = Vm {
            interner: Interner::default(),
            globals: VarTable::new(),
            globalvar: Vec::new(),
            settings: Settings::default(),
            ds: DsStore::default(),
            rng_state: 0,
            rng_seed: 0.0,
            depth: 0,
            arg_pool: Vec::new(),
            select_pool: Vec::new(),
            locals_pool: Vec::new(),
            exec_cache: HashMap::new(),
            clock: Box::new(crate::stdlib::date::system_now),
        };
        vm.set_seed(0.0);
        vm
    }

    // ------------------------------------------------------------ compiling

    /// Compiles a program (script, code action, creation code). `name` is used in
    /// error locations. Names are resolved through `host` (see
    /// [`Host::resolve_constant`], [`Host::resolve_script`]).
    pub fn compile(
        &mut self,
        name: &str,
        src: &[u8],
        host: &dyn Host,
    ) -> Result<Rc<Program>, CompileError> {
        let ast = parser::parse_program(src)?;
        let mut c = Compiler::new(&mut self.interner, host);
        let body = c.block(&ast)?;
        Ok(Rc::new(c.finish(name, body)))
    }

    /// Compiles a single expression (action argument, user constant). Running the
    /// result returns the expression's value.
    pub fn compile_expression(
        &mut self,
        name: &str,
        src: &[u8],
        host: &dyn Host,
    ) -> Result<Rc<Program>, CompileError> {
        let ast = parser::parse_expression(src)?;
        let mut c = Compiler::new(&mut self.interner, host);
        let e = c.expr(&ast)?;
        Ok(Rc::new(c.finish(name, vec![St::Return(e)])))
    }

    // ------------------------------------------------------------ running

    /// Runs a compiled program with the given context and arguments (at most 16 are
    /// used; for code actions pass none). Returns the `return` value, or real 0.
    ///
    /// On error the program is abandoned and the error (with location) returned;
    /// the caller decides whether to show it, set `error_occurred`/`error_last`, etc.
    pub fn run(
        &mut self,
        host: &mut dyn Host,
        prog: &Program,
        ctx: Ctx,
        args: &[Value],
    ) -> Result<Value, Error> {
        self.run_program(host, prog, ctx, args)
    }

    /// Compiles and runs source text with the given context, as `execute_string`.
    /// Compile errors are returned as runtime errors.
    pub fn execute_string(
        &mut self,
        host: &mut dyn Host,
        src: &[u8],
        ctx: Ctx,
        args: &[Value],
    ) -> Result<Value, Error> {
        let prog = match self.exec_cache.get(src) {
            Some(p) => p.clone(),
            None => {
                let p = self
                    .compile("execute_string", src, &*host)
                    .map_err(Error::from)?;
                if self.exec_cache.len() >= EXEC_CACHE_LIMIT {
                    self.exec_cache.clear();
                }
                self.exec_cache.insert(src.to_vec(), p.clone());
                p
            }
        };
        self.run_program(host, &prog, ctx, args)
    }

    /// Calls a built-in function by id: natively if implemented by this crate, else
    /// through [`Host::call_function`]. The argument count is checked against the
    /// function's arity (an error is returned if it does not match).
    pub fn call_function(
        &mut self,
        host: &mut dyn Host,
        ctx: &Ctx,
        func: FunctionId,
        args: &[Value],
    ) -> Result<Value, Error> {
        let info = func.info();
        if args.len() < info.min_args as usize || args.len() > info.max_args as usize {
            return Err(Error::new(format!(
                "Wrong number of arguments to function {}.",
                info.name
            )));
        }
        match natives::native(func) {
            Some(f) => f(self, host, ctx, Args { vals: args, func }),
            None => host.call_function(self, ctx, func, args),
        }
    }

    // ------------------------------------------------------------ variables

    /// Interns a user variable name.
    pub fn intern(&mut self, name: &str) -> VarId {
        self.interner.intern(name)
    }

    /// Looks up an interned name.
    pub fn lookup_var(&self, name: &str) -> Option<VarId> {
        self.interner.get(name)
    }

    /// The name of an interned variable.
    pub fn var_name(&self, id: VarId) -> &str {
        self.interner.name(id)
    }

    /// Global user variables (`global.x`, `globalvar` names).
    pub fn globals(&self) -> &VarTable {
        &self.globals
    }

    /// Mutable access to global user variables.
    pub fn globals_mut(&mut self) -> &mut VarTable {
        &mut self.globals
    }

    /// Whether `id` has been declared with `globalvar`.
    #[inline]
    pub fn is_globalvar(&self, id: VarId) -> bool {
        self.globalvar.get(id.0 as usize).copied().unwrap_or(false)
    }

    /// Declares `id` as a `globalvar` name.
    pub fn declare_globalvar(&mut self, id: VarId) {
        let i = id.0 as usize;
        if i >= self.globalvar.len() {
            self.globalvar.resize(i + 1, false);
        }
        self.globalvar[i] = true;
    }

    /// All names declared with `globalvar`.
    pub fn globalvars(&self) -> impl Iterator<Item = VarId> + '_ {
        self.globalvar
            .iter()
            .enumerate()
            .filter(|(_, &b)| b)
            .map(|(i, _)| VarId(i as u32))
    }

    // ------------------------------------------------------------ random numbers

    /// Seeds the random generator (`random_set_seed`).
    pub fn set_seed(&mut self, seed: f64) {
        self.rng_seed = seed;
        self.rng_state = (crate::value::to_int(seed) as u64) ^ 0x853C_49E6_748F_EA9B;
    }

    /// The last seed set (`random_get_seed`).
    pub fn seed(&self) -> f64 {
        self.rng_seed
    }

    /// Seeds the generator from the system clock (`randomize`).
    pub fn randomize(&mut self) {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        self.set_seed((t % 2_147_483_647) as f64);
    }

    /// A uniformly distributed real in `[0, 1)`.
    pub fn random(&mut self) -> f64 {
        // SplitMix64
        self.rng_state = self.rng_state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    // ------------------------------------------------------------ time

    /// Replaces the clock used by `date_current_*` (e.g. for deterministic tests).
    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
    }

    /// The current local date-time value.
    pub fn now(&self) -> f64 {
        (self.clock)()
    }
}
