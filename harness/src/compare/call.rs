use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol, Val,
    testutils::{generate_account_id, generate_bytes_array},
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
pub struct Cost {
    pub cpu: u64,
    pub mem: u64,
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
    let host = Host::test_host_with_recording_footprint();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    let contract =
        host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;
    if setup.cached {
        host.ensure_module_cache_contains_host_storage_contracts()?;
    }
    if setup.authed {
        host.switch_to_recording_auth(false)?;
    }

    let args = args(&host, contract.to_val())?;
    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(&args)?;

    let budget = host.budget_cloned();
    let base_cpu = budget.get_cpu_insns_consumed()?;
    let base_mem = budget.get_mem_bytes_consumed()?;

    let out = host.call(contract, sym, argv)?;

    let cost = Cost {
        cpu: budget.get_cpu_insns_consumed()? - base_cpu,
        mem: budget.get_mem_bytes_consumed()? - base_mem,
    };
    read(&host, out, cost)
}
