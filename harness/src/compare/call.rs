use soroban_env_host::{
    AddressObject, Env, EnvBase, Host, HostError, Symbol, Val,
    budget::Budget,
    testutils::{generate_account_id, generate_bytes_array},
    xdr::ContractCostType,
};

/// How the host is set up before the call.
#[derive(Clone, Copy)]
pub struct Setup {
    /// Put the module in the host's module cache first, as the network does
    /// (CAP-0065, protocol 23).
    pub cached: bool,
    /// Grant every authorization the call asks for. Without it, none.
    pub authed: bool,
}

/// What one call consumed.
#[derive(Clone, PartialEq)]
pub struct Cost {
    pub cpu: u64,
    pub mem: u64,
    /// What each cost type charged, for every type that charged anything, in the host's
    /// order. A type's `mem` is what it charged; the total `mem` is the peak, which is lower
    /// when memory was given back during the call.
    pub types: Vec<TypeCost>,
}

/// What one `ContractCostType` charged.
#[derive(Clone, Copy, PartialEq)]
pub struct TypeCost {
    pub ty: ContractCostType,
    pub cpu: u64,
    pub mem: u64,
}

/// What every cost type charged so far, in the host's order.
pub fn trackers(budget: &Budget) -> Result<Vec<TypeCost>, HostError> {
    ContractCostType::variants()
        .into_iter()
        .map(|ty| {
            let t = budget.get_tracker(ty)?;
            Ok(TypeCost {
                ty,
                cpu: t.cpu,
                mem: t.mem,
            })
        })
        .collect()
}

/// What each type charged between `before` and `after`, leaving out the types that
/// charged nothing.
pub fn charged(before: &[TypeCost], after: &[TypeCost]) -> Vec<TypeCost> {
    before
        .iter()
        .zip(after)
        .map(|(b, a)| TypeCost {
            ty: a.ty,
            cpu: a.cpu - b.cpu,
            mem: a.mem - b.mem,
        })
        .filter(|t| t.cpu > 0 || t.mem > 0)
        .collect()
}

/// Deploy `wasm` into a fresh host, set up as `setup` says. Returns the host and the
/// contract's address.
pub fn deploy(wasm: &[u8], setup: Setup) -> Result<(Host, AddressObject), HostError> {
    let host = Host::test_host_with_recording_footprint();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    let contract = host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;
    if setup.cached {
        host.ensure_module_cache_contains_host_storage_contracts()?;
    }
    if setup.authed {
        host.switch_to_recording_auth(false)?;
    }
    Ok((host, contract))
}

/// Call `func` on `contract` with the arguments `args` builds. The cost is what the call
/// alone consumed, not the building of its arguments.
pub fn call(
    host: &Host,
    contract: AddressObject,
    func: &str,
    args: impl Fn(&Host, Val) -> Result<Vec<Val>, HostError>,
) -> Result<(Val, Cost), HostError> {
    let args = args(host, contract.to_val())?;
    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(&args)?;

    let budget = host.budget_cloned();
    let base_cpu = budget.get_cpu_insns_consumed()?;
    let base_mem = budget.get_mem_bytes_consumed()?;
    let base_types = trackers(&budget)?;

    let out = host.call(contract, sym, argv)?;

    let cost = Cost {
        cpu: budget.get_cpu_insns_consumed()? - base_cpu,
        mem: budget.get_mem_bytes_consumed()? - base_mem,
        types: charged(&base_types, &trackers(&budget)?),
    };
    Ok((out, cost))
}

/// Deploy `wasm` into a fresh host, call `func` once, and hand the result to `read`.
///
/// `args` gets the host and the contract's own address, because object handles belong
/// to the host that runs the call. `read` gets the host, the returned value and what
/// the call alone consumed.
pub fn deploy_and_call<T>(
    wasm: &[u8],
    func: &str,
    setup: Setup,
    args: impl Fn(&Host, Val) -> Result<Vec<Val>, HostError>,
    read: impl FnOnce(&Host, Val, Cost) -> Result<T, HostError>,
) -> Result<T, HostError> {
    let (host, contract) = deploy(wasm, setup)?;
    let (out, cost) = call(&host, contract, func, args)?;
    read(&host, out, cost)
}
