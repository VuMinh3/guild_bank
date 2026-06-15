# guild_bank

## Project Title
guild_bank

## Project Description
In multiplayer online games, guilds (clans) typically pool in-game currency to fund shared activities: raid consumables, base upgrades, tournament entry fees, wagers against rival guilds, and prize payouts to top contributors. In practice, that pooled money is held by a single treasurer's account, which is a single point of failure, a target for account theft, and an opaque ledger that other members cannot audit. `guild_bank` solves this by moving the shared treasury into a Soroban smart contract on Stellar: members deposit into a guild-owned pool, every contribution is permanently recorded, and no officer can drain the balance alone — withdrawals require a configurable number of officer approvals.

## Project Vision
The long-term vision for `guild_bank` is to become the default on-chain treasury for play-to-earn guilds, GameFi DAOs, and competitive esports organizations running on Stellar. The contract is intentionally minimal so it can be embedded inside larger game economies: a guild dApp, a tournament organizer, or an MMO could plug `guild_bank` in as a drop-in treasury module and trust that withdrawals are governed by a transparent, auditable multi-officer workflow. Over time the primitive composes with other Soroban contracts — NFT membership cards, off-chain game asset oracles, reputation scoring — to enable fully on-chain guild governance without trusting a human treasurer.

## Key Features
- **Guild creation with founder authority** — `create_guild` lets a founder open a new guild, becoming the first officer and member, and decides upfront how many officer signatures a withdrawal will need.
- **Open membership** — `join_guild` lets any address self-register as a member and become eligible to deposit into the shared pool.
- **Auditable deposits with per-member history** — `deposit` records both the new guild balance and an incrementing `member_contribution` counter, giving the guild a permanent on-chain record of who funded what.
- **Multi-officer approval workflow for withdrawals** — `request_withdraw` opens a proposal with a reason string; `approve_withdraw` collects officer signatures one at a time, and the funds are only released once the configured approval threshold is reached.
- **Founder-managed officer roster** — `promote_officer` lets the founder elevate trusted members to officer status, expanding the signer set over the lifetime of the guild.
- **Read-only views** — `guild_balance`, `member_contribution`, and `guild_founder` expose treasury and roster state to off-chain UIs without requiring any transaction.

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** gaming dApp — see `contracts/guild_bank/src/lib.rs` for the full guild_bank business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** CC2RSNEV4LBERFW22JBRVMAYH6VU7C7EN6NNOJUEFJZEGSVGO5GLGV33
- **Explorer template:** https://stellar.expert/explorer/testnet/tx/8152e1df52d9c951e44277486a22cea9e46348718baef59a419ada80477a6d19
- **Screenshot of deployed contract on Stellar Expert:**
![screenshot](https://ibb.co/xTZvJgK)


## Future Scope
- **On-chain withdrawal execution with a destination address** — extend the `WithdrawalRequest` struct with a `recipient: Address` and call Stellar's native asset transfer (or a wrapped game-token transfer) when the approval threshold is reached, so approved funds actually leave the contract.
- **Role rotation and demotion** — add `demote_officer` and a founder-transfer flow so guild leadership can hand off control without redeploying the contract.
- **Time-locked withdrawals and veto windows** — once a request reaches its approval threshold, hold it in a pending state for a configurable delay during which any officer can veto, protecting against rushed decisions.
- **Per-member contribution-based voting weight** — weight officer approvals by lifetime contribution so whales cannot simply stack yes-votes, aligning governance with skin-in-the-game.
- **Frontend dApp** — build a Freighter-connected web UI (HTML/JS) that lets a guild dashboard show live balance, contribution leaderboard, and pending withdrawal proposals with one-click approve/reject.
- **Integration with a wrapped game-token** — accept deposits and release withdrawals in a custom Stellar asset issued for the game, instead of an abstract i128 counter, by calling the token contract's `transfer` / `transfer_from` methods.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `guild_bank` (gaming)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
