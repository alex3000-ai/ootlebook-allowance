//! The agent allowance oracle: each test is one unit of SPEC.md, run in-process by the Ootle engine.

use ootle_byte_type::ToByteType;
use tari_crypto::{
    keys::PublicKey as _,
    ristretto::{RistrettoPublicKey, RistrettoSecretKey},
};
use tari_engine_types::virtual_substate::{VirtualSubstate, VirtualSubstateId};
use tari_ootle_transaction::{args, builder::named_args::NamedArg, Transaction};
use tari_template_lib_types::{constants::TARI_TOKEN, crypto::RistrettoPublicKeyBytes, Amount, ComponentAddress, NonFungibleAddress};
use tari_template_test_tooling::TemplateTest;

const FUNDS: u64 = 1_000;
const CAP: u64 = 300;
const WINDOW: u64 = 10;

#[derive(Clone, Copy, Debug)]
enum Role {
    Owner,
    Agent,
    NewAgent,
    Stranger,
}
use Role::*;

struct Party {
    account: ComponentAddress,
    proof: NonFungibleAddress,
    key: RistrettoSecretKey,
}

struct Wallet {
    test: TemplateTest,
    parties: [Party; 4],
    allowance: Option<ComponentAddress>,
}

type Status = (RistrettoPublicKeyBytes, RistrettoPublicKeyBytes, Amount, u64, Amount, Amount, bool);

impl Wallet {
    /// Four funded accounts and no allowance yet.
    fn parties() -> Self {
        let mut test = TemplateTest::new(env!("CARGO_MANIFEST_DIR"), ["../template"]);
        let parties = [(); 4].map(|_| {
            let (account, proof, key) = test.create_funded_account();
            Party { account, proof, key }
        });
        Self { test, parties, allowance: None }
    }

    /// An allowance of FUNDS for the agent, CAP per WINDOW epochs, created at epoch 0.
    fn open() -> Self {
        let mut w = Self::parties();
        let tx = w.create_tx(w.pk(Agent), CAP, WINDOW, FUNDS);
        w.ok(Owner, tx);
        w.allowance = w.test.return_values().iter().find_map(|v| v.component_addresses().first().copied());
        w
    }

    fn party(&self, r: Role) -> &Party {
        &self.parties[r as usize]
    }

    fn pk(&self, r: Role) -> RistrettoPublicKeyBytes {
        RistrettoPublicKey::from_secret_key(&self.party(r).key).to_byte_type()
    }

    fn allowance(&self) -> ComponentAddress {
        self.allowance.expect("allowance created")
    }

    fn create_tx(&self, agent: RistrettoPublicKeyBytes, cap: u64, window: u64, funds: u64) -> Transaction {
        let owner = self.party(Owner);
        let template = self.test.get_template_address("Allowance");
        self.test
            .transaction()
            .call_method(owner.account, "withdraw", args![TARI_TOKEN, Amount::from(funds)])
            .put_last_instruction_output_on_workspace("funds")
            .call_function(template, "new", args![agent, Amount::from(cap), window, Workspace("funds")])
            .finish()
            .seal(&owner.key)
    }

    fn create_fails(&mut self, agent: RistrettoPublicKeyBytes, cap: u64, window: u64, funds: u64, expected: &str) {
        let tx = self.create_tx(agent, cap, window, funds);
        self.fails(Owner, tx, expected);
    }

    /// `r` calls `method`, which returns a bucket, and deposits it in its account.
    fn take_tx(&self, r: Role, method: &str, amount: u64) -> Transaction {
        let p = self.party(r);
        self.test
            .transaction()
            .call_method(self.allowance(), method, args![Amount::from(amount)])
            .put_last_instruction_output_on_workspace("out")
            .call_method(p.account, "deposit", args![Workspace("out")])
            .finish()
            .seal(&p.key)
    }

    fn spend(&mut self, r: Role, amount: u64) {
        let tx = self.take_tx(r, "spend", amount);
        self.ok(r, tx);
    }

    fn spend_fails(&mut self, r: Role, amount: u64, expected: &str) {
        let tx = self.take_tx(r, "spend", amount);
        self.fails(r, tx, expected);
    }

    fn withdraw(&mut self, r: Role, amount: u64) {
        let tx = self.take_tx(r, "withdraw", amount);
        self.ok(r, tx);
    }

    fn withdraw_fails(&mut self, r: Role, amount: u64, expected: &str) {
        let tx = self.take_tx(r, "withdraw", amount);
        self.fails(r, tx, expected);
    }

    fn top_up_tx(&self, r: Role, amount: u64) -> Transaction {
        let p = self.party(r);
        self.test
            .transaction()
            .call_method(p.account, "withdraw", args![TARI_TOKEN, Amount::from(amount)])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(self.allowance(), "top_up", args![Workspace("funds")])
            .finish()
            .seal(&p.key)
    }

    fn call(&mut self, r: Role, method: &str, args: Vec<NamedArg>) {
        let tx = self.test.transaction().call_method(self.allowance(), method, args).finish().seal(&self.party(r).key);
        self.ok(r, tx);
    }

    fn call_fails(&mut self, r: Role, method: &str, args: Vec<NamedArg>, expected: &str) {
        let tx = self.test.transaction().call_method(self.allowance(), method, args).finish().seal(&self.party(r).key);
        self.fails(r, tx, expected);
    }

    fn ok(&mut self, r: Role, tx: Transaction) {
        let proof = self.party(r).proof.clone();
        self.test.execute_expect_success(tx, vec![proof]);
    }

    fn fails(&mut self, r: Role, tx: Transaction, expected: &str) {
        let proof = self.party(r).proof.clone();
        let reason = self.test.execute_expect_failure(tx, vec![proof]).to_string();
        assert!(reason.contains(expected), "{r:?}: expected rejection containing {expected:?}, got: {reason}");
    }

    fn status(&mut self) -> Status {
        self.test.call_method(self.allowance(), "status", args![], vec![])
    }

    /// (spent in the current window, balance, revoked).
    fn state(&mut self) -> (Amount, Amount, bool) {
        let s = self.status();
        (s.4, s.5, s.6)
    }

    fn balance(&mut self, r: Role) -> Amount {
        let account = self.party(r).account;
        self.test.call_method(account, "balance", args![TARI_TOKEN], vec![])
    }

    fn set_epoch(&mut self, epoch: u64) {
        self.test.set_virtual_substate(VirtualSubstateId::CurrentEpoch, VirtualSubstate::CurrentEpoch(epoch));
    }
}

fn state(spent: u64, balance: u64, revoked: bool) -> (Amount, Amount, bool) {
    (spent.into(), balance.into(), revoked)
}

// ---- new ----

#[test]
fn new_records_the_terms_and_holds_the_funds() {
    let mut w = Wallet::open();
    let s = w.status();
    assert_eq!(s, (w.pk(Owner), w.pk(Agent), CAP.into(), WINDOW, Amount::zero(), FUNDS.into(), false));
}

#[test]
fn new_rejects_an_empty_bucket() {
    let mut w = Wallet::parties();
    w.create_fails(w.pk(Agent), CAP, WINDOW, 0, "amount must be positive");
}

#[test]
fn new_rejects_a_zero_cap() {
    let mut w = Wallet::parties();
    w.create_fails(w.pk(Agent), 0, WINDOW, FUNDS, "cap must be positive");
}

#[test]
fn new_rejects_a_zero_window() {
    let mut w = Wallet::parties();
    w.create_fails(w.pk(Agent), CAP, 0, FUNDS, "window must be at least one epoch");
}

#[test]
fn new_rejects_the_owner_as_agent() {
    let mut w = Wallet::parties();
    w.create_fails(w.pk(Owner), CAP, WINDOW, FUNDS, "agent can't be the owner");
}

#[test]
fn new_checks_in_the_order_given() {
    let mut w = Wallet::parties();
    w.create_fails(w.pk(Owner), 0, 0, 0, "amount must be positive");
    w.create_fails(w.pk(Owner), 0, 0, FUNDS, "cap must be positive");
    w.create_fails(w.pk(Owner), CAP, 0, FUNDS, "window must be at least one epoch");
}

// ---- spend ----

#[test]
fn spend_pays_the_agent() {
    let mut w = Wallet::open();
    let before = w.balance(Agent);
    w.spend(Agent, 120);
    assert_eq!(w.balance(Agent), before + Amount::from(120u64));
    assert_eq!(w.state(), state(120, FUNDS - 120, false));
}

#[test]
fn spend_up_to_exactly_the_cap() {
    let mut w = Wallet::open();
    w.spend(Agent, 100);
    w.spend(Agent, CAP - 100);
    assert_eq!(w.state(), state(CAP, FUNDS - CAP, false));
}

#[test]
fn spend_rejects_going_over_the_cap() {
    let mut w = Wallet::open();
    w.spend(Agent, 200);
    w.spend_fails(Agent, CAP - 199, "over the cap for this window");
    assert_eq!(w.state(), state(200, FUNDS - 200, false));
}

#[test]
fn spend_rejects_one_spend_above_the_cap() {
    let mut w = Wallet::open();
    w.spend_fails(Agent, CAP + 1, "over the cap for this window");
}

#[test]
fn spend_resets_in_the_next_window() {
    let mut w = Wallet::open();
    w.spend(Agent, CAP);
    w.set_epoch(WINDOW);
    assert_eq!(w.state(), state(0, FUNDS - CAP, false));
    w.spend(Agent, CAP);
    assert_eq!(w.state(), state(CAP, FUNDS - 2 * CAP, false));
}

#[test]
fn spend_windows_are_fixed_blocks_of_epochs() {
    let mut w = Wallet::open();
    w.set_epoch(WINDOW - 1);
    w.spend(Agent, CAP);
    w.spend_fails(Agent, 1, "over the cap for this window");
    w.set_epoch(WINDOW);
    w.spend(Agent, CAP);
    w.set_epoch(2 * WINDOW - 1);
    w.spend_fails(Agent, 1, "over the cap for this window");
}

#[test]
fn spend_is_only_for_the_agent() {
    let mut w = Wallet::open();
    for r in [Owner, NewAgent, Stranger] {
        w.spend_fails(r, 1, "only the agent");
    }
}

#[test]
fn spend_rejects_zero() {
    let mut w = Wallet::open();
    w.spend_fails(Agent, 0, "amount must be positive");
}

#[test]
fn spend_rejects_more_than_the_balance() {
    let mut w = Wallet::open();
    w.withdraw(Owner, FUNDS - 50);
    w.spend_fails(Agent, 51, "not enough funds");
    w.spend(Agent, 50);
    assert_eq!(w.state(), state(50, 0, false));
}

#[test]
fn spend_checks_the_cap_before_the_balance() {
    let mut w = Wallet::open();
    w.withdraw(Owner, FUNDS - 50);
    w.spend_fails(Agent, CAP + 1, "over the cap for this window");
}

#[test]
fn spend_checks_the_signer_before_revocation() {
    let mut w = Wallet::open();
    w.call(Owner, "revoke", args![]);
    w.spend_fails(Stranger, 1, "only the agent");
}

#[test]
fn spend_checks_revocation_before_the_amount() {
    let mut w = Wallet::open();
    w.call(Owner, "revoke", args![]);
    w.spend_fails(Agent, 0, "allowance is revoked");
}

// ---- top_up ----

#[test]
fn top_up_adds_funds() {
    let mut w = Wallet::open();
    let tx = w.top_up_tx(Owner, 500);
    w.ok(Owner, tx);
    assert_eq!(w.state(), state(0, FUNDS + 500, false));
}

#[test]
fn top_up_is_only_for_the_owner() {
    let mut w = Wallet::open();
    for r in [Agent, Stranger] {
        let tx = w.top_up_tx(r, 10);
        w.fails(r, tx, "only the owner");
    }
}

#[test]
fn top_up_rejects_an_empty_bucket() {
    let mut w = Wallet::open();
    let tx = w.top_up_tx(Owner, 0);
    w.fails(Owner, tx, "amount must be positive");
}

#[test]
fn top_up_works_after_revoking() {
    let mut w = Wallet::open();
    w.call(Owner, "revoke", args![]);
    let tx = w.top_up_tx(Owner, 500);
    w.ok(Owner, tx);
    assert_eq!(w.state(), state(0, FUNDS + 500, true));
}

// ---- set_agent ----

#[test]
fn set_agent_hands_spending_to_the_new_key() {
    let mut w = Wallet::open();
    let new = w.pk(NewAgent);
    w.call(Owner, "set_agent", args![new]);
    w.spend_fails(Agent, 1, "only the agent");
    w.spend(NewAgent, 100);
    assert_eq!(w.status().1, new);
}

#[test]
fn set_agent_keeps_the_window_spending() {
    let mut w = Wallet::open();
    w.spend(Agent, 250);
    let new = w.pk(NewAgent);
    w.call(Owner, "set_agent", args![new]);
    w.spend_fails(NewAgent, 51, "over the cap for this window");
    w.spend(NewAgent, 50);
}

#[test]
fn set_agent_is_only_for_the_owner() {
    let mut w = Wallet::open();
    let new = w.pk(NewAgent);
    for r in [Agent, Stranger] {
        w.call_fails(r, "set_agent", args![new], "only the owner");
    }
}

#[test]
fn set_agent_rejects_the_owner() {
    let mut w = Wallet::open();
    let owner = w.pk(Owner);
    w.call_fails(Owner, "set_agent", args![owner], "agent can't be the owner");
}

// ---- set_cap ----

#[test]
fn set_cap_raises_the_cap_at_once() {
    let mut w = Wallet::open();
    w.spend(Agent, CAP);
    w.call(Owner, "set_cap", args![Amount::from(CAP + 100)]);
    w.spend(Agent, 100);
    assert_eq!(w.status().2, Amount::from(CAP + 100));
}

#[test]
fn set_cap_lowers_the_cap_below_what_was_spent() {
    let mut w = Wallet::open();
    w.spend(Agent, 200);
    w.call(Owner, "set_cap", args![Amount::from(100u64)]);
    w.spend_fails(Agent, 1, "over the cap for this window");
    w.set_epoch(WINDOW);
    w.spend(Agent, 100);
}

#[test]
fn set_cap_is_only_for_the_owner() {
    let mut w = Wallet::open();
    for r in [Agent, Stranger] {
        w.call_fails(r, "set_cap", args![Amount::from(1u64)], "only the owner");
    }
}

#[test]
fn set_cap_rejects_zero() {
    let mut w = Wallet::open();
    w.call_fails(Owner, "set_cap", args![Amount::zero()], "cap must be positive");
}

// ---- revoke ----

#[test]
fn revoke_stops_spending() {
    let mut w = Wallet::open();
    w.call(Owner, "revoke", args![]);
    w.spend_fails(Agent, 1, "allowance is revoked");
    assert_eq!(w.state(), state(0, FUNDS, true));
}

#[test]
fn revoke_is_for_good() {
    let mut w = Wallet::open();
    w.call(Owner, "revoke", args![]);
    let new = w.pk(NewAgent);
    w.call(Owner, "set_agent", args![new]);
    w.call(Owner, "set_cap", args![Amount::from(CAP)]);
    w.set_epoch(5 * WINDOW);
    w.spend_fails(NewAgent, 1, "allowance is revoked");
}

#[test]
fn revoke_is_only_for_the_owner() {
    let mut w = Wallet::open();
    for r in [Agent, Stranger] {
        w.call_fails(r, "revoke", args![], "only the owner");
    }
}

#[test]
fn revoke_fails_the_second_time() {
    let mut w = Wallet::open();
    w.call(Owner, "revoke", args![]);
    w.call_fails(Owner, "revoke", args![], "allowance is revoked");
}

// ---- withdraw ----

#[test]
fn withdraw_returns_funds_to_the_owner() {
    let mut w = Wallet::open();
    let before = w.balance(Owner);
    w.withdraw(Owner, 400);
    assert_eq!(w.balance(Owner), before + Amount::from(400u64));
    assert_eq!(w.state(), state(0, FUNDS - 400, false));
}

#[test]
fn withdraw_works_after_revoking() {
    let mut w = Wallet::open();
    w.spend(Agent, 100);
    w.call(Owner, "revoke", args![]);
    w.withdraw(Owner, FUNDS - 100);
    assert_eq!(w.state(), state(100, 0, true));
}

#[test]
fn withdraw_is_only_for_the_owner() {
    let mut w = Wallet::open();
    for r in [Agent, Stranger] {
        w.withdraw_fails(r, 1, "only the owner");
    }
}

#[test]
fn withdraw_rejects_zero() {
    let mut w = Wallet::open();
    w.withdraw_fails(Owner, 0, "amount must be positive");
}

#[test]
fn withdraw_rejects_more_than_the_balance() {
    let mut w = Wallet::open();
    w.withdraw_fails(Owner, FUNDS + 1, "not enough funds");
}
