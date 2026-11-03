//! Agent allowance. See SPEC.md; this stub fails every test.

use tari_template_lib::prelude::*;

#[template]
mod allowance {
    use super::*;

    pub struct Allowance {}

    impl Allowance {
        pub fn new(_agent: RistrettoPublicKeyBytes, _cap: Amount, _window_epochs: u64, _funds: Bucket) -> Component<Self> {
            panic!("not implemented")
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
            panic!("not implemented")
        }
    }
}
