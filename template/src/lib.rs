//! Agent allowance. See SPEC.md.

use tari_template_lib::prelude::*;

#[template]
mod allowance {
    use super::*;

    pub struct Allowance {
        owner: RistrettoPublicKeyBytes,
        agent: RistrettoPublicKeyBytes,
        cap: Amount,
        window_epochs: u64,
        // The window `spent` belongs to; a different current window means nothing is spent yet.
        window: u64,
        spent: Amount,
        funds: Vault,
        revoked: bool,
    }

    impl Allowance {
        pub fn new(agent: RistrettoPublicKeyBytes, cap: Amount, window_epochs: u64, funds: Bucket) -> Component<Self> {
            let owner = CallerContext::transaction_signer_public_key();
            assert!(funds.amount().is_positive(), "amount must be positive");
            assert!(cap.is_positive(), "cap must be positive");
            assert!(window_epochs > 0, "window must be at least one epoch");
            assert!(agent != owner, "agent can't be the owner");
            let window = Consensus::current_epoch() / window_epochs;
            Component::new(Self {
                owner,
                agent,
                cap,
                window_epochs,
                window,
                spent: Amount::zero(),
                funds: Vault::from_bucket(funds),
                revoked: false,
            })
            .with_access_rules(AccessRules::new().default(rule!(allow_all)))
            .with_owner_rule(OwnerRule::None)
            .create()
        }

        pub fn spend(&mut self, amount: Amount) -> Bucket {
            assert!(CallerContext::transaction_signer_public_key() == self.agent, "only the agent");
            assert!(!self.revoked, "allowance is revoked");
            assert!(amount.is_positive(), "amount must be positive");
            let spent = self.spent_now() + amount;
            assert!(spent <= self.cap, "over the cap for this window");
            assert!(amount <= self.funds.balance(), "not enough funds");
            self.window = Consensus::current_epoch() / self.window_epochs;
            self.spent = spent;
            self.funds.withdraw(amount)
        }

        pub fn top_up(&mut self, funds: Bucket) {
            self.assert_owner();
            assert!(funds.amount().is_positive(), "amount must be positive");
            self.funds.deposit(funds);
        }

        pub fn set_agent(&mut self, agent: RistrettoPublicKeyBytes) {
            self.assert_owner();
            assert!(agent != self.owner, "agent can't be the owner");
            self.agent = agent;
        }

        pub fn set_cap(&mut self, cap: Amount) {
            self.assert_owner();
            assert!(cap.is_positive(), "cap must be positive");
            self.cap = cap;
        }

        pub fn revoke(&mut self) {
            panic!("not implemented")
        }

        pub fn withdraw(&mut self, _amount: Amount) -> Bucket {
            panic!("not implemented")
        }

        pub fn status(&self) -> (RistrettoPublicKeyBytes, RistrettoPublicKeyBytes, Amount, u64, Amount, Amount, bool) {
            (self.owner, self.agent, self.cap, self.window_epochs, self.spent_now(), self.funds.balance(), self.revoked)
        }

        fn assert_owner(&self) {
            assert!(CallerContext::transaction_signer_public_key() == self.owner, "only the owner");
        }

        fn spent_now(&self) -> Amount {
            if Consensus::current_epoch() / self.window_epochs == self.window {
                self.spent
            } else {
                Amount::zero()
            }
        }
    }
}
