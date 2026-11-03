# Agent allowance: the spec

An **owner** funds an allowance and names an **agent** key that may spend up to a cap in each window of epochs.
The owner can top it up, change the agent or the cap, take funds back, or revoke the agent for good. No admin
beyond the owner's own funds, nothing else can be changed after creation.

This file and `tests/` are frozen. An implementation is done when `./score.sh` reports every test passing, with
`tests/` unchanged.

## Template

Template name `Allowance`, one component per allowance, in `template/src/lib.rs`, using only `tari_template_lib`.

A party is identified by the **transaction signer's public key** (`CallerContext::transaction_signer_public_key()`).
The owner is whoever signs the `new` transaction. Epochs are `Consensus::current_epoch()`.

The component allows every method to anyone (`AccessRules::new().default(rule!(allow_all))`) and has no owner rule
(`OwnerRule::None`); the methods check the signer themselves.

## Windows

Windows are fixed blocks of `window_epochs` epochs: the current window is `current_epoch / window_epochs`
(integer division), so with a window of 10, epochs 0-9 are one window and 10-19 the next. The amount spent counts
from zero again in each new window. Changing the agent or the cap does not reset it.

## Methods

| Method | Who | Effect |
|---|---|---|
| `new(agent: RistrettoPublicKeyBytes, cap: Amount, window_epochs: u64, funds: Bucket) -> Component<Self>` | anyone (becomes the owner) | Holds `funds` (any fungible resource). |
| `spend(amount: Amount) -> Bucket` | agent | Returns `amount` and adds it to this window's spending. |
| `top_up(funds: Bucket)` | owner | Adds `funds`. Allowed after revoking. |
| `set_agent(agent: RistrettoPublicKeyBytes)` | owner | Replaces the agent key at once. |
| `set_cap(cap: Amount)` | owner | Replaces the cap at once, for the current window too. |
| `revoke()` | owner | Stops all spending for good. |
| `withdraw(amount: Amount) -> Bucket` | owner | Returns `amount`, revoked or not. |
| `status() -> (RistrettoPublicKeyBytes, RistrettoPublicKeyBytes, Amount, u64, Amount, Amount, bool)` | anyone | (owner, agent, cap, window epochs, spent in the current window, balance, revoked). |

## Rejections

A rejected call panics with a message **containing** the text below. Checks run in the order listed for each method
(**who is signing, then state, then arguments**), and the first failure wins.

| Method | Order of checks |
|---|---|
| `new` | `amount must be positive` (empty bucket) → `cap must be positive` → `window must be at least one epoch` → `agent can't be the owner` |
| `spend` | `only the agent` → `allowance is revoked` → `amount must be positive` → `over the cap for this window` (spent this window + amount > cap) → `not enough funds` |
| `top_up` | `only the owner` → `amount must be positive` |
| `set_agent` | `only the owner` → `agent can't be the owner` |
| `set_cap` | `only the owner` → `cap must be positive` |
| `revoke` | `only the owner` → `allowance is revoked` (already) |
| `withdraw` | `only the owner` → `amount must be positive` → `not enough funds` |

## Invariant

Balance == everything put in − everything spent − everything withdrawn. Spent in a window never exceeds the cap
as it stood at each spend. A rejected call changes nothing.

## Units of work

Each test in `tests/tests/allowance.rs` is one unit, prefixed by what it covers (`new_`, `spend_`, `top_up_`,
`set_agent_`, `set_cap_`, `revoke_`, `withdraw_`). Claim a prefix, make its tests pass without breaking others.
