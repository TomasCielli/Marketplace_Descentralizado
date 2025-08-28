#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod segundo_contrato {

    use primer_contrato::PrimerContratoRef;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct SegundoContrato {
        /// Stores a single `bool` value on the storage.
        marketplace: PrimerContratoRef,
    }

    impl SegundoContrato {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        // In `basic_contract_ref/lib.rs`

        #[ink(constructor)]
        pub fn new(other_contract_code_hash: Hash) -> Self {

            let marketplace = PrimerContratoRef::new()
                .code_hash(other_contract_code_hash)
                .endowment(0)
                .salt_bytes([0xDE, 0xAD, 0xBE, 0xEF])
                .instantiate();

            Self { marketplace }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

        #[ink(message)]
        pub fn nada(&self) -> u32{
            return self.marketplace.get_dimension_logica()
        }
    }
}
