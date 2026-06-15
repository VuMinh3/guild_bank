#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Map, String, Symbol, Vec};

/// Namespaced storage keys used by the guild bank contract. Each
/// guild identified by its `Symbol` gets its own slot for every
/// piece of state (founder, balance, members, officers, ...).
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// `Symbol -> Address` — the account that founded the guild.
    Founder(Symbol),
    /// `Symbol -> i128` — pooled treasury balance.
    Balance(Symbol),
    /// `Symbol -> u32` — officer signatures needed to release funds.
    RequiredApprovals(Symbol),
    /// `Symbol -> Map<Address, bool>` — roster of guild members.
    Members(Symbol),
    /// `Symbol -> Map<Address, bool>` — roster of guild officers.
    Officers(Symbol),
    /// `Symbol -> Map<Address, i128>` — lifetime contribution per
    /// member.
    Contributions(Symbol),
    /// `Symbol -> u64` — monotonic counter for withdrawal requests.
    RequestCounter(Symbol),
    /// `Symbol, u64 -> WithdrawalRequest` — pending or executed
    /// withdrawal proposal.
    Withdrawal(Symbol, u64),
}

/// On-chain proposal to release a portion of the guild treasury.
/// Multiple officers must approve before `executed` flips to true.
#[contracttype]
#[derive(Clone)]
pub struct WithdrawalRequest {
    /// Officer that submitted the request.
    pub requester: Address,
    /// Amount that would leave the treasury on full approval.
    pub amount: i128,
    /// Free-form reason the requester supplied.
    pub reason: String,
    /// Number of officer signatures collected so far.
    pub approvals: u32,
    /// Ordered list of officer addresses that have signed.
    pub approvers: Vec<Address>,
    /// True once the request has been executed (funds released).
    pub executed: bool,
}

/// `guild_bank` — multi-officer shared treasury for gaming guilds
/// (clans). Members pool funds, the founder promotes officers, and
/// withdrawals require a configurable number of officer approvals
/// before any funds leave the contract.
#[contract]
pub struct GuildBank;

#[contractimpl]
impl GuildBank {
    /// Open a new guild with `founder` as the first officer and
    /// member. `required_approvals` is the number of officer
    /// signatures a withdrawal request must collect before funds
    /// are released; it must be at least one.
    pub fn create_guild(
        env: Env,
        founder: Address,
        guild_id: Symbol,
        required_approvals: u32,
    ) {
        founder.require_auth();

        if env
            .storage()
            .instance()
            .has(&DataKey::Founder(guild_id.clone()))
        {
            panic!("guild already exists");
        }
        if required_approvals == 0 {
            panic!("required_approvals must be greater than zero");
        }

        let mut members: Map<Address, bool> = Map::new(&env);
        members.set(founder.clone(), true);

        let mut officers: Map<Address, bool> = Map::new(&env);
        officers.set(founder.clone(), true);

        env.storage()
            .instance()
            .set(&DataKey::Founder(guild_id.clone()), &founder);
        env.storage()
            .instance()
            .set(&DataKey::Balance(guild_id.clone()), &0i128);
        env.storage().instance().set(
            &DataKey::RequiredApprovals(guild_id.clone()),
            &required_approvals,
        );
        env.storage()
            .instance()
            .set(&DataKey::Members(guild_id.clone()), &members);
        env.storage()
            .instance()
            .set(&DataKey::Officers(guild_id.clone()), &officers);
        env.storage().instance().set(
            &DataKey::Contributions(guild_id.clone()),
            &Map::<Address, i128>::new(&env),
        );
        env.storage()
            .instance()
            .set(&DataKey::RequestCounter(guild_id.clone()), &0u64);
    }

    /// Have `member` join an existing guild. Joining lets the
    /// address deposit into the treasury and become eligible for
    /// future officer promotion. The caller must authorize the
    /// join themselves.
    pub fn join_guild(env: Env, member: Address, guild_id: Symbol) {
        member.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Founder(guild_id.clone()))
        {
            panic!("guild does not exist");
        }

        let mut members: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&DataKey::Members(guild_id.clone()))
            .unwrap();

        if members.get(member.clone()).unwrap_or(false) {
            panic!("already a member");
        }

        members.set(member, true);
        env.storage()
            .instance()
            .set(&DataKey::Members(guild_id.clone()), &members);
    }

    /// Deposit `amount` into the guild treasury. The caller must
    /// be a current member; their lifetime contribution and the
    /// guild's pooled balance are both updated in a single
    /// transaction.
    pub fn deposit(env: Env, member: Address, guild_id: Symbol, amount: i128) {
        member.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }
        if !env
            .storage()
            .instance()
            .has(&DataKey::Founder(guild_id.clone()))
        {
            panic!("guild does not exist");
        }

        let members: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&DataKey::Members(guild_id.clone()))
            .unwrap();
        if !members.get(member.clone()).unwrap_or(false) {
            panic!("not a member of this guild");
        }

        let balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Balance(guild_id.clone()))
            .unwrap_or(0);
        let new_balance = balance
            .checked_add(amount)
            .expect("guild balance overflow");

        let mut contributions: Map<Address, i128> = env
            .storage()
            .instance()
            .get(&DataKey::Contributions(guild_id.clone()))
            .unwrap();
        let prev = contributions.get(member.clone()).unwrap_or(0);
        contributions.set(
            member,
            prev.checked_add(amount).expect("contribution overflow"),
        );

        env.storage()
            .instance()
            .set(&DataKey::Balance(guild_id.clone()), &new_balance);
        env.storage()
            .instance()
            .set(&DataKey::Contributions(guild_id.clone()), &contributions);
    }

    /// Promote an existing member to officer. Only the founder of
    /// the guild may call this function. Officers gain the right
    /// to submit and sign withdrawal requests.
    pub fn promote_officer(
        env: Env,
        founder: Address,
        guild_id: Symbol,
        new_officer: Address,
    ) {
        founder.require_auth();

        let stored_founder: Address = env
            .storage()
            .instance()
            .get(&DataKey::Founder(guild_id.clone()))
            .expect("guild not found");
        if stored_founder != founder {
            panic!("only the founder can promote officers");
        }

        let mut officers: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&DataKey::Officers(guild_id.clone()))
            .unwrap();
        officers.set(new_officer, true);
        env.storage()
            .instance()
            .set(&DataKey::Officers(guild_id.clone()), &officers);
    }

    /// Submit a new withdrawal request. Only officers can call
    /// this; the call returns the assigned request id, which is
    /// later passed to `approve_withdraw`. The request starts
    /// unapproved and unfunded.
    pub fn request_withdraw(
        env: Env,
        requester: Address,
        guild_id: Symbol,
        amount: i128,
        reason: String,
    ) -> u64 {
        requester.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        let officers: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&DataKey::Officers(guild_id.clone()))
            .expect("guild not found");
        if !officers.get(requester.clone()).unwrap_or(false) {
            panic!("only officers can request withdrawals");
        }

        let balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Balance(guild_id.clone()))
            .unwrap_or(0);
        if amount > balance {
            panic!("insufficient guild balance");
        }

        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RequestCounter(guild_id.clone()))
            .unwrap_or(0);
        counter = counter.checked_add(1).expect("request counter overflow");

        let request = WithdrawalRequest {
            requester,
            amount,
            reason,
            approvals: 0,
            approvers: Vec::new(&env),
            executed: false,
        };

        env.storage()
            .instance()
            .set(&DataKey::RequestCounter(guild_id.clone()), &counter);
        env.storage()
            .instance()
            .set(&DataKey::Withdrawal(guild_id.clone(), counter), &request);

        counter
    }

    /// Cast `officer`'s approval on a pending withdrawal request.
    /// When the number of collected approvals reaches the
    /// guild's `required_approvals`, the requested amount is
    /// deducted from the treasury and the request is marked
    /// executed. Each officer may approve a given request at most
    /// once.
    pub fn approve_withdraw(
        env: Env,
        officer: Address,
        guild_id: Symbol,
        request_id: u64,
    ) {
        officer.require_auth();

        let officers: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&DataKey::Officers(guild_id.clone()))
            .expect("guild not found");
        if !officers.get(officer.clone()).unwrap_or(false) {
            panic!("not an officer");
        }

        let mut request: WithdrawalRequest = env
            .storage()
            .instance()
            .get(&DataKey::Withdrawal(guild_id.clone(), request_id))
            .expect("withdrawal request not found");

        if request.executed {
            panic!("request already executed");
        }

        for existing in request.approvers.iter() {
            if existing == officer {
                panic!("officer already approved this request");
            }
        }

        request.approvers.push_back(officer);
        request.approvals = request
            .approvals
            .checked_add(1)
            .expect("approval count overflow");

        let required: u32 = env
            .storage()
            .instance()
            .get(&DataKey::RequiredApprovals(guild_id.clone()))
            .unwrap_or(1);

        if request.approvals >= required {
            let balance: i128 = env
                .storage()
                .instance()
                .get(&DataKey::Balance(guild_id.clone()))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::Balance(guild_id.clone()),
                &balance
                    .checked_sub(request.amount)
                    .expect("balance underflow on execute"),
            );
            request.executed = true;
        }

        env.storage()
            .instance()
            .set(&DataKey::Withdrawal(guild_id.clone(), request_id), &request);
    }

    /// Read the current pooled balance of the guild's treasury.
    /// Returns 0 if the guild has not been created.
    pub fn guild_balance(env: Env, guild_id: Symbol) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Balance(guild_id))
            .unwrap_or(0)
    }

    /// Read the total amount a single member has contributed to
    /// the guild over time. Returns 0 for non-members or unknown
    /// guilds.
    pub fn member_contribution(
        env: Env,
        guild_id: Symbol,
        member: Address,
    ) -> i128 {
        let contributions: Map<Address, i128> = env
            .storage()
            .instance()
            .get(&DataKey::Contributions(guild_id))
            .unwrap_or(Map::new(&env));
        contributions.get(member).unwrap_or(0)
    }

    /// Look up the founder address of a guild. Used by clients to
    /// verify officer promotions and as a cheap "does this guild
    /// exist?" check.
    pub fn guild_founder(env: Env, guild_id: Symbol) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Founder(guild_id))
            .expect("guild not found")
    }
}
