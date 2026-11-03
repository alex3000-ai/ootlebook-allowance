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

        pub fn spend(&mut self, _amount: Amount) -> Bucket {
            panic!("not implemented")
        }

        pub fn top_up(&mut self, _funds: Bucket) {
            panic!("not implemented")
        }

        pub fn set_agent(&mut self, _agent: RistrettoPublicKeyBytes) {
            panic!("not implemented")
        }

        pub fn set_cap(&mut self, _cap: Amount) {
            panic!("not implemented")
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

        fn spent_now(&self) -> Amount {
            if Consensus::current_epoch() / self.window_epochs == self.window {
                self.spent
            } else {
                Amount::zero()
            }
        }
    }
}
