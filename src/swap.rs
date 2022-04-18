#![no_std]

const ONE_DAY: u64 = 86400;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::contract]
pub trait DevNetFaucet {
    #[init]
    fn init(&self) {}

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueToken)]
    fn issue_token(
        &self,
        #[payment] issue_cost: BigUint,
        token_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
    ) {
        self.send()
            .esdt_system_sc_proxy()
            .issue_fungible(
                issue_cost,
                &token_name,
                &token_ticker,
                &(BigUint::from(100000000u32) * BigUint::from(10u32).pow(18)),
                FungibleTokenProperties {
                    can_burn: false,
                    can_mint: false,
                    num_decimals: 18usize,
                    can_freeze: false,
                    can_wipe: false,
                    can_pause: false,
                    can_change_owner: true,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(self.callbacks().issue_callback())
            .call_and_exit();
    }

    #[callback]
    fn issue_callback(&self, #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>) {
        match result {
            ManagedAsyncCallResult::Ok(_) => {}
            ManagedAsyncCallResult::Err(_) => {
                let caller = self.blockchain().get_owner_address();
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    self.send()
                        .direct(&caller, &token_id, 0, &returned_tokens, &[]);
                }
            }
        }
    }

    #[only_owner]
    #[endpoint(defundContractEgld)]
    fn defund_contract_egld(&self) {
        let balance = self
            .blockchain()
            .get_sc_balance(&TokenIdentifier::egld(), 0);
        require!(balance != 0u32, "contract has no egld");
        let caller = self.blockchain().get_caller();
        self.send()
            .direct(&caller, &TokenIdentifier::egld(), 0, &balance, &[]);
    }

    #[only_owner]
    #[endpoint(defundContractTokens)]
    fn defund_contract_tokens(&self, token_id: TokenIdentifier) {
        let balance = self.blockchain().get_sc_balance(&token_id, 0u64);
        let caller = self.blockchain().get_caller();
        self.send().direct(&caller, &token_id, 0, &balance, &[]);
    }

    #[endpoint(claimToken)]
    fn claim_token(&self, token_id: TokenIdentifier) {
        let caller = self.blockchain().get_caller();

        let current_time = self.blockchain().get_block_timestamp();
        let last_claim = self.last_claim(&caller, &token_id).get();
        require!(
            current_time - last_claim > ONE_DAY,
            "You must wait one day before claiming a token again"
        );

        let balance = self.blockchain().get_sc_balance(&token_id, 0u64);
        let amount = BigUint::from(1000u32) * BigUint::from(10u32).pow(18);
        require!(amount <= balance, "Not enough tokens in the SC to claim");

        self.last_claim(&caller, &token_id).set(current_time);

        self.send().direct(&caller, &token_id, 0, &balance, &[]);
    }

    #[view(getLastClaim)]
    #[storage_mapper("last_claim")]
    fn last_claim(
        &self,
        address: &ManagedAddress,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<u64>;
}
